use logos_kernel::{Claim, state::Pending};
fn consume(c: Claim<Pending>) -> Claim<Pending> { c }
fn main() {}
fn reuse(c: Claim<Pending>) {
    let _first = consume(c);
    let _second = consume(c);
}
