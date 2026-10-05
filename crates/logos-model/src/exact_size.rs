//! Medição exata de tamanho em bytes da codificação CBOR canônica sem alocação.

use crate::data::ModelData;
use crate::error::ModelError;
use logos_core::Exhausted;

/// Retorna o tamanho do cabeçalho CBOR (major type + argumento) para o argumento numérico dado.
#[inline]
pub(crate) const fn cbor_head_len(arg: u64) -> usize {
    if arg < 24 {
        1
    } else if arg <= 0xFF {
        2
    } else if arg <= 0xFFFF {
        3
    } else if arg <= 0xFFFF_FFFF {
        5
    } else {
        9
    }
}

/// Mede o comprimento exato em bytes da codificação CBOR canônica de `ModelData`,
/// sem alocar estruturas intermediárias de `Value`.
pub(crate) fn exact_wire_len(data: &ModelData) -> Result<usize, ModelError> {
    // Array raiz com 6 itens
    let mut total: usize = cbor_head_len(6);

    // 0. Versão unsigned 1
    total = total
        .checked_add(cbor_head_len(1))
        .ok_or(ModelError::Unknown(Exhausted::Size))?;

    // 1. hV: byte string de 32 bytes (head 2 bytes + 32 bytes de conteúdo)
    let hv_len = cbor_head_len(32)
        .checked_add(32)
        .ok_or(ModelError::Unknown(Exhausted::Size))?;
    total = total
        .checked_add(hv_len)
        .ok_or(ModelError::Unknown(Exhausted::Size))?;

    // 2. domains: array de u32
    let mut domains_len = cbor_head_len(data.domains.len() as u64);
    for &d in &data.domains {
        domains_len = domains_len
            .checked_add(cbor_head_len(u64::from(d)))
            .ok_or(ModelError::Unknown(Exhausted::Size))?;
    }
    total = total
        .checked_add(domains_len)
        .ok_or(ModelError::Unknown(Exhausted::Size))?;

    // 3. consts: array de [id, val]
    let mut consts_len = cbor_head_len(data.consts.len() as u64);
    for &(id, val) in &data.consts {
        let pair_len = cbor_head_len(2)
            .checked_add(cbor_head_len(u64::from(id)))
            .ok_or(ModelError::Unknown(Exhausted::Size))?
            .checked_add(cbor_head_len(u64::from(val)))
            .ok_or(ModelError::Unknown(Exhausted::Size))?;
        consts_len = consts_len
            .checked_add(pair_len)
            .ok_or(ModelError::Unknown(Exhausted::Size))?;
    }
    total = total
        .checked_add(consts_len)
        .ok_or(ModelError::Unknown(Exhausted::Size))?;

    // 4. funcs: array de [id, bytes_packed]
    let mut funcs_len = cbor_head_len(data.funcs.len() as u64);
    for (id, outs) in &data.funcs {
        let byte_len = outs
            .len()
            .checked_mul(4)
            .ok_or(ModelError::Unknown(Exhausted::Size))?;
        let bytes_item_len = cbor_head_len(byte_len as u64)
            .checked_add(byte_len)
            .ok_or(ModelError::Unknown(Exhausted::Size))?;
        let pair_len = cbor_head_len(2)
            .checked_add(cbor_head_len(u64::from(*id)))
            .ok_or(ModelError::Unknown(Exhausted::Size))?
            .checked_add(bytes_item_len)
            .ok_or(ModelError::Unknown(Exhausted::Size))?;
        funcs_len = funcs_len
            .checked_add(pair_len)
            .ok_or(ModelError::Unknown(Exhausted::Size))?;
    }
    total = total
        .checked_add(funcs_len)
        .ok_or(ModelError::Unknown(Exhausted::Size))?;

    // 5. preds: array de [id, [[tuple u32...]...]]
    let mut preds_len = cbor_head_len(data.preds.len() as u64);
    for (id, tuples) in &data.preds {
        let mut tuples_len = cbor_head_len(tuples.len() as u64);
        for tuple in tuples {
            let mut tuple_len = cbor_head_len(tuple.len() as u64);
            for &val in tuple {
                tuple_len = tuple_len
                    .checked_add(cbor_head_len(u64::from(val)))
                    .ok_or(ModelError::Unknown(Exhausted::Size))?;
            }
            tuples_len = tuples_len
                .checked_add(tuple_len)
                .ok_or(ModelError::Unknown(Exhausted::Size))?;
        }
        let pair_len = cbor_head_len(2)
            .checked_add(cbor_head_len(u64::from(*id)))
            .ok_or(ModelError::Unknown(Exhausted::Size))?
            .checked_add(tuples_len)
            .ok_or(ModelError::Unknown(Exhausted::Size))?;
        preds_len = preds_len
            .checked_add(pair_len)
            .ok_or(ModelError::Unknown(Exhausted::Size))?;
    }
    total = total
        .checked_add(preds_len)
        .ok_or(ModelError::Unknown(Exhausted::Size))?;

    Ok(total)
}
