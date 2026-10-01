//! Strings de texto canônicas: UTF-8 em NFC, só com code points atribuídos.

use crate::error::CanonError;
use crate::unicode_assigned::ASSIGNED;
use unicode_normalization::UnicodeNormalization;

/// Texto válido para a codificação canônica.
///
/// Só se constrói por [`Text::new`], que rejeita code points não atribuídos na versão de
/// Unicode fixada ([`crate::UNICODE_VERSION`]) e strings fora de NFC. Assim a codificação de um
/// `Text` nunca falha e `decode(encode(x)) == x`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Text(String);

impl Text {
    /// Valida e embrulha uma string.
    ///
    /// # Errors
    /// [`CanonError::UnassignedCodePoint`] se houver code point não atribuído;
    /// [`CanonError::NotNfc`] se a string não estiver em NFC. (Nessa ordem.)
    pub fn new(s: impl Into<String>) -> Result<Self, CanonError> {
        let s = s.into();
        if !s.chars().all(is_assigned) {
            return Err(CanonError::UnassignedCodePoint);
        }
        if !s.chars().nfc().eq(s.chars()) {
            return Err(CanonError::NotNfc);
        }
        Ok(Self(s))
    }

    /// A string validada.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `true` se `c` está atribuído (categoria geral diferente de `Cn`) em [`crate::UNICODE_VERSION`].
fn is_assigned(c: char) -> bool {
    let cp = u32::from(c);
    ASSIGNED
        .binary_search_by(|&(lo, hi)| {
            if cp < lo {
                core::cmp::Ordering::Greater
            } else if cp > hi {
                core::cmp::Ordering::Less
            } else {
                core::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_and_accents_are_accepted() {
        assert!(Text::new("logos").is_ok());
        assert!(Text::new("\u{e9}").is_ok()); // é precomposto (NFC)
        assert!(Text::new("").is_ok());
    }

    #[test]
    fn decomposed_is_not_nfc() {
        assert_eq!(Text::new("e\u{301}"), Err(CanonError::NotNfc));
    }

    #[test]
    fn unassigned_and_noncharacters_are_rejected() {
        assert_eq!(Text::new("\u{378}"), Err(CanonError::UnassignedCodePoint));
        assert_eq!(Text::new("\u{FFFF}"), Err(CanonError::UnassignedCodePoint));
        assert_eq!(
            Text::new("\u{10FFFF}"),
            Err(CanonError::UnassignedCodePoint)
        );
        assert_eq!(Text::new("\u{E000}"), Ok(Text("\u{E000}".to_string()))); // uso privado: atribuído
    }

    #[test]
    fn unassigned_wins_over_not_nfc() {
        assert_eq!(
            Text::new("e\u{301}\u{378}"),
            Err(CanonError::UnassignedCodePoint)
        );
    }

    #[test]
    fn table_is_sorted_and_disjoint() {
        for w in ASSIGNED.windows(2) {
            assert!(w[0].0 <= w[0].1 && w[0].1 + 1 < w[1].0);
        }
    }

    #[test]
    fn table_version_matches_normalization_crate() {
        assert_eq!(
            crate::unicode_assigned::UNICODE_VERSION,
            unicode_normalization::UNICODE_VERSION
        );
    }
}
