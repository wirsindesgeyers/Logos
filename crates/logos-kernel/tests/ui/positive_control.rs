use logos_core::Verdict;
use logos_kernel::{Claim, SignedEvent, state::{Draft, Pending, Verified, Refuted, Unknown, Invalidated}};
fn inspect<S>(c: &Claim<S>) {
    let _ = (c.proposal(), c.epoch(), c.obligation());
    let ev: &SignedEvent<S> = c.receipt();
    let _ = (ev.event(), ev.hash());
}
fn all_states(a: &Claim<Draft>, b: &Claim<Pending>, c: &Claim<Verified>, d: &Claim<Refuted>, e: &Claim<Unknown>, f: &Claim<Invalidated>) {
    inspect(a); inspect(b); inspect(c); inspect(d); inspect(e); inspect(f);
}
fn main() { let _ = Verdict::Verified; }
