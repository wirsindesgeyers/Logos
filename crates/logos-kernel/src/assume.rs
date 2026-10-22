//! ASSUME (5.7, ADR0054): o kernel registra a hipótese declarada pelo agente.
//!
//! Autor = chave do kernel: a assumption é *do agente*, nunca do usuário, e nunca
//! satisfaz requisito de confiança de política para ações (8.2). Assumptions de
//! usuário entram por caminho local de operador, não por este socket.

use logos_air::{Formula, Limits, Vocabulary, check};
use logos_canon::{Array, Canon, Text, Value};
use logos_crypto::{Hash, Signer, hash};
use logos_ledger::{EventBody, Ledger};

use crate::{
    TransportError as Error,
    scope::{Scope, parse},
};

pub(crate) fn declare<S: Signer>(
    ledger: &mut Ledger<'_, S>,
    vocabulary: Hash<Vocabulary>,
    formula: Formula,
    scope: Text,
) -> Result<Value, Error> {
    let head = ledger
        .head()?
        .ok_or(Error::Invalid("ASSUME requires Genesis"))?;
    match parse(scope.as_str()) {
        None => {
            return Err(Error::Invalid(
                "assumption scope is not session/until/claim",
            ));
        }
        Some(Scope::Until(limit)) if limit < head.epoch => {
            return Err(Error::Invalid("assumption scope already expired"));
        }
        Some(Scope::Claim(claim)) if !ledger.live_index()?.is_citable(claim, head.epoch)? => {
            return Err(Error::Invalid("assumption scope claim is not citable"));
        }
        Some(_) => {}
    }
    let bytes = ledger
        .cas()
        .get(&vocabulary)?
        .ok_or(Error::Invalid("missing assumption vocabulary"))?;
    let decoded = Vocabulary::decode_with_limits(&bytes, Limits::default())
        .map_err(|_| Error::Invalid("invalid assumption vocabulary"))?;
    check(&decoded, &formula)
        .map_err(|_| Error::Invalid("assumption is not closed and well-sorted"))?;
    let author = ledger
        .events_from(0, 1)?
        .first()
        .ok_or(Error::Invalid("ASSUME requires Genesis"))?
        .kernel_key;
    let event = ledger.append(EventBody::AssumptionDeclared {
        vocabulary,
        formula,
        scope,
        author,
    })?;
    // assume-result/v1 = [1, h(AssumptionDeclared)].
    Ok(Value::Array(Array::new(vec![
        Value::Unsigned(1),
        hash(&event)?.to_value()?,
    ])?))
}
