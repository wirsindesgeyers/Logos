//! Modelo de dados da codificação canônica e o encoder.
//!
//! [`Value`] só admite valores que têm codificação canônica: o encoder é **total** (nunca falha)
//! porque toda restrição (texto NFC, ordem e unicidade das chaves, profundidade) é verificada na
//! construção.

use crate::MAX_DEPTH;
use crate::error::CanonError;
use crate::text::Text;
use core::fmt;

/// Um valor do subconjunto canônico de CBOR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// `null` (`0xf6`).
    Null,
    /// `false` (`0xf4`) ou `true` (`0xf5`).
    Bool(bool),
    /// Inteiro `n ≥ 0` (tipo maior 0).
    Unsigned(u64),
    /// Inteiro `-1 - n` (tipo maior 1); `Negative(0)` é `-1`.
    Negative(u64),
    /// String de bytes.
    Bytes(Vec<u8>),
    /// String de texto (UTF-8, NFC, atribuída).
    Text(Text),
    /// Array.
    Array(Array),
    /// Mapa com chaves ordenadas e únicas.
    Map(Map),
}

/// Array canônico. Só se constrói por [`Array::new`] (verifica a profundidade).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array {
    items: Vec<Value>,
    depth: usize,
}

/// Mapa canônico: entradas ordenadas pelos bytes codificados da chave, sem chaves repetidas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    entries: Vec<(Value, Value)>,
    depth: usize,
}

impl Value {
    /// Constrói um inteiro; falha fora de `[-2^64, 2^64-1]`.
    ///
    /// # Errors
    /// [`CanonError::IntOutOfRange`].
    pub fn int(n: i128) -> Result<Self, CanonError> {
        if n >= 0 {
            u64::try_from(n).map(Self::Unsigned)
        } else {
            u64::try_from(-1 - n).map(Self::Negative)
        }
        .map_err(|_| CanonError::IntOutOfRange)
    }

    /// O valor como inteiro, se for `Unsigned` ou `Negative`.
    #[must_use]
    pub fn as_int(&self) -> Option<i128> {
        match self {
            Self::Unsigned(n) => Some(i128::from(*n)),
            Self::Negative(n) => Some(-1 - i128::from(*n)),
            _ => None,
        }
    }

    /// Profundidade de aninhamento de arrays/mapas (escalares têm 0).
    #[must_use]
    pub fn depth(&self) -> usize {
        match self {
            Self::Array(a) => a.depth,
            Self::Map(m) => m.depth,
            _ => 0,
        }
    }

    /// Anexa a codificação canônica a `out`.
    pub fn encode_into(&self, out: &mut Vec<u8>) {
        match self {
            Self::Null => out.push(0xf6),
            Self::Bool(false) => out.push(0xf4),
            Self::Bool(true) => out.push(0xf5),
            Self::Unsigned(n) => write_head(out, 0, *n),
            Self::Negative(n) => write_head(out, 1, *n),
            Self::Bytes(b) => {
                write_head(out, 2, b.len() as u64);
                out.extend_from_slice(b);
            }
            Self::Text(t) => {
                write_head(out, 3, t.as_str().len() as u64);
                out.extend_from_slice(t.as_str().as_bytes());
            }
            Self::Array(a) => {
                write_head(out, 4, a.items.len() as u64);
                for item in &a.items {
                    item.encode_into(out);
                }
            }
            Self::Map(m) => {
                write_head(out, 5, m.entries.len() as u64);
                for (k, v) in &m.entries {
                    k.encode_into(out);
                    v.encode_into(out);
                }
            }
        }
    }

    /// A codificação canônica como vetor novo.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode_into(&mut out);
        out
    }
}

/// Escreve o cabeçalho (tipo maior + argumento) na menor forma possível.
fn write_head(out: &mut Vec<u8>, major: u8, arg: u64) {
    let m = major << 5;
    if arg < 24 {
        out.push(m | arg as u8);
    } else if let Ok(v) = u8::try_from(arg) {
        out.extend_from_slice(&[m | 24, v]);
    } else if let Ok(v) = u16::try_from(arg) {
        out.push(m | 25);
        out.extend_from_slice(&v.to_be_bytes());
    } else if let Ok(v) = u32::try_from(arg) {
        out.push(m | 26);
        out.extend_from_slice(&v.to_be_bytes());
    } else {
        out.push(m | 27);
        out.extend_from_slice(&arg.to_be_bytes());
    }
}

impl Array {
    /// Constrói um array.
    ///
    /// # Errors
    /// [`CanonError::DepthExceeded`] se o aninhamento passar de [`MAX_DEPTH`].
    pub fn new(items: Vec<Value>) -> Result<Self, CanonError> {
        let depth = 1 + items.iter().map(Value::depth).max().unwrap_or(0);
        if depth > MAX_DEPTH {
            return Err(CanonError::DepthExceeded);
        }
        Ok(Self { items, depth })
    }

    /// Os elementos.
    #[must_use]
    pub fn items(&self) -> &[Value] {
        &self.items
    }
}

impl Map {
    /// Constrói um mapa a partir de entradas em qualquer ordem: ordena pelos bytes codificados
    /// das chaves (RFC 8949 §4.2.1).
    ///
    /// # Errors
    /// [`CanonError::DuplicateKey`] se duas chaves tiverem a mesma codificação;
    /// [`CanonError::DepthExceeded`] se o aninhamento passar de [`MAX_DEPTH`].
    pub fn new(entries: Vec<(Value, Value)>) -> Result<Self, CanonError> {
        let mut keyed: Vec<(Vec<u8>, Value, Value)> = entries
            .into_iter()
            .map(|(k, v)| (k.to_bytes(), k, v))
            .collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        if keyed.windows(2).any(|w| w[0].0 == w[1].0) {
            return Err(CanonError::DuplicateKey);
        }
        let entries: Vec<(Value, Value)> = keyed.into_iter().map(|(_, k, v)| (k, v)).collect();
        Self::from_sorted(entries)
    }

    /// Constrói a partir de entradas **já** ordenadas e sem duplicatas (uso do decoder).
    pub(crate) fn from_sorted(entries: Vec<(Value, Value)>) -> Result<Self, CanonError> {
        let depth = 1 + entries
            .iter()
            .map(|(k, v)| k.depth().max(v.depth()))
            .max()
            .unwrap_or(0);
        if depth > MAX_DEPTH {
            return Err(CanonError::DepthExceeded);
        }
        Ok(Self { entries, depth })
    }

    /// As entradas, na ordem canônica.
    #[must_use]
    pub fn entries(&self) -> &[(Value, Value)] {
        &self.entries
    }

    /// Valor associado à chave de texto `key`, se existir.
    #[must_use]
    pub fn get_text(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find_map(|(k, v)| match k {
            Value::Text(t) if t.as_str() == key => Some(v),
            _ => None,
        })
    }
}

/// Renderização textual determinística, idêntica na implementação Python
/// (`py/logos_client/canon.py`); usada pelos testes diferenciais e vetores.
///
/// `null`, `true`, `false`, inteiros em decimal, `h'<hex>'`, `t"<texto>"` (só ASCII imprimível
/// literal; `"` e `\` escapados; o resto como `\u{hex}` minúsculo), `[a,b]` e `{k:v,k:v}`.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Unsigned(n) => write!(f, "{n}"),
            Self::Negative(n) => write!(f, "{}", -1 - i128::from(*n)),
            Self::Bytes(b) => {
                f.write_str("h'")?;
                for byte in b {
                    write!(f, "{byte:02x}")?;
                }
                f.write_str("'")
            }
            Self::Text(t) => {
                f.write_str("t\"")?;
                for c in t.as_str().chars() {
                    match c {
                        '"' => f.write_str("\\\"")?,
                        '\\' => f.write_str("\\\\")?,
                        ' '..='~' => write!(f, "{c}")?,
                        _ => write!(f, "\\u{{{:x}}}", u32::from(c))?,
                    }
                }
                f.write_str("\"")
            }
            Self::Array(a) => {
                f.write_str("[")?;
                for (i, item) in a.items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Self::Map(m) => {
                f.write_str("{")?;
                for (i, (k, v)) in m.entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{k}:{v}")?;
                }
                f.write_str("}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(v: &Value) -> String {
        v.to_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn rfc8949_appendix_a_integers() {
        let cases: [(i128, &str); 12] = [
            (0, "00"),
            (1, "01"),
            (10, "0a"),
            (23, "17"),
            (24, "1818"),
            (100, "1864"),
            (1000, "1903e8"),
            (1_000_000, "1a000f4240"),
            (1_000_000_000_000, "1b000000e8d4a51000"),
            (-1, "20"),
            (-10, "29"),
            (-1000, "3903e7"),
        ];
        for (n, hex) in cases {
            assert_eq!(enc(&Value::int(n).unwrap()), hex, "{n}");
        }
    }

    #[test]
    fn integer_range_edges() {
        assert_eq!(
            enc(&Value::int(i128::from(u64::MAX)).unwrap()),
            "1bffffffffffffffff"
        );
        assert_eq!(
            enc(&Value::int(-1 - i128::from(u64::MAX)).unwrap()),
            "3bffffffffffffffff"
        );
        assert_eq!(
            Value::int(i128::from(u64::MAX) + 1),
            Err(CanonError::IntOutOfRange)
        );
        assert_eq!(
            Value::int(-2 - i128::from(u64::MAX)),
            Err(CanonError::IntOutOfRange)
        );
    }

    #[test]
    fn simple_values_and_strings() {
        assert_eq!(enc(&Value::Bool(false)), "f4");
        assert_eq!(enc(&Value::Bool(true)), "f5");
        assert_eq!(enc(&Value::Null), "f6");
        assert_eq!(enc(&Value::Bytes(vec![1, 2, 3, 4])), "4401020304");
        assert_eq!(enc(&Value::Text(Text::new("IETF").unwrap())), "6449455446");
    }

    #[test]
    fn map_keys_are_sorted_by_encoded_bytes() {
        let t = |s: &str| Value::Text(Text::new(s).unwrap());
        // "b" = 61 62; "aa" = 62 61 61; 10 = 0a; -1 = 20; false = f4
        let m = Map::new(vec![
            (Value::Bool(false), Value::Null),
            (t("aa"), Value::Null),
            (t("b"), Value::Null),
            (Value::int(-1).unwrap(), Value::Null),
            (Value::Unsigned(10), Value::Null),
        ])
        .unwrap();
        assert_eq!(enc(&Value::Map(m)), "a50af620f66162f6626161f6f4f6");
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        let r = Map::new(vec![
            (Value::Unsigned(1), Value::Null),
            (Value::Unsigned(1), Value::Null),
        ]);
        assert_eq!(r, Err(CanonError::DuplicateKey));
    }

    #[test]
    fn depth_limit() {
        let mut v = Value::Null;
        for _ in 0..MAX_DEPTH {
            v = Value::Array(Array::new(vec![v]).unwrap());
        }
        assert_eq!(v.depth(), MAX_DEPTH);
        assert_eq!(Array::new(vec![v]), Err(CanonError::DepthExceeded));
    }

    #[test]
    fn display_is_deterministic() {
        let t = Value::Text(Text::new("a\"\\\u{e9}").unwrap());
        assert_eq!(t.to_string(), "t\"a\\\"\\\\\\u{e9}\"");
        let a = Value::Array(
            Array::new(vec![
                Value::Null,
                Value::Bool(true),
                Value::int(-5).unwrap(),
            ])
            .unwrap(),
        );
        assert_eq!(a.to_string(), "[null,true,-5]");
        assert_eq!(Value::Bytes(vec![0xde, 0xad]).to_string(), "h'dead'");
    }
}
