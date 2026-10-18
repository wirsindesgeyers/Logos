use logos_core::CheckOutcome;
use logos_kernel::{Claim, state::Verified};
fn main() { let _: Claim<Verified> = CheckOutcome::Accepted.into(); }
