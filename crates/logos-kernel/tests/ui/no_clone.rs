use logos_kernel::{Claim, state::Verified};
fn duplicate(c: Claim<Verified>) -> (Claim<Verified>, Claim<Verified>) {
    (c.clone(), c)
}
fn main() {}
