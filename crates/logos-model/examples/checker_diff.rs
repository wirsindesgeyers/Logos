//! Batch checker bridge for the independent Python/Rust differential (Etapa 2.4).
//!
//! Protocol: one line per case with whitespace-separated fields:
//!   `<mode> <vocab_hex> <obligation_hex> <expected_obligation_hash_hex> <model_hex>`
//! Mode must be either "refute" or "witness".
//!
//! Output: exactly "Accepted", "Rejected", or "Unknown" per line.
//! Structural/model/obligation invalid -> "Rejected"
//! Resource exhaustion -> "Unknown"
//!
//! Invocation:
//!   `cargo run --quiet -p logos-model --example checker_diff --`

use std::error::Error;
use std::io::{self, BufRead, Write};

use logos_air::{AirError, Limits, Vocabulary, VocabularyError};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::Hash;
use logos_model::{FiniteModel, ModelChecker, ModelError, ModelLimits};
use logos_obligation::Obligation;

fn hex_byte(chunk: &[u8; 2]) -> Result<u8, ()> {
    let digit = |byte| match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(()),
    };
    Ok((digit(chunk[0])? << 4) | digit(chunk[1])?)
}

fn unhex(text: &str) -> Result<Vec<u8>, ()> {
    if !text.len().is_multiple_of(2) {
        return Err(());
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    for chunk in text.as_bytes().as_chunks::<2>().0 {
        bytes.push(hex_byte(chunk)?);
    }
    Ok(bytes)
}

enum OutcomeLabel {
    Accepted,
    Rejected,
    Unknown,
}

fn process_line(line: &str) -> OutcomeLabel {
    let mut parts = line.split_whitespace();
    let (
        Some(mode),
        Some(vocab_hex),
        Some(obligation_hex),
        Some(expected_hash_hex),
        Some(model_hex),
    ) = (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    )
    else {
        return OutcomeLabel::Rejected;
    };
    if parts.next().is_some() {
        return OutcomeLabel::Rejected;
    }

    if mode != "refute" && mode != "witness" {
        return OutcomeLabel::Rejected;
    }

    let vocab_bytes = match unhex(vocab_hex) {
        Ok(b) => b,
        Err(()) => return OutcomeLabel::Rejected,
    };

    let vocab = match Vocabulary::decode_with_limits(&vocab_bytes, Limits::default()) {
        Ok(v) => v,
        Err(VocabularyError::Air(
            AirError::DepthExceeded { .. } | AirError::NodesExceeded { .. },
        )) => return OutcomeLabel::Unknown,
        Err(_) => return OutcomeLabel::Rejected,
    };

    let checker = match ModelChecker::new(&vocab) {
        Ok(c) => c,
        Err(_) => return OutcomeLabel::Rejected,
    };

    if expected_hash_hex.len() != 64 {
        return OutcomeLabel::Rejected;
    }
    let mut hash_arr = [0u8; 32];
    for (output, chunk) in hash_arr
        .iter_mut()
        .zip(expected_hash_hex.as_bytes().as_chunks::<2>().0)
    {
        *output = match hex_byte(chunk) {
            Ok(byte) => byte,
            Err(()) => return OutcomeLabel::Rejected,
        };
    }
    let expected_hash = Hash::<Obligation>::from_bytes(hash_arr);

    let obligation_bytes = match unhex(obligation_hex) {
        Ok(b) => b,
        Err(()) => return OutcomeLabel::Rejected,
    };

    let model_bytes = match unhex(model_hex) {
        Ok(b) => b,
        Err(()) => return OutcomeLabel::Rejected,
    };

    let model = match FiniteModel::decode_with_limits(&model_bytes, &vocab, ModelLimits::default())
    {
        Ok(m) => m,
        Err(ModelError::Unknown(_)) => return OutcomeLabel::Unknown,
        Err(_) => return OutcomeLabel::Rejected,
    };

    // Shared budget generous enough for all finite cases
    let budget = Budget::new(10_000_000, 10_000_000, 10_000_000, 0);

    let outcome = if mode == "refute" {
        checker.check_refutation(&obligation_bytes, expected_hash, &model, &budget)
    } else {
        checker.check_consistency(&obligation_bytes, expected_hash, &model, &budget)
    };

    match outcome {
        CheckOutcome::Accepted => OutcomeLabel::Accepted,
        CheckOutcome::Rejected(_) => OutcomeLabel::Rejected,
        CheckOutcome::Unknown(_) => OutcomeLabel::Unknown,
    }
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
        let verdict = process_line(trimmed);
        match verdict {
            OutcomeLabel::Accepted => writeln!(stdout, "Accepted")?,
            OutcomeLabel::Rejected => writeln!(stdout, "Rejected")?,
            OutcomeLabel::Unknown => writeln!(stdout, "Unknown")?,
        }
    }
    stdout.flush()?;
    Ok(())
}
