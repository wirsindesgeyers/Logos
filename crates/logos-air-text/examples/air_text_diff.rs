//! Batch bridge for the independent Python/Rust surface differential.
//! Input: one UTF-8 document encoded as hex per line.
//! Output: ok<TAB>Vocabulary hex<TAB>comma-separated Formula hex<TAB>printed UTF-8 hex.

use logos_air_text::{References, parse_document, print_document};
use logos_canon::to_canon;
use std::{
    error::Error,
    io::{self, BufRead, Write},
};

fn write_hex(output: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        output.write_all(&[
            DIGITS[usize::from(byte >> 4)],
            DIGITS[usize::from(byte & 15)],
        ])?;
    }
    Ok(())
}

fn unhex(text: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !text.len().is_multiple_of(2) {
        return Err("odd hex input length".into());
    }
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let digit = |byte: u8| -> Result<u8, Box<dyn Error>> {
                match byte {
                    b'0'..=b'9' => Ok(byte - b'0'),
                    b'a'..=b'f' => Ok(byte - b'a' + 10),
                    b'A'..=b'F' => Ok(byte - b'A' + 10),
                    _ => Err("invalid hex input".into()),
                }
            };
            Ok((digit(pair[0])? << 4) | digit(pair[1])?)
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let references = References::new();
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());
    for line in io::stdin().lock().lines() {
        let bytes = unhex(&line?)?;
        let source = std::str::from_utf8(&bytes)?;
        let document = parse_document(source, &references)?;
        let printed = print_document(&document);
        let roundtrip = parse_document(&printed, &references)?;
        if roundtrip.vocabulary != document.vocabulary || roundtrip.claims != document.claims {
            return Err(
                "Rust document printer changed canonical identity or claim metadata".into(),
            );
        }
        output.write_all(b"ok\t")?;
        write_hex(&mut output, &to_canon(&document.vocabulary)?)?;
        output.write_all(b"\t")?;
        for (index, claim) in document.claims.iter().enumerate() {
            if index != 0 {
                output.write_all(b",")?;
            }
            write_hex(&mut output, &to_canon(&claim.formula)?)?;
        }
        output.write_all(b"\t")?;
        write_hex(&mut output, printed.as_bytes())?;
        output.write_all(b"\n")?;
    }
    output.flush()?;
    Ok(())
}
