//! Validação semântica de modelos finitos em relação a um vocabulário.

use std::collections::BTreeMap;

use logos_air::{Grounding, SortDomain, SymbolKind, SymbolResult, Vocabulary};
use logos_canon::Text;
use logos_core::Exhausted;

use crate::data::ModelData;
use crate::error::ModelError;
use crate::exact_size::exact_wire_len;
use crate::limits::ModelLimits;

/// Mapeamentos de nomes de sorts e símbolos para seus índices ordinais na declaração do vocabulário.
pub(crate) type SymbolIndices<'a> = (BTreeMap<&'a Text, usize>, BTreeMap<&'a Text, usize>);

/// Valida as assinaturas de todos os símbolos declarados no vocabulário,
/// garantindo correspondência com sorts declarados, aridades permitidas e tipos de resultado.
pub(crate) fn validate_vocabulary_signatures(
    vocabulary: &Vocabulary,
) -> Result<SymbolIndices<'_>, ModelError> {
    let mut sort_map = BTreeMap::new();
    for (idx, sort_decl) in vocabulary.sorts().iter().enumerate() {
        sort_map.insert(&sort_decl.name, idx);
    }

    let mut symbol_map = BTreeMap::new();
    for (idx, sym_decl) in vocabulary.symbols().iter().enumerate() {
        symbol_map.insert(&sym_decl.signature.name, idx);
        let sig = &sym_decl.signature;
        match sig.kind {
            SymbolKind::Const => {
                if !sig.arity.is_empty() {
                    return Err(ModelError::InvalidSignature);
                }
                match &sig.result {
                    SymbolResult::Sort(res_sort) => {
                        if !sort_map.contains_key(&res_sort.name) {
                            return Err(ModelError::InvalidSignature);
                        }
                    }
                    SymbolResult::Bool => return Err(ModelError::InvalidSignature),
                }
            }
            SymbolKind::Func => {
                match &sig.result {
                    SymbolResult::Sort(res_sort) => {
                        if !sort_map.contains_key(&res_sort.name) {
                            return Err(ModelError::InvalidSignature);
                        }
                    }
                    SymbolResult::Bool => return Err(ModelError::InvalidSignature),
                }
                for arg in &sig.arity {
                    if !sort_map.contains_key(&arg.name) {
                        return Err(ModelError::InvalidSignature);
                    }
                }
            }
            SymbolKind::Pred => {
                if sig.result != SymbolResult::Bool {
                    return Err(ModelError::InvalidSignature);
                }
                for arg in &sig.arity {
                    if !sort_map.contains_key(&arg.name) {
                        return Err(ModelError::InvalidSignature);
                    }
                }
            }
        }
    }

    Ok((sort_map, symbol_map))
}

/// Valida semanticamente a estrutura `ModelData` fornecida contra o vocabulário e limites.
pub(crate) fn validate_model_data(
    data: &ModelData,
    vocabulary: &Vocabulary,
    limits: ModelLimits,
) -> Result<(), ModelError> {
    // 1. Verificação prévia de tamanho no fio
    let wire_len = exact_wire_len(data)?;
    if wire_len > limits.max_bytes {
        return Err(ModelError::Unknown(Exhausted::Size));
    }

    // 2. Vinculação criptográfica obrigatória ao vocabulário
    let expected_hv = logos_crypto::hash(vocabulary)?;
    if data.vocab != expected_hv {
        return Err(ModelError::VocabularyMismatch);
    }

    // 3. Validação de assinaturas e resolução de sorts/símbolos
    let (sort_map, symbol_map) = validate_vocabulary_signatures(vocabulary)?;

    // 4. Validação de domínios (FOL clássica: domínios não vazios e finitos)
    if data.domains.len() != vocabulary.sorts().len() {
        return Err(ModelError::InvalidDomains);
    }
    for &d in &data.domains {
        if d == 0 {
            return Err(ModelError::InvalidDomains);
        }
        if d > limits.max_domain_size {
            return Err(ModelError::Unknown(Exhausted::Size));
        }
    }

    // 5. Validação de cardinalidades de sorts enumerados
    let mut primitive_enum_ordinals = BTreeMap::new();
    for (s_idx, sort_decl) in vocabulary.sorts().iter().enumerate() {
        if let SortDomain::Enumerated { constants, .. } = &sort_decl.domain {
            if data.domains[s_idx] as usize != constants.len() {
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

    // 6. Varredura com cursores monótonos pelas declarações de símbolos
    let mut const_cursor: usize = 0;
    let mut func_cursor: usize = 0;
    let mut pred_cursor: usize = 0;
    let mut total_cells: usize = 0;

    for (sym_idx, sym_decl) in vocabulary.symbols().iter().enumerate() {
        let sym_id = u32::try_from(sym_idx).map_err(|_| ModelError::Unknown(Exhausted::Size))?;
        if matches!(sym_decl.grounding, Grounding::Defined(_)) {
            // Símbolos Defined não possuem tabela de interpretação direta
            continue;
        }

        let sig = &sym_decl.signature;
        match sig.kind {
            SymbolKind::Const => {
                if const_cursor >= data.consts.len() {
                    return Err(ModelError::InvalidInterpretation);
                }
                let (id, val) = data.consts[const_cursor];
                if id != sym_id {
                    return Err(ModelError::InvalidInterpretation);
                }
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
                let res_card = data.domains[res_s_idx];
                if val >= res_card {
                    return Err(ModelError::ValueOutOfDomain);
                }
                const_cursor += 1;
            }
            SymbolKind::Func => {
                if func_cursor >= data.funcs.len() {
                    return Err(ModelError::InvalidInterpretation);
                }
                let (id, ref outputs) = data.funcs[func_cursor];
                if id != sym_id {
                    return Err(ModelError::InvalidInterpretation);
                }
                // Produto das cardinalidades dos argumentos verificado ANTES
                let mut prod: usize = 1;
                for arg in &sig.arity {
                    let arg_s_idx = *sort_map
                        .get(&arg.name)
                        .ok_or(ModelError::InvalidSignature)?;
                    let card = data.domains[arg_s_idx] as usize;
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
                if outputs.len() != prod {
                    return Err(ModelError::NonTotalFunction);
                }
                let res_sort_name = match &sig.result {
                    SymbolResult::Sort(s) => &s.name,
                    _ => return Err(ModelError::InvalidSignature),
                };
                let res_s_idx = *sort_map
                    .get(res_sort_name)
                    .ok_or(ModelError::InvalidSignature)?;
                let res_card = data.domains[res_s_idx];
                for &out_val in outputs {
                    if out_val >= res_card {
                        return Err(ModelError::ValueOutOfDomain);
                    }
                }
                func_cursor += 1;
            }
            SymbolKind::Pred => {
                if pred_cursor >= data.preds.len() {
                    return Err(ModelError::InvalidInterpretation);
                }
                let (id, ref tuples) = data.preds[pred_cursor];
                if id != sym_id {
                    return Err(ModelError::InvalidInterpretation);
                }
                let k = sig.arity.len();
                let w = k.max(1);
                let cells = tuples
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
                    .try_reserve_exact(if tuples.is_empty() { 0 } else { k })
                    .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;
                if !tuples.is_empty() {
                    for arg in &sig.arity {
                        let arg_s_idx = *sort_map
                            .get(&arg.name)
                            .ok_or(ModelError::InvalidSignature)?;
                        arg_cards.push(data.domains[arg_s_idx]);
                    }
                }
                for (t_idx, tuple) in tuples.iter().enumerate() {
                    if tuple.len() != k {
                        return Err(ModelError::InvalidTuple);
                    }
                    for (axis, &v) in tuple.iter().enumerate() {
                        if v >= arg_cards[axis] {
                            return Err(ModelError::ValueOutOfDomain);
                        }
                    }
                    if t_idx > 0 && tuples[t_idx - 1] >= *tuple {
                        return Err(ModelError::InvalidTuple);
                    }
                }
                pred_cursor += 1;
            }
        }
    }

    if const_cursor != data.consts.len()
        || func_cursor != data.funcs.len()
        || pred_cursor != data.preds.len()
    {
        return Err(ModelError::InvalidInterpretation);
    }

    Ok(())
}
