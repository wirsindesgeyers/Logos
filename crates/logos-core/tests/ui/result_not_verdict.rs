use logos_core::{CheckOutcome, Verdict, verified_from_proof};

fn accept_verdict(_v: Verdict) {}

fn main() {
    let res = verified_from_proof(CheckOutcome::Accepted);
    accept_verdict(res);
}
