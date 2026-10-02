use logos_core::{CheckOutcome, Exhausted, Verdict};

fn main() {
    let outcome = CheckOutcome::Unknown(Exhausted::Steps);
    let _verdict: Result<Verdict, _> = Verdict::try_from(outcome);
}
