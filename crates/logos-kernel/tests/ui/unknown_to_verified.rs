use logos_kernel::{Claim, state::{Unknown, Verified}};
fn promote(c: Claim<Unknown>) -> Claim<Verified> { c }
fn main() {}
