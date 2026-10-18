use logos_kernel::{Claim, state::{Refuted, Verified}};
fn promote(c: Claim<Refuted>) -> Claim<Verified> { c.accept() }
fn main() {}
