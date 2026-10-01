//! Traits `Canon` / `CanonDecode` e implementações para os tipos básicos.
//!
//! Um tipo do LOGOS se descreve como [`Value`] (`to_value` / `from_value`); a serialização e a
//! decodificação estrita são únicas e ficam neste crate, então ordem de chaves, tamanho mínimo
//! dos inteiros e NFC não dependem de quem escreve o tipo.

use crate::decode::Decoder;
use crate::error::CanonError;
use crate::text::Text;
use crate::value::{Array, Value};

/// Tipos com codificação canônica.
pub trait Canon {
    /// O objeto como valor do modelo canônico.
    ///
    /// # Errors
    /// [`CanonError::DepthExceeded`] se o objeto aninha mais que [`MAX_DEPTH`](crate::MAX_DEPTH),
    /// ou [`CanonError::DuplicateKey`] se um mapa tiver chaves repetidas: nesses casos o objeto
    /// não tem codificação canônica e **não** é codificado como outra coisa.
    fn to_value(&self) -> Result<Value, CanonError>;

    /// Anexa os bytes canônicos a `out`.
    ///
    /// # Errors
    /// Os de [`Canon::to_value`]; `out` não é modificado em caso de erro.
    fn encode(&self, out: &mut Vec<u8>) -> Result<(), CanonError> {
        self.to_value()?.encode_into(out);
        Ok(())
    }
}

/// Tipos que se lêem a partir da codificação canônica estrita.
pub trait CanonDecode: Sized {
    /// Reconstrói o objeto a partir de um valor já validado.
    ///
    /// # Errors
    /// [`CanonError::WrongType`] (ou [`CanonError::IntOutOfRange`]) se o valor não tem a forma esperada.
    fn from_value(value: &Value) -> Result<Self, CanonError>;

    /// Lê um objeto do decoder (que rejeita qualquer codificação não canônica).
    ///
    /// # Errors
    /// Violações da codificação canônica ou da forma esperada.
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, CanonError> {
        Self::from_value(&decoder.value()?)
    }
}

/// Codifica `x` em bytes canônicos.
///
/// # Errors
/// Os de [`Canon::to_value`].
pub fn to_canon<T: Canon + ?Sized>(x: &T) -> Result<Vec<u8>, CanonError> {
    let mut out = Vec::new();
    x.encode(&mut out)?;
    Ok(out)
}

/// Decodifica `bytes` inteiros como um `T`; recusa bytes sobrando.
///
/// # Errors
/// Qualquer violação da codificação canônica, [`CanonError::TrailingBytes`] ou erro de forma.
pub fn from_canon<T: CanonDecode>(bytes: &[u8]) -> Result<T, CanonError> {
    let mut decoder = Decoder::new(bytes);
    let x = T::decode(&mut decoder)?;
    decoder.finish()?;
    Ok(x)
}

/// String de bytes (CBOR tipo 2). `Vec<u8>` é um array de inteiros; para bytes use `Bytes`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes(pub Vec<u8>);

impl Canon for Value {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(self.clone())
    }
}

impl CanonDecode for Value {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        Ok(value.clone())
    }
}

impl Canon for bool {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(Value::Bool(*self))
    }
}

impl CanonDecode for bool {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match value {
            Value::Bool(b) => Ok(*b),
            _ => Err(CanonError::WrongType),
        }
    }
}

macro_rules! impl_int {
    ($($t:ty),*) => {$(
        impl Canon for $t {
            fn to_value(&self) -> Result<Value, CanonError> {
                Value::int(i128::from(*self))
            }
        }
        impl CanonDecode for $t {
            fn from_value(value: &Value) -> Result<Self, CanonError> {
                let n = value.as_int().ok_or(CanonError::WrongType)?;
                <$t>::try_from(n).map_err(|_| CanonError::IntOutOfRange)
            }
        }
    )*};
}
impl_int!(u8, u16, u32, u64, i8, i16, i32, i64);

impl Canon for Text {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(Value::Text(self.clone()))
    }
}

impl CanonDecode for Text {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match value {
            Value::Text(t) => Ok(t.clone()),
            _ => Err(CanonError::WrongType),
        }
    }
}

impl Canon for Bytes {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(Value::Bytes(self.0.clone()))
    }
}

impl CanonDecode for Bytes {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match value {
            Value::Bytes(b) => Ok(Self(b.clone())),
            _ => Err(CanonError::WrongType),
        }
    }
}

impl<const N: usize> Canon for [u8; N] {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(Value::Bytes(self.to_vec()))
    }
}

impl<const N: usize> CanonDecode for [u8; N] {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match value {
            Value::Bytes(b) => Self::try_from(b.as_slice()).map_err(|_| CanonError::WrongType),
            _ => Err(CanonError::WrongType),
        }
    }
}

/// `Option<T>` usa presença explícita: `None` é o array vazio e `Some(x)` é o array `[x]`.
///
/// Não se usa `null` para `None` porque `Some(Value::Null)` e `Some(None)` colidiriam com ele:
/// a codificação precisa ser injetiva (hashes e assinaturas dependem disso).
impl<T: Canon> Canon for Option<T> {
    fn to_value(&self) -> Result<Value, CanonError> {
        let items = match self {
            None => Vec::new(),
            Some(x) => vec![x.to_value()?],
        };
        Ok(Value::Array(Array::new(items)?))
    }
}

impl<T: CanonDecode> CanonDecode for Option<T> {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match value {
            Value::Array(a) => match a.items() {
                [] => Ok(None),
                [x] => T::from_value(x).map(Some),
                _ => Err(CanonError::WrongType),
            },
            _ => Err(CanonError::WrongType),
        }
    }
}

impl<T: Canon> Canon for Vec<T> {
    fn to_value(&self) -> Result<Value, CanonError> {
        let items = self
            .iter()
            .map(Canon::to_value)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Value::Array(Array::new(items)?))
    }
}

impl<T: CanonDecode> CanonDecode for Vec<T> {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match value {
            Value::Array(a) => a.items().iter().map(T::from_value).collect(),
            _ => Err(CanonError::WrongType),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc<T: Canon + ?Sized>(x: &T) -> Vec<u8> {
        to_canon(x).unwrap()
    }

    #[test]
    fn typed_roundtrip() {
        for n in [0u64, 1, 23, 24, 255, 256, 65535, 65536, u64::MAX] {
            assert_eq!(from_canon::<u64>(&enc(&n)), Ok(n));
        }
        for n in [i64::MIN, -1, 0, i64::MAX] {
            assert_eq!(from_canon::<i64>(&enc(&n)), Ok(n));
        }
        assert_eq!(
            from_canon::<Option<bool>>(&enc(&Some(true))),
            Ok(Some(true))
        );
        assert_eq!(from_canon::<Option<bool>>(&enc(&None::<bool>)), Ok(None));
        assert_eq!(
            from_canon::<[u8; 4]>(&enc(&[1u8, 2, 3, 4])),
            Ok([1, 2, 3, 4])
        );
        assert_eq!(from_canon::<Vec<u8>>(&enc(&vec![1u8, 2])), Ok(vec![1, 2]));
    }

    #[test]
    fn integer_target_range_is_checked() {
        assert_eq!(
            from_canon::<u8>(&enc(&256u64)),
            Err(CanonError::IntOutOfRange)
        );
        assert_eq!(
            from_canon::<u64>(&enc(&-1i64)),
            Err(CanonError::IntOutOfRange)
        );
        assert_eq!(from_canon::<bool>(&enc(&1u64)), Err(CanonError::WrongType));
    }

    #[test]
    fn from_canon_rejects_trailing_bytes() {
        let mut b = enc(&1u64);
        b.push(0);
        assert_eq!(from_canon::<u64>(&b), Err(CanonError::TrailingBytes));
    }

    #[test]
    fn wrong_length_array_is_wrong_type() {
        assert_eq!(
            from_canon::<[u8; 4]>(&enc(&Bytes(vec![1, 2, 3]))),
            Err(CanonError::WrongType)
        );
    }

    #[test]
    fn option_encoding_is_injective() {
        // None, Some(Null) e Some(None) nunca compartilham bytes e voltam idênticos.
        let none = enc(&None::<Value>);
        let some_null = enc(&Some(Value::Null));
        assert_ne!(none, some_null);
        assert_eq!(from_canon::<Option<Value>>(&none), Ok(None));
        assert_eq!(
            from_canon::<Option<Value>>(&some_null),
            Ok(Some(Value::Null))
        );

        let all: [Option<Option<bool>>; 4] =
            [None, Some(None), Some(Some(false)), Some(Some(true))];
        let encoded: Vec<Vec<u8>> = all.iter().map(enc).collect();
        for (i, x) in all.iter().enumerate() {
            assert_eq!(from_canon::<Option<Option<bool>>>(&encoded[i]), Ok(*x));
            for other in &encoded[i + 1..] {
                assert_ne!(&encoded[i], other);
            }
        }
    }

    #[test]
    fn option_rejects_longer_arrays() {
        assert_eq!(
            from_canon::<Option<bool>>(&enc(&vec![true, false])),
            Err(CanonError::WrongType)
        );
        assert_eq!(
            from_canon::<Option<bool>>(&enc(&true)),
            Err(CanonError::WrongType)
        );
    }

    #[test]
    fn too_deep_object_is_an_error_not_a_silent_null() {
        let mut v: Vec<Value> = Vec::new();
        let mut value = Value::Null;
        for _ in 0..crate::MAX_DEPTH {
            value = Value::Array(Array::new(vec![value]).unwrap());
        }
        v.push(value);
        assert_eq!(to_canon(&v), Err(CanonError::DepthExceeded));
    }
}
