use logos_canon::Text;
use logos_core::{
    Budget, CheckOutcome, Exhausted, Reason, StepMeter, Verdict,
    refuted_from_countermodel, verified_from_proof,
};

fn main() {
    let budget = Budget::new(100, 1024, 2048, 500);

    let outcome = StepMeter::run(&budget, |meter| {
        if meter.charge(10).is_err() {
            return CheckOutcome::Unknown(Exhausted::Steps);
        }
        CheckOutcome::Accepted
    });

    match verified_from_proof(outcome) {
        Ok(Verdict::Verified) => {}
        Ok(Verdict::Refuted | Verdict::Unknown) => {}
        Err(CheckOutcome::Accepted) => {}
        Err(CheckOutcome::Rejected(reason)) => {
            let _s: &str = reason.as_str();
        }
        Err(CheckOutcome::Unknown(e)) => match e {
            Exhausted::Time | Exhausted::Steps | Exhausted::Memory | Exhausted::Size => {}
        },
    }

    let text = Text::new("contramodelo").expect("valid text");
    let reason = Reason::new(text);
    let outcome_refuted = CheckOutcome::Rejected(reason);

    match refuted_from_countermodel(outcome_refuted) {
        Ok(Verdict::Refuted) => {}
        Ok(Verdict::Verified | Verdict::Unknown) => {}
        Err(CheckOutcome::Accepted) => {}
        Err(CheckOutcome::Rejected(reason)) => {
            let _s: &str = reason.as_str();
        }
        Err(CheckOutcome::Unknown(_)) => {}
    }

    let _v_ver = Verdict::Verified;
    let _v_ref = Verdict::Refuted;
    let _v_unk = Verdict::Unknown;
}
