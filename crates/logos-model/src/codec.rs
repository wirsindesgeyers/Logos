//! Codificação e decodificação estrita de envelopes `model/v1`.

use std::collections::BTreeMap;

use logos_air::{Grounding, SortDomain, SymbolKind, SymbolResult, Vocabulary};
use logos_canon::{Array, CanonError, Decoder, Value};
use logos_core::Exhausted;

use crate::data::ModelData;
use crate::error::ModelError;
use crate::limits::ModelLimits;
use crate::validate::validate_vocabulary_signatures;

/// Converte `ModelData` validado para o modelo canônico de dados CBOR (`canon/v1`).
pub(crate) fn model_data_to_value(data: &ModelData) -> Result<Value, CanonError> {
    // 0. Versão unsigned 1
    let version_val = Value::Unsigned(1);

    // 1. hV: hash do vocabulário em 32 bytes
    let hv_val = Value::Bytes(data.vocab.as_bytes().to_vec());

    // 2. domains: [domain u32...]
    let domain_items = data
        .domains
        .iter()
        .map(|&d| Value::Unsigned(u64::from(d)))
        .collect();
    let domains_val = Array::new(domain_items).map(Value::Array)?;

    // 3. consts: [[id, const_value]...]
    let mut const_items = Vec::new();
    const_items
        .try_reserve_exact(data.consts.len())
        .map_err(|_| CanonError::WrongType)?;
    for &(id, val) in &data.consts {
        let pair = Array::new(vec![
            Value::Unsigned(u64::from(id)),
            Value::Unsigned(u64::from(val)),
        ])
        .map(Value::Array)?;
        const_items.push(pair);
    }
    let consts_val = Array::new(const_items).map(Value::Array)?;

    // 4. funcs: [[id, bytes packed big-endian u32 outputs]...]
    let mut func_items = Vec::new();
    func_items
        .try_reserve_exact(data.funcs.len())
        .map_err(|_| CanonError::WrongType)?;
    for (id, outs) in &data.funcs {
        let byte_len = outs.len().checked_mul(4).ok_or(CanonError::WrongType)?;
        let mut packed_bytes = Vec::new();
        packed_bytes
            .try_reserve_exact(byte_len)
            .map_err(|_| CanonError::WrongType)?;
        for &out in outs {
            packed_bytes.extend_from_slice(&out.to_be_bytes());
        }
        let pair = Array::new(vec![
            Value::Unsigned(u64::from(*id)),
            Value::Bytes(packed_bytes),
        ])
        .map(Value::Array)?;
        func_items.push(pair);
    }
    let funcs_val = Array::new(func_items).map(Value::Array)?;

    // 5. preds: [[id, [[tuple u32...]...]]...]
    let mut pred_items = Vec::new();
    pred_items
        .try_reserve_exact(data.preds.len())
        .map_err(|_| CanonError::WrongType)?;
    for (id, tuples) in &data.preds {
        let mut tuple_items = Vec::new();
        tuple_items
            .try_reserve_exact(tuples.len())
            .map_err(|_| CanonError::WrongType)?;
        for tuple in tuples {
            let t_items = tuple
                .iter()
                .map(|&v| Value::Unsigned(u64::from(v)))
                .collect();
            let t_val = Array::new(t_items).map(Value::Array)?;
            tuple_items.push(t_val);
        }
        let tuples_val = Array::new(tuple_items).map(Value::Array)?;
        let pair =
            Array::new(vec![Value::Unsigned(u64::from(*id)), tuples_val]).map(Value::Array)?;
        pred_items.push(pair);
    }
    let preds_val = Array::new(pred_items).map(Value::Array)?;

    // Envelope raiz com 6 elementos
    Array::new(vec![
        version_val,
        hv_val,
        domains_val,
        consts_val,
        funcs_val,
        preds_val,
    ])
    .map(Value::Array)
}

/// Decodifica e valida um envelope binário `model/v1` sob os limites fornecidos.
pub(crate) fn decode_model_with_limits(
    bytes: &[u8],
    vocabulary: &Vocabulary,
    limits: ModelLimits,
) -> Result<ModelData, ModelError> {
    // 1. Verificação prévia do comprimento total em bytes antes de alocar generic Value
    if bytes.len() > limits.max_bytes {
        return Err(ModelError::Unknown(Exhausted::Size));
    }

    // 2. Leitura canônica estrita pelo Decoder
    let mut decoder = Decoder::new(bytes);
    let root_val = decoder.value()?;
    decoder.finish()?;

    // 3. Inspeção estrutural do array raiz de 6 posições
    let root_items = match &root_val {
        Value::Array(arr) => arr.items(),
        _ => return Err(CanonError::WrongType.into()),
    };
    if root_items.len() != 6 {
        return Err(CanonError::WrongType.into());
    }

    // 4. Versão deve ser exatamente 1
    match root_items[0] {
        Value::Unsigned(1) => {}
        _ => return Err(CanonError::WrongType.into()),
    }

    // 5. Hash do vocabulário verificado antes de processar tabelas
    let hv_bytes = match &root_items[1] {
        Value::Bytes(b) => <[u8; 32]>::try_from(b.as_slice()).map_err(|_| CanonError::WrongType)?,
        _ => return Err(CanonError::WrongType.into()),
    };
    let expected_hv = logos_crypto::hash(vocabulary)?;
    if hv_bytes != *expected_hv.as_bytes() {
        return Err(ModelError::VocabularyMismatch);
    }

    // 6. Resolução e validação de assinaturas do vocabulário
    let (sort_map, symbol_map) = validate_vocabulary_signatures(vocabulary)?;

    // 7. Decodificação dos domínios
    let raw_domains = match &root_items[2] {
        Value::Array(arr) => arr.items(),
        _ => return Err(CanonError::WrongType.into()),
    };
    if raw_domains.len() != vocabulary.sorts().len() {
        return Err(ModelError::InvalidDomains);
    }
    let mut domains = Vec::new();
    domains
        .try_reserve_exact(raw_domains.len())
        .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;
    for item in raw_domains {
        let u = match item {
            Value::Unsigned(u) => *u,
            _ => return Err(CanonError::WrongType.into()),
        };
        let card = u32::try_from(u).map_err(|_| CanonError::IntOutOfRange)?;
        if card == 0 {
            return Err(ModelError::InvalidDomains);
        }
        if card > limits.max_domain_size {
            return Err(ModelError::Unknown(Exhausted::Size));
        }
        domains.push(card);
    }

    // 8. Checagem de cardinalidades de sorts enumerados
    let mut primitive_enum_ordinals = BTreeMap::new();
    for (s_idx, sort_decl) in vocabulary.sorts().iter().enumerate() {
        if let SortDomain::Enumerated { constants, .. } = &sort_decl.domain {
            if domains[s_idx] as usize != constants.len() {
                return Err(ModelError::InvalidEnumeration);
            }
            for (ordinal, const_sym) in constants.iter().enumerate() {
                let sym_idx = *symbol_map
                    .get(&const_sym.name)
                    .ok_or(ModelError::InvalidInterpretation)?;
                let sym_decl = &vocabulary.symbols()[sym_idx];
                if !matches!(sym_decl.grounding, Grounding::Defined(_)) {
                    let ord =
                        u32::try_from(ordinal).map_err(|_| ModelError::Unknown(Exhausted::Size))?;
                    primitive_enum_ordinals.insert(sym_idx, ord);
                }
            }
        }
    }

    // 9. Extração dos arrays de constantes, funções e predicados
    let raw_consts = match &root_items[3] {
        Value::Array(arr) => arr.items(),
        _ => return Err(CanonError::WrongType.into()),
    };
    let raw_funcs = match &root_items[4] {
        Value::Array(arr) => arr.items(),
        _ => return Err(CanonError::WrongType.into()),
    };
    let raw_preds = match &root_items[5] {
        Value::Array(arr) => arr.items(),
        _ => return Err(CanonError::WrongType.into()),
    };

    let mut consts = Vec::new();
    consts
        .try_reserve_exact(raw_consts.len())
        .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;
    let mut funcs = Vec::new();
    funcs
        .try_reserve_exact(raw_funcs.len())
        .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;
    let mut preds = Vec::new();
    preds
        .try_reserve_exact(raw_preds.len())
        .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;

    // 10. Varredura com cursores monótonos para decodificação segura
    let mut const_cursor: usize = 0;
    let mut func_cursor: usize = 0;
    let mut pred_cursor: usize = 0;
    let mut total_cells: usize = 0;

    for (sym_idx, sym_decl) in vocabulary.symbols().iter().enumerate() {
        let sym_id = u32::try_from(sym_idx).map_err(|_| ModelError::Unknown(Exhausted::Size))?;
        if matches!(sym_decl.grounding, Grounding::Defined(_)) {
            continue;
        }

        let sig = &sym_decl.signature;
        match sig.kind {
            SymbolKind::Const => {
                if const_cursor >= raw_consts.len() {
                    return Err(ModelError::InvalidInterpretation);
                }
                let pair = match &raw_consts[const_cursor] {
                    Value::Array(arr) => arr.items(),
                    _ => return Err(CanonError::WrongType.into()),
                };
                if pair.len() != 2 {
                    return Err(CanonError::WrongType.into());
                }
                let id_u = match pair[0] {
                    Value::Unsigned(u) => u,
                    _ => return Err(CanonError::WrongType.into()),
                };
                let id = u32::try_from(id_u).map_err(|_| CanonError::IntOutOfRange)?;
                if id != sym_id {
                    return Err(ModelError::InvalidInterpretation);
                }
                let val_u = match pair[1] {
                    Value::Unsigned(u) => u,
                    _ => return Err(CanonError::WrongType.into()),
                };
                let val = u32::try_from(val_u).map_err(|_| CanonError::IntOutOfRange)?;

                total_cells = total_cells
                    .checked_add(1)
                    .ok_or(ModelError::Unknown(Exhausted::Size))?;
                if total_cells > limits.max_cells {
                    return Err(ModelError::Unknown(Exhausted::Size));
                }
                if let Some(&expected_ordinal) = primitive_enum_ordinals.get(&sym_idx)
                    && val != expected_ordinal
                {
                    return Err(ModelError::InvalidEnumeration);
                }
                let res_sort_name = match &sig.result {
                    SymbolResult::Sort(s) => &s.name,
                    _ => return Err(ModelError::InvalidSignature),
                };
                let res_s_idx = *sort_map
                    .get(res_sort_name)
                    .ok_or(ModelError::InvalidSignature)?;
                let res_card = domains[res_s_idx];
                if val >= res_card {
                    return Err(ModelError::ValueOutOfDomain);
                }
                consts.push((id, val));
                const_cursor += 1;
            }
            SymbolKind::Func => {
                if func_cursor >= raw_funcs.len() {
                    return Err(ModelError::InvalidInterpretation);
                }
                let pair = match &raw_funcs[func_cursor] {
                    Value::Array(arr) => arr.items(),
                    _ => return Err(CanonError::WrongType.into()),
                };
                if pair.len() != 2 {
                    return Err(CanonError::WrongType.into());
                }
                let id_u = match pair[0] {
                    Value::Unsigned(u) => u,
                    _ => return Err(CanonError::WrongType.into()),
                };
                let id = u32::try_from(id_u).map_err(|_| CanonError::IntOutOfRange)?;
                if id != sym_id {
                    return Err(ModelError::InvalidInterpretation);
                }
                let packed_bytes = match &pair[1] {
                    Value::Bytes(b) => b.as_slice(),
                    _ => return Err(CanonError::WrongType.into()),
                };

                // Checagem de produto/células ANTES de alocar tabela tipada
                let mut prod: usize = 1;
                for arg in &sig.arity {
                    let arg_s_idx = *sort_map
                        .get(&arg.name)
                        .ok_or(ModelError::InvalidSignature)?;
                    let card = domains[arg_s_idx] as usize;
                    prod = prod
                        .checked_mul(card)
                        .ok_or(ModelError::Unknown(Exhausted::Size))?;
                    if prod > limits.max_cells {
                        return Err(ModelError::Unknown(Exhausted::Size));
                    }
                }
                total_cells = total_cells
                    .checked_add(prod)
                    .ok_or(ModelError::Unknown(Exhausted::Size))?;
                if total_cells > limits.max_cells {
                    return Err(ModelError::Unknown(Exhausted::Size));
                }

                // Tamanho de saída compactada deve ser divisível por 4 e igual a prod * 4
                let expected_bytes_len = prod
                    .checked_mul(4)
                    .ok_or(ModelError::Unknown(Exhausted::Size))?;
                if packed_bytes.len() != expected_bytes_len {
                    return Err(ModelError::NonTotalFunction);
                }

                let mut outputs = Vec::new();
                outputs
                    .try_reserve_exact(prod)
                    .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;

                let res_sort_name = match &sig.result {
                    SymbolResult::Sort(s) => &s.name,
                    _ => return Err(ModelError::InvalidSignature),
                };
                let res_s_idx = *sort_map
                    .get(res_sort_name)
                    .ok_or(ModelError::InvalidSignature)?;
                let res_card = domains[res_s_idx];

                for &chunk in packed_bytes.as_chunks::<4>().0 {
                    let out_val = u32::from_be_bytes(chunk);
                    if out_val >= res_card {
                        return Err(ModelError::ValueOutOfDomain);
                    }
                    outputs.push(out_val);
                }
                funcs.push((id, outputs));
                func_cursor += 1;
            }
            SymbolKind::Pred => {
                if pred_cursor >= raw_preds.len() {
                    return Err(ModelError::InvalidInterpretation);
                }
                let pair = match &raw_preds[pred_cursor] {
                    Value::Array(arr) => arr.items(),
                    _ => return Err(CanonError::WrongType.into()),
                };
                if pair.len() != 2 {
                    return Err(CanonError::WrongType.into());
                }
                let id_u = match pair[0] {
                    Value::Unsigned(u) => u,
                    _ => return Err(CanonError::WrongType.into()),
                };
                let id = u32::try_from(id_u).map_err(|_| CanonError::IntOutOfRange)?;
                if id != sym_id {
                    return Err(ModelError::InvalidInterpretation);
                }
                let raw_tuples = match &pair[1] {
                    Value::Array(arr) => arr.items(),
                    _ => return Err(CanonError::WrongType.into()),
                };

                let k = sig.arity.len();
                let w = k.max(1);
                let cells = raw_tuples
                    .len()
                    .checked_mul(w)
                    .ok_or(ModelError::Unknown(Exhausted::Size))?;
                total_cells = total_cells
                    .checked_add(cells)
                    .ok_or(ModelError::Unknown(Exhausted::Size))?;
                if total_cells > limits.max_cells {
                    return Err(ModelError::Unknown(Exhausted::Size));
                }

                let mut arg_cards = Vec::new();
                arg_cards
                    .try_reserve_exact(if raw_tuples.is_empty() { 0 } else { k })
                    .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;
                if !raw_tuples.is_empty() {
                    for arg in &sig.arity {
                        let arg_s_idx = *sort_map
                            .get(&arg.name)
                            .ok_or(ModelError::InvalidSignature)?;
                        arg_cards.push(domains[arg_s_idx]);
                    }
                }

                let mut tuples = Vec::new();
                tuples
                    .try_reserve_exact(raw_tuples.len())
                    .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;

                for (t_idx, raw_t) in raw_tuples.iter().enumerate() {
                    let t_items = match raw_t {
                        Value::Array(arr) => arr.items(),
                        _ => return Err(CanonError::WrongType.into()),
                    };
                    if t_items.len() != k {
                        return Err(ModelError::InvalidTuple);
                    }
                    let mut tuple = Vec::new();
                    tuple
                        .try_reserve_exact(k)
                        .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;
                    for (axis, item) in t_items.iter().enumerate() {
                        let v_u = match item {
                            Value::Unsigned(u) => *u,
                            _ => return Err(CanonError::WrongType.into()),
                        };
                        let v = u32::try_from(v_u).map_err(|_| CanonError::IntOutOfRange)?;
                        if v >= arg_cards[axis] {
                            return Err(ModelError::ValueOutOfDomain);
                        }
                        tuple.push(v);
                    }
                    if t_idx > 0 && tuples[t_idx - 1] >= tuple {
                        return Err(ModelError::InvalidTuple);
                    }
                    tuples.push(tuple);
                }
                preds.push((id, tuples));
                pred_cursor += 1;
            }
        }
    }

    if const_cursor != raw_consts.len()
        || func_cursor != raw_funcs.len()
        || pred_cursor != raw_preds.len()
    {
        return Err(ModelError::InvalidInterpretation);
    }

    Ok(ModelData {
        vocab: expected_hv,
        domains,
        consts,
        funcs,
        preds,
    })
}
