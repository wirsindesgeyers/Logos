use logos_core::{CheckOutcome, Verdict};

fn main() {
    let outcome = CheckOutcome::Accepted;
    let _verdict: Verdict = outcome;
}
