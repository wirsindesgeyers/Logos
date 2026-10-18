use logos_kernel::{Claim, state::{Pending, Verified}};
fn promote(c: Claim<Pending>) -> Claim<Verified> { c }
fn main() {}
