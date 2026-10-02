//! Smoke reproduzível da API pública: `cargo run -p logos-core --example core_smoke`.

use std::error::Error;

use logos_canon::{CanonError, Text, from_canon, to_canon};
use logos_core::{
    Budget, CheckOutcome, Exhausted, Reason, StepMeter, Verdict, refuted_from_countermodel,
    verified_from_proof,
};

fn require(condition: bool, description: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(description.into())
    }
}

// Uma cobrança antes de cada byte; 255 é inválido apenas se houver saldo para inspecioná-lo.
fn check(meter: &StepMeter, bytes: &[u8], reason: &Reason) -> CheckOutcome {
    for &byte in bytes {
        if let Err(exhausted) = meter.charge(1) {
            return CheckOutcome::Unknown(exhausted);
        }
        if byte == 255 {
            return CheckOutcome::Rejected(reason.clone());
        }
    }
    CheckOutcome::Accepted
}

fn main() -> Result<(), Box<dyn Error>> {
    let reason = Reason::new(Text::new("invalid byte 255")?);
    let budget = |steps| Budget {
        steps,
        mem_bytes: 0,
        size_bytes: 0,
        wall_ms: 0,
    };
    for steps in [3, 10] {
        require(
            StepMeter::run(&budget(steps), |meter| check(meter, &[1, 2, 3], &reason))
                == CheckOutcome::Accepted,
            "limite exato/suficiente não aceitou",
        )?;
    }
    for (steps, bytes) in [(0, &[255][..]), (2, &[1, 2, 255][..])] {
        require(
            StepMeter::run(&budget(steps), |meter| check(meter, bytes, &reason))
                == CheckOutcome::Unknown(Exhausted::Steps),
            "exaustão não precedeu rejeição",
        )?;
    }
    require(
        StepMeter::run(&budget(3), |meter| check(meter, &[1, 2, 255], &reason))
            == CheckOutcome::Rejected(reason.clone()),
        "byte inválido com saldo não foi rejeitado",
    )?;
    require(
        StepMeter::run(&budget(0), |meter| check(meter, &[], &reason)) == CheckOutcome::Accepted,
        "zero trabalho com zero orçamento não aceitou",
    )?;
    for attempted in [
        CheckOutcome::Accepted,
        CheckOutcome::Rejected(reason.clone()),
        CheckOutcome::Unknown(Exhausted::Time),
    ] {
        let outcome = StepMeter::run(&budget(1), |meter| {
            let _ = meter.charge(2);
            attempted
        });
        require(
            outcome == CheckOutcome::Unknown(Exhausted::Steps),
            "erro ignorado perdeu latch",
        )?;
    }
    require(
        verified_from_proof(CheckOutcome::Accepted) == Ok(Verdict::Verified),
        "prova aceita",
    )?;
    require(
        refuted_from_countermodel(CheckOutcome::Accepted) == Ok(Verdict::Refuted),
        "contramodelo aceito",
    )?;
    for outcome in [
        CheckOutcome::Rejected(reason),
        CheckOutcome::Unknown(Exhausted::Time),
        CheckOutcome::Unknown(Exhausted::Steps),
        CheckOutcome::Unknown(Exhausted::Memory),
        CheckOutcome::Unknown(Exhausted::Size),
    ] {
        require(
            verified_from_proof(outcome.clone()) == Err(outcome.clone()),
            "prova promoveu erro",
        )?;
        require(
            refuted_from_countermodel(outcome.clone()) == Err(outcome),
            "contramodelo promoveu erro",
        )?;
    }

    // Bytes normativos core/v1, não produzidos pelo encoder sob checagem.
    require(
        from_canon::<Budget>(&[0x85, 1, 1, 2, 3, 4])?
            == Budget {
                steps: 1,
                mem_bytes: 2,
                size_bytes: 3,
                wall_ms: 4,
            },
        "decode Budget",
    )?;
    require(
        from_canon::<Reason>(&[0x82, 1, 0x62, 0xc3, 0xa9])?.as_str() == "é",
        "decode Reason NFC",
    )?;
    require(
        from_canon::<Exhausted>(&[0x82, 1, 3])? == Exhausted::Size,
        "decode Exhausted",
    )?;
    require(
        from_canon::<Verdict>(&[0x82, 1, 1])? == Verdict::Refuted,
        "decode Verdict",
    )?;
    let unknown_bytes = [0x83, 1, 2, 0x82, 1, 1];
    let unknown = from_canon::<CheckOutcome>(&unknown_bytes)?;
    require(
        unknown == CheckOutcome::Unknown(Exhausted::Steps),
        "decode outcome",
    )?;
    require(to_canon(&unknown)? == unknown_bytes, "bytes do outcome")?;
    require(
        from_canon::<CheckOutcome>(&[0x83, 1, 0, 0]) == Err(CanonError::WrongType),
        "Accepted aceitou payload",
    )?;
    require(
        from_canon::<Verdict>(&[0x82, 2, 1]) == Err(CanonError::WrongType),
        "versão desconhecida aceita",
    )?;
    require(
        from_canon::<Exhausted>(&[0x82, 1, 4]) == Err(CanonError::WrongType),
        "tag desconhecida aceita",
    )?;
    println!(
        "CORE_SMOKE_OK: budgets, latch, veredictos e core/v1 ({})",
        std::env::consts::ARCH
    );
    Ok(())
}
