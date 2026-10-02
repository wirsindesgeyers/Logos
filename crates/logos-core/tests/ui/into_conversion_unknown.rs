use logos_core::{CheckOutcome, Exhausted, Verdict};

fn main() {
    let outcome = CheckOutcome::Unknown(Exhausted::Steps);
    let _verdict: Verdict = outcome.into();
}
