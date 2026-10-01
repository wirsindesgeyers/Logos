//! Decoder estrito: aceita **somente** a codificação canônica (`canon/v1`).

use crate::MAX_DEPTH;
use crate::error::CanonError;
use crate::text::Text;
use crate::value::{Array, Map, Value};

/// Leitor sobre uma fatia de bytes.
#[derive(Debug, Clone)]
pub struct Decoder<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    /// Começa a ler no início de `input`.
    #[must_use]
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    /// Bytes ainda não consumidos.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.input.len() - self.pos
    }

    /// Confirma que a entrada acabou.
    ///
    /// # Errors
    /// [`CanonError::TrailingBytes`] se sobrar algum byte.
    pub fn finish(self) -> Result<(), CanonError> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(CanonError::TrailingBytes)
        }
    }

    /// Lê o próximo item (e tudo que ele contém).
    ///
    /// # Errors
    /// Qualquer violação da codificação canônica; ver [`CanonError`].
    pub fn value(&mut self) -> Result<Value, CanonError> {
        self.value_at(0)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], CanonError> {
        let end = self.pos.checked_add(n).ok_or(CanonError::Eof)?;
        let slice = self.input.get(self.pos..end).ok_or(CanonError::Eof)?;
        self.pos = end;
        Ok(slice)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], CanonError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    /// Lê o argumento do cabeçalho e exige a menor codificação.
    fn argument(&mut self, info: u8) -> Result<u64, CanonError> {
        match info {
            0..=23 => Ok(u64::from(info)),
            24 => {
                let v = u64::from(self.take_array::<1>()?[0]);
                minimal(v, 24)
            }
            25 => {
                let v = u64::from(u16::from_be_bytes(self.take_array()?));
                minimal(v, 0x100)
            }
            26 => {
                let v = u64::from(u32::from_be_bytes(self.take_array()?));
                minimal(v, 0x1_0000)
            }
            27 => {
                let v = u64::from_be_bytes(self.take_array()?);
                minimal(v, 0x1_0000_0000)
            }
            _ => Err(CanonError::ReservedInfo),
        }
    }

    fn value_at(&mut self, depth: usize) -> Result<Value, CanonError> {
        let initial = self.take_array::<1>()?[0];
        let (major, info) = (initial >> 5, initial & 0x1f);
        match major {
            6 => return Err(CanonError::Tag),
            7 => {
                return match info {
                    20 => Ok(Value::Bool(false)),
                    21 => Ok(Value::Bool(true)),
                    22 => Ok(Value::Null),
                    28..=30 => Err(CanonError::ReservedInfo),
                    31 => Err(CanonError::Indefinite),
                    _ => Err(CanonError::UnsupportedSimple),
                };
            }
            _ => {}
        }
        match info {
            28..=30 => return Err(CanonError::ReservedInfo),
            31 if major >= 2 => return Err(CanonError::Indefinite),
            31 => return Err(CanonError::ReservedInfo),
            _ => {}
        }
        let arg = self.argument(info)?;
        match major {
            0 => Ok(Value::Unsigned(arg)),
            1 => Ok(Value::Negative(arg)),
            2 => Ok(Value::Bytes(self.take_len(arg)?.to_vec())),
            3 => {
                let bytes = self.take_len(arg)?;
                let s = core::str::from_utf8(bytes).map_err(|_| CanonError::InvalidUtf8)?;
                Ok(Value::Text(Text::new(s)?))
            }
            4 => {
                self.enter(depth)?;
                let mut items = Vec::new();
                for _ in 0..arg {
                    items.push(self.value_at(depth + 1)?);
                }
                Ok(Value::Array(Array::new(items)?))
            }
            _ => {
                self.enter(depth)?;
                let mut entries: Vec<(Value, Value)> = Vec::new();
                let input: &'a [u8] = self.input;
                let mut previous: Option<&'a [u8]> = None;
                for _ in 0..arg {
                    let start = self.pos;
                    let key = self.value_at(depth + 1)?;
                    let key_bytes = input.get(start..self.pos).ok_or(CanonError::Eof)?;
                    if let Some(prev) = previous {
                        match prev.cmp(key_bytes) {
                            core::cmp::Ordering::Less => {}
                            core::cmp::Ordering::Equal => return Err(CanonError::DuplicateKey),
                            core::cmp::Ordering::Greater => return Err(CanonError::MapKeyOrder),
                        }
                    }
                    previous = Some(key_bytes);
                    let value = self.value_at(depth + 1)?;
                    entries.push((key, value));
                }
                Ok(Value::Map(Map::from_sorted(entries)?))
            }
        }
    }

    /// Entrar em um array/mapa que fica no nível `depth + 1`.
    fn enter(&self, depth: usize) -> Result<(), CanonError> {
        if depth + 1 > MAX_DEPTH {
            Err(CanonError::DepthExceeded)
        } else {
            Ok(())
        }
    }

    /// Consome `len` bytes; falha com `Eof` se não houver tantos (sem alocar antes de checar).
    fn take_len(&mut self, len: u64) -> Result<&'a [u8], CanonError> {
        let len = usize::try_from(len).map_err(|_| CanonError::Eof)?;
        self.take(len)
    }
}

/// `v` precisa ser pelo menos `min` (senão caberia numa forma menor).
fn minimal(v: u64, min: u64) -> Result<u64, CanonError> {
    if v < min {
        Err(CanonError::NonMinimalHead)
    } else {
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(hex: &str) -> Result<Value, CanonError> {
        let bytes: Vec<u8> = (0..hex.len() / 2)
            .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
            .collect();
        let mut d = Decoder::new(&bytes);
        let v = d.value()?;
        d.finish()?;
        Ok(v)
    }

    #[test]
    fn accepts_canonical_scalars() {
        assert_eq!(dec("00"), Ok(Value::Unsigned(0)));
        assert_eq!(dec("1818"), Ok(Value::Unsigned(24)));
        assert_eq!(
            dec("3903e7").map(|v| v.to_string()),
            Ok("-1000".to_string())
        );
        assert_eq!(dec("f6"), Ok(Value::Null));
    }

    #[test]
    fn rejects_non_minimal_heads() {
        assert_eq!(dec("1817"), Err(CanonError::NonMinimalHead));
        assert_eq!(dec("1900ff"), Err(CanonError::NonMinimalHead));
        assert_eq!(dec("1a0000ffff"), Err(CanonError::NonMinimalHead));
        assert_eq!(dec("1b00000000ffffffff"), Err(CanonError::NonMinimalHead));
        assert_eq!(dec("5800"), Err(CanonError::NonMinimalHead));
    }

    #[test]
    fn rejects_forbidden_features() {
        assert_eq!(
            dec("c074323031332d30332d32315432303a30343a30305a"),
            Err(CanonError::Tag)
        );
        assert_eq!(dec("f97e00"), Err(CanonError::UnsupportedSimple)); // float16 NaN
        assert_eq!(
            dec("fb3ff199999999999a"),
            Err(CanonError::UnsupportedSimple)
        ); // float64
        assert_eq!(dec("f7"), Err(CanonError::UnsupportedSimple)); // undefined
        assert_eq!(dec("5fff"), Err(CanonError::Indefinite));
        assert_eq!(dec("9fff"), Err(CanonError::Indefinite));
        assert_eq!(dec("ff"), Err(CanonError::Indefinite));
        assert_eq!(dec("1c"), Err(CanonError::ReservedInfo));
        assert_eq!(dec("1f"), Err(CanonError::ReservedInfo));
    }

    #[test]
    fn rejects_truncated_and_trailing() {
        assert_eq!(dec(""), Err(CanonError::Eof));
        assert_eq!(dec("19"), Err(CanonError::Eof));
        assert_eq!(dec("4401"), Err(CanonError::Eof));
        assert_eq!(dec("0000"), Err(CanonError::TrailingBytes));
    }

    #[test]
    fn huge_length_does_not_allocate() {
        assert_eq!(dec("5bffffffffffffffff"), Err(CanonError::Eof));
        assert_eq!(dec("9bffffffffffffffff"), Err(CanonError::Eof));
    }

    #[test]
    fn map_order_and_duplicates() {
        assert_eq!(dec("a202000100"), Err(CanonError::MapKeyOrder)); // chaves 2, 1
        assert_eq!(dec("a201000100"), Err(CanonError::DuplicateKey)); // chaves 1, 1
        assert_eq!(dec("a26100006000"), Err(CanonError::MapKeyOrder)); // " " depois de ""
        assert!(dec("a201000200").is_ok());
    }

    #[test]
    fn text_validity() {
        assert_eq!(dec("62c328"), Err(CanonError::InvalidUtf8));
        assert_eq!(dec("63656cc2"), Err(CanonError::InvalidUtf8)); // truncado no meio de um code point
        assert_eq!(dec("6365cc81"), Err(CanonError::NotNfc)); // "e" + U+0301
        assert_eq!(dec("62cdb8"), Err(CanonError::UnassignedCodePoint)); // U+0378
    }

    #[test]
    fn depth_limit_is_enforced() {
        let ok = "81".repeat(MAX_DEPTH) + "f6";
        assert!(dec(&ok).is_ok());
        let too_deep = "81".repeat(MAX_DEPTH + 1) + "f6";
        assert_eq!(dec(&too_deep), Err(CanonError::DepthExceeded));
    }
}
