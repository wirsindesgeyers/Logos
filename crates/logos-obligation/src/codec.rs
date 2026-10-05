//! Codificação canônica de `obligation/v1` (somente saída).
//!
//! Contrato normativo: `spec/obligation.md` §2; ADR 0012.

use logos_canon::{Array, Canon, CanonError, Text, Value};

use crate::types::Obligation;

impl Canon for Obligation {
    fn to_value(&self) -> Result<Value, CanonError> {
        let mut premise_items = Vec::with_capacity(self.premises.len());
        for (reference, formula) in &self.premises {
            let pair = Array::new(vec![
                Value::Bytes(reference.event_hash().to_vec()),
                formula.to_value()?,
            ])?;
            premise_items.push(Value::Array(pair));
        }
        let premises_array = Array::new(premise_items)?;

        let envelope = Array::new(vec![
            Value::Unsigned(1),
            Value::Unsigned(0),
            Value::Text(Text::new(self.logic.as_str())?),
            Value::Unsigned(self.encoder_version),
            Value::Bytes(self.vocab.as_bytes().to_vec()),
            Value::Array(premises_array),
            self.goal.to_value()?,
        ])?;

        Ok(Value::Array(envelope))
    }
}
