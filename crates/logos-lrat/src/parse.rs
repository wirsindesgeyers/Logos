//! Parsers ASCII emprestados; nunca normalizam os bytes de entrada.

use logos_core::Exhausted;
use logos_ground::CnfLimits;

use crate::checker::Clause;
use crate::resources::{Failure, Resources, Result};

pub(crate) fn natural(bytes: &[u8]) -> Result<u64> {
    if bytes.is_empty() || (bytes.len() > 1 && bytes[0] == b'0') {
        return Err(Failure::Invalid);
    }
    let mut value = 0_u64;
    for &byte in bytes {
        if !byte.is_ascii_digit() {
            return Err(Failure::Invalid);
        }
        value = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(u64::from(byte - b'0')))
            .ok_or(Failure::Invalid)?;
    }
    Ok(value)
}

pub(crate) fn literal(bytes: &[u8], variables: usize) -> Result<i32> {
    let (negative, digits) = match bytes.strip_prefix(b"-") {
        Some(digits) => (true, digits),
        None => (false, bytes),
    };
    let value = natural(digits)?;
    if value == 0 || value > variables as u64 {
        return Err(Failure::Invalid);
    }
    let value = i32::try_from(value).map_err(|_| Failure::Invalid)?;
    Ok(if negative { -value } else { value })
}

pub(crate) fn cnf(bytes: &[u8], res: &mut Resources<'_>) -> Result<(usize, Vec<Clause>)> {
    let limits = CnfLimits::default();
    if bytes.len() > limits.max_bytes {
        return Err(Exhausted::Size.into());
    }
    let body = bytes.strip_suffix(b"\n").ok_or(Failure::Invalid)?;
    let mut lines = body.split(|&b| b == b'\n');
    let mut header = lines.next().ok_or(Failure::Invalid)?.split(|&b| b == b' ');
    if header.next() != Some(b"p") || header.next() != Some(b"cnf") {
        return Err(Failure::Invalid);
    }
    let variables = usize::try_from(natural(header.next().ok_or(Failure::Invalid)?)?)
        .map_err(|_| Exhausted::Size)?;
    let count = usize::try_from(natural(header.next().ok_or(Failure::Invalid)?)?)
        .map_err(|_| Exhausted::Size)?;
    if header.next().is_some() {
        return Err(Failure::Invalid);
    }
    if variables > limits.max_variables || count > limits.max_clauses {
        return Err(Exhausted::Size.into());
    }
    let mut clauses = Vec::<Clause>::new();
    let mut total = 0_usize;
    for line in lines {
        if clauses.len() >= count {
            return Err(Failure::Invalid);
        }
        let mut tokens = line.split(|&b| b == b' ');
        let mut lits = Vec::new();
        loop {
            let token = tokens.next().ok_or(Failure::Invalid)?;
            if token == b"0" {
                break;
            }
            let lit = literal(token, variables)?;
            if lits.last().is_some_and(|&last| last > lit) {
                return Err(Failure::Invalid);
            }
            total = total.checked_add(1).ok_or(Exhausted::Size)?;
            if total > limits.max_literals {
                return Err(Exhausted::Size.into());
            }
            res.push(&mut lits, lit)?;
        }
        if tokens.next().is_some() || clauses.last().is_some_and(|last| last.lits > lits) {
            return Err(Failure::Invalid);
        }
        let id = u64::try_from(clauses.len()).map_err(|_| Exhausted::Size)? + 1;
        res.push(
            &mut clauses,
            Clause {
                id,
                lits,
                deps: Vec::new(),
                active: true,
            },
        )?;
    }
    if clauses.len() != count {
        return Err(Failure::Invalid);
    }
    Ok((variables, clauses))
}

pub(crate) fn proof_lines(bytes: &[u8]) -> Result<impl Iterator<Item = &[u8]>> {
    if bytes
        .iter()
        .any(|b| !b.is_ascii() || (*b < 32 && !matches!(b, b'\n' | b'\t')))
    {
        return Err(Failure::Invalid);
    }
    let body = if bytes.is_empty() {
        bytes
    } else {
        bytes.strip_suffix(b"\n").ok_or(Failure::Invalid)?
    };
    Ok(body.split(|&b| b == b'\n'))
}

pub(crate) fn tokens(line: &[u8]) -> impl Iterator<Item = &[u8]> {
    line.split(|&b| b == b' ' || b == b'\t')
        .filter(|t| !t.is_empty())
}
