//! Protocolo estrito de saída do binário fixado, não de um solver arbitrário.
use crate::cake::{Result, fail};
use logos_canon::Text;
use logos_core::{CheckOutcome, Exhausted, Reason};

/// Decodifica saída completa e status; função pura também exercitada pelo fuzz.
///
/// # Errors
/// Status/saída não reconhecidos são falha operacional, nunca aceite parcial.
pub fn decode_cake_output(status: i32, stdout: &[u8], stderr: &[u8]) -> Result<CheckOutcome> {
    if stdout.len() > 65_536 || stderr.len() > 65_536 {
        return Err(fail("cake_lpr output limit"));
    }
    if status == 0 && stdout == b"s VERIFIED UNSAT\n" && stderr.is_empty() {
        return Ok(CheckOutcome::Accepted);
    }
    if stderr == b"CakeML heap space exhausted.\n" || stderr == b"CakeML stack space exhausted.\n" {
        return Ok(CheckOutcome::Unknown(Exhausted::Memory));
    }
    // O binário fixado usa exit0 também para erros de prova: o protocolo é a saída completa.
    if status == 0
        && stdout.is_empty()
        && (stderr == b"c empty clause not derived at end of proof\n"
            || (stderr.starts_with(b"c Checking failed at line: ") && stderr.ends_with(b"\n")))
    {
        let reason =
            Text::new("cake_lpr rejected certificate").map_err(|_| fail("invalid reason"))?;
        return Ok(CheckOutcome::Rejected(Reason::new(reason)));
    }
    Err(fail("cake_lpr unexpected exit/output or sandbox failure"))
}
