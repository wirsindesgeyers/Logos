//! Frases fixas da língua controlada paraphrase-pt/v1 (`spec/paraphrase-pt.md`).
//!
//! Renderizador e parser usam as mesmas constantes; nenhum texto estrutural
//! vem de templates, que não podem conter `⟦`, `⟧`, `{` ou `}` (ADR0046).

/// Identificador da gramática, incluído no hash da paráfrase.
pub const GRAMMAR_ID: &str = "paraphrase-pt/v1";
/// Versão do renderizador, incluída no hash da paráfrase.
pub const RENDERER_VERSION: &str = "logos-seal/render_pt/1";

pub(crate) const OPEN: &str = "⟦";
pub(crate) const CLOSE: &str = "⟧";
pub(crate) const OPEN_CHAR: char = '⟦';
pub(crate) const CLOSE_CHAR: char = '⟧';

pub(crate) const TRUE: &str = "verdadeiro";
pub(crate) const FALSE: &str = "falso";
pub(crate) const EQ_OPEN: &str = "o valor ";
pub(crate) const EQ_MIDDLE: &str = " é igual a ";
pub(crate) const NOT_OPEN: &str = "não é o caso que (";
pub(crate) const AND_OPEN: &str = "vale cada uma de ";
pub(crate) const OR_OPEN: &str = "vale pelo menos uma de ";
pub(crate) const ONE_CONDITION: &str = " condição";
pub(crate) const MANY_CONDITIONS: &str = " condições";
pub(crate) const LIST_FIRST: &str = ": (";
pub(crate) const LIST_NEXT: &str = "; (";
pub(crate) const IMPLIES_OPEN: &str = "se (";
pub(crate) const IMPLIES_MIDDLE: &str = "), então (";
pub(crate) const IFF_OPEN: &str = "(";
pub(crate) const IFF_MIDDLE: &str = ") se e somente se (";
pub(crate) const FORALL_OPEN: &str = "para todo ";
pub(crate) const EXISTS_OPEN: &str = "existe ";
pub(crate) const SORT_OPEN: &str = " do tipo ⟦";
pub(crate) const FORALL_BODY: &str = "⟧, vale que (";
pub(crate) const EXISTS_BODY: &str = "⟧ tal que (";
pub(crate) const GROUP_CLOSE: &str = ")";
pub(crate) const VARIABLE_PREFIX: char = 'x';

/// Separa um template validado em `n+1` segmentos literais ao redor de `{0}`…`{n-1}`.
pub(crate) fn segments(template: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push(&rest[..open]);
        rest = rest[open..]
            .find('}')
            .map_or("", |close| &rest[open + close + 1..]);
    }
    out.push(rest);
    out
}

/// Número decimal canônico (sem zero à esquerda, exceto o próprio `0`).
pub(crate) fn parse_decimal(digits: &str) -> Option<usize> {
    if digits.is_empty()
        || !digits.bytes().all(|b| b.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
    {
        return None;
    }
    digits.parse().ok()
}

/// `xK` com `K ≥ 1` canônico; devolve `K`.
pub(crate) fn variable_ordinal(name: &str) -> Option<usize> {
    let digits = name.strip_prefix(VARIABLE_PREFIX)?;
    parse_decimal(digits).filter(|&ordinal| ordinal >= 1)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn segments_split_placeholders_in_order() {
        assert_eq!(segments("{0} é dono de {1}"), vec!["", " é dono de ", ""]);
        assert_eq!(segments("chove"), vec!["chove"]);
        assert_eq!(segments("o pai de {0}"), vec!["o pai de ", ""]);
    }

    #[test]
    fn decimal_and_variable_forms_are_canonical() {
        assert_eq!(parse_decimal("0"), Some(0));
        assert_eq!(parse_decimal("10"), Some(10));
        assert_eq!(parse_decimal("01"), None);
        assert_eq!(parse_decimal("+1"), None);
        assert_eq!(parse_decimal(""), None);
        assert_eq!(variable_ordinal("x1"), Some(1));
        assert_eq!(variable_ordinal("x0"), None);
        assert_eq!(variable_ordinal("x01"), None);
        assert_eq!(variable_ordinal("y1"), None);
        assert_eq!(variable_ordinal("x"), None);
    }
}
