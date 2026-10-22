//! Escopos normativos de assumption e escopo derivado de uma proposta (5.7, ADR0054).
//!
//! O texto `scope` do evento continua opaco para o índice; aqui ele ganha três
//! formas aceitas pelo ASSUME. O PROPOSE não escolhe escopo: ele é o escopo único
//! das assumptions alcançáveis pelas citações, e as mesmas regras valem na
//! regeneração de CERTIFY/WITNESS.

use logos_crypto::Hash;
use logos_ledger::{Event, LedgerLiveIndex};

use crate::TransportError as Error;

/// Forma normativa de um escopo declarado por ASSUME.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Scope {
    /// `session:<rótulo>`: agrupamento declarado, sem restrição adicional.
    Session,
    /// `until:<época>`: citável enquanto a época corrente for ≤ a época dada.
    Until(u64),
    /// `claim:<h(Event)>`: citável só por propostas que também citem esse claim.
    Claim(Hash<Event>),
}

/// Lê uma das formas normativas; qualquer outro texto é `None`.
pub(crate) fn parse(text: &str) -> Option<Scope> {
    if let Some(label) = text.strip_prefix("session:") {
        let valid = (1..=64).contains(&label.len())
            && label
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b));
        return valid.then_some(Scope::Session);
    }
    if let Some(digits) = text.strip_prefix("until:") {
        let canonical = !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit())
            && (digits == "0" || !digits.starts_with('0'));
        return canonical
            .then(|| digits.parse().ok().map(Scope::Until))
            .flatten();
    }
    if let Some(hex) = text.strip_prefix("claim:") {
        if hex.len() != 64
            || !hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return None;
        }
        let mut digest = [0u8; 32];
        for (i, slot) in digest.iter_mut().enumerate() {
            *slot = u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok()?;
        }
        return Some(Scope::Claim(Hash::from_bytes(digest)));
    }
    None
}

/// Escopo derivado das citações: vazio sem assumptions, o único escopo comum,
/// ou recusa. `until`/`claim` são conferidos contra a época e as citações.
pub(crate) fn for_proposal(
    index: &LedgerLiveIndex<'_>,
    cites: &[Hash<Event>],
    epoch: u64,
) -> Result<String, Error> {
    let mut scopes = std::collections::BTreeSet::new();
    for reference in cites {
        if let Some(open) = index.open_assumptions(*reference)? {
            scopes.extend(open.into_iter().map(|(_, scope)| scope));
        }
    }
    let mut scopes = scopes.into_iter();
    let Some(scope) = scopes.next() else {
        return Ok(String::new());
    };
    if scopes.next().is_some() {
        return Err(Error::Invalid("mixed assumption scopes"));
    }
    match parse(&scope) {
        Some(Scope::Until(limit)) if epoch > limit => {
            Err(Error::Invalid("assumption scope expired"))
        }
        Some(Scope::Claim(claim)) if !cites.contains(&claim) => {
            Err(Error::Invalid("assumption scope requires citing its claim"))
        }
        // Escopos históricos fora das formas normativas continuam rótulos exatos.
        _ => Ok(scope),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn only_normative_forms_parse() {
        assert_eq!(parse("session:estudo-1"), Some(Scope::Session));
        assert_eq!(parse("until:0"), Some(Scope::Until(0)));
        assert_eq!(
            parse("until:18446744073709551615"),
            Some(Scope::Until(u64::MAX))
        );
        let h = "ab".repeat(32);
        assert_eq!(
            parse(&format!("claim:{h}")),
            Some(Scope::Claim(Hash::from_bytes([0xab; 32])))
        );
        for bad in [
            "",
            "local",
            "session:",
            "session:Maiúscula",
            "until:01",
            "until:",
            "until:-1",
            "until:18446744073709551616",
            "claim:AB",
            &format!("claim:{}", "AB".repeat(32)),
        ] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }
}
