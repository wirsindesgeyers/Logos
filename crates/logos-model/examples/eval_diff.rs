//! Batch evaluation bridge for the independent Python/Rust differential (Etapa 2.2).
//!
//! Input: one line per case with whitespace-separated fields:
//!   `<vocab_hex> <model_hex> <formula_hex> [<steps>]`
//! Output:
//!   "true", "false", or "error: <details>"

use std::error::Error;
use std::io::{self, BufRead, Write};

use logos_air::{Formula, Limits, Vocabulary};
use logos_core::Budget;
use logos_model::{FiniteModel, ModelLimits, eval};

fn unhex(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err("odd hex string length".to_string());
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    for chunk in text.as_bytes().as_chunks::<2>().0 {
        let hi = match chunk[0] {
            b'0'..=b'9' => chunk[0] - b'0',
            b'a'..=b'f' => chunk[0] - b'a' + 10,
            b'A'..=b'F' => chunk[0] - b'A' + 10,
            _ => return Err("invalid hex digit".to_string()),
        };
        let lo = match chunk[1] {
            b'0'..=b'9' => chunk[1] - b'0',
            b'a'..=b'f' => chunk[1] - b'a' + 10,
            b'A'..=b'F' => chunk[1] - b'A' + 10,
            _ => return Err("invalid hex digit".to_string()),
        };
        bytes.push((hi << 4) | lo);
    }
    Ok(bytes)
}

fn evaluate_case(line: &str) -> Result<bool, String> {
    let mut parts = line.split_whitespace();
    let vocab_hex = parts
        .next()
        .ok_or_else(|| "missing vocabulary field".to_string())?;
    let model_hex = parts
        .next()
        .ok_or_else(|| "missing model field".to_string())?;
    let formula_hex = parts
        .next()
        .ok_or_else(|| "missing formula field".to_string())?;
    let steps = if let Some(steps_str) = parts.next() {
        steps_str
            .parse::<u64>()
            .map_err(|e| format!("invalid steps: {e}"))?
    } else {
        100_000_000
    };

    let vocab_bytes = unhex(vocab_hex)?;
    let vocab = Vocabulary::decode_with_limits(&vocab_bytes, Limits::default())
        .map_err(|e| format!("vocabulary decode: {e}"))?;

    let model_bytes = unhex(model_hex)?;
    let model = FiniteModel::decode_with_limits(&model_bytes, &vocab, ModelLimits::default())
        .map_err(|e| format!("model decode: {e}"))?;

    let formula_bytes = unhex(formula_hex)?;
    let formula = Formula::decode_with_limits(&formula_bytes, Limits::default())
        .map_err(|e| format!("formula decode: {e}"))?;

    let budget = Budget::new(steps, u64::MAX, u64::MAX, 0);
    eval(&model, &vocab, &formula, &budget).map_err(|e| format!("eval: {e}"))
}

fn main() -> Result<(), Box<dyn Error>> {
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();

    for line_res in stdin.lock().lines() {
        let line = line_res?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match evaluate_case(trimmed) {
            Ok(true) => writeln!(stdout, "true")?,
            Ok(false) => writeln!(stdout, "false")?,
            Err(e) => writeln!(stdout, "error: {e}")?,
        }
    }
    stdout.flush()?;
    Ok(())
}
