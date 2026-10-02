//! Codificação canônica dos tipos do core conforme `core/v1`.
//!
//! Cada tipo é codificado como um array CBOR de comprimento fixo com o inteiro
//! de versão `1` na primeira posição.

use crate::budget::Budget;
use crate::outcome::{CheckOutcome, Exhausted, Reason};
use crate::verdict::Verdict;
use logos_canon::{Array, Canon, CanonDecode, CanonError, Text, Value};

/// Versão do formato canônico `core/v1`.
const CORE_FORMAT_VERSION: u64 = 1;

fn check_version(value: &Value) -> Result<(), CanonError> {
    match value {
        Value::Unsigned(CORE_FORMAT_VERSION) => Ok(()),
        _ => Err(CanonError::WrongType),
    }
}

fn decode_u64(value: &Value) -> Result<u64, CanonError> {
    u64::from_value(value)
}

impl Canon for Budget {
    fn to_value(&self) -> Result<Value, CanonError> {
        let items = vec![
            Value::Unsigned(CORE_FORMAT_VERSION),
            Value::Unsigned(self.steps),
            Value::Unsigned(self.mem_bytes),
            Value::Unsigned(self.size_bytes),
            Value::Unsigned(self.wall_ms),
        ];
        Ok(Value::Array(Array::new(items)?))
    }
}

impl CanonDecode for Budget {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let Value::Array(arr) = value else {
            return Err(CanonError::WrongType);
        };
        let items = arr.items();
        if items.len() != 5 {
            return Err(CanonError::WrongType);
        }
        check_version(&items[0])?;
        let steps = decode_u64(&items[1])?;
        let mem_bytes = decode_u64(&items[2])?;
        let size_bytes = decode_u64(&items[3])?;
        let wall_ms = decode_u64(&items[4])?;
        Ok(Self {
            steps,
            mem_bytes,
            size_bytes,
            wall_ms,
        })
    }
}

impl Canon for Exhausted {
    fn to_value(&self) -> Result<Value, CanonError> {
        let tag = match self {
            Self::Time => 0,
            Self::Steps => 1,
            Self::Memory => 2,
            Self::Size => 3,
        };
        let items = vec![Value::Unsigned(CORE_FORMAT_VERSION), Value::Unsigned(tag)];
        Ok(Value::Array(Array::new(items)?))
    }
}

impl CanonDecode for Exhausted {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let Value::Array(arr) = value else {
            return Err(CanonError::WrongType);
        };
        let items = arr.items();
        if items.len() != 2 {
            return Err(CanonError::WrongType);
        }
        check_version(&items[0])?;
        match &items[1] {
            Value::Unsigned(0) => Ok(Self::Time),
            Value::Unsigned(1) => Ok(Self::Steps),
            Value::Unsigned(2) => Ok(Self::Memory),
            Value::Unsigned(3) => Ok(Self::Size),
            _ => Err(CanonError::WrongType),
        }
    }
}

impl Canon for Reason {
    fn to_value(&self) -> Result<Value, CanonError> {
        let items = vec![Value::Unsigned(CORE_FORMAT_VERSION), self.text.to_value()?];
        Ok(Value::Array(Array::new(items)?))
    }
}

impl CanonDecode for Reason {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let Value::Array(arr) = value else {
            return Err(CanonError::WrongType);
        };
        let items = arr.items();
        if items.len() != 2 {
            return Err(CanonError::WrongType);
        }
        check_version(&items[0])?;
        let text = Text::from_value(&items[1])?;
        Ok(Self::new(text))
    }
}

impl Canon for CheckOutcome {
    fn to_value(&self) -> Result<Value, CanonError> {
        let items = match self {
            Self::Accepted => vec![Value::Unsigned(CORE_FORMAT_VERSION), Value::Unsigned(0)],
            Self::Rejected(reason) => vec![
                Value::Unsigned(CORE_FORMAT_VERSION),
                Value::Unsigned(1),
                reason.to_value()?,
            ],
            Self::Unknown(exhausted) => vec![
                Value::Unsigned(CORE_FORMAT_VERSION),
                Value::Unsigned(2),
                exhausted.to_value()?,
            ],
        };
        Ok(Value::Array(Array::new(items)?))
    }
}

impl CanonDecode for CheckOutcome {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let Value::Array(arr) = value else {
            return Err(CanonError::WrongType);
        };
        let items = arr.items();
        if items.len() < 2 {
            return Err(CanonError::WrongType);
        }
        check_version(&items[0])?;
        match &items[1] {
            Value::Unsigned(0) => {
                if items.len() != 2 {
                    return Err(CanonError::WrongType);
                }
                Ok(Self::Accepted)
            }
            Value::Unsigned(1) => {
                if items.len() != 3 {
                    return Err(CanonError::WrongType);
                }
                let reason = Reason::from_value(&items[2])?;
                Ok(Self::Rejected(reason))
            }
            Value::Unsigned(2) => {
                if items.len() != 3 {
                    return Err(CanonError::WrongType);
                }
                let exhausted = Exhausted::from_value(&items[2])?;
                Ok(Self::Unknown(exhausted))
            }
            _ => Err(CanonError::WrongType),
        }
    }
}

impl Canon for Verdict {
    fn to_value(&self) -> Result<Value, CanonError> {
        let tag = match self {
            Self::Verified => 0,
            Self::Refuted => 1,
            Self::Unknown => 2,
        };
        let items = vec![Value::Unsigned(CORE_FORMAT_VERSION), Value::Unsigned(tag)];
        Ok(Value::Array(Array::new(items)?))
    }
}

impl CanonDecode for Verdict {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let Value::Array(arr) = value else {
            return Err(CanonError::WrongType);
        };
        let items = arr.items();
        if items.len() != 2 {
            return Err(CanonError::WrongType);
        }
        check_version(&items[0])?;
        match &items[1] {
            Value::Unsigned(0) => Ok(Self::Verified),
            Value::Unsigned(1) => Ok(Self::Refuted),
            Value::Unsigned(2) => Ok(Self::Unknown),
            _ => Err(CanonError::WrongType),
        }
    }
}
