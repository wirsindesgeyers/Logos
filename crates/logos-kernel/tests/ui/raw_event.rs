use logos_kernel::{SignedEvent, state::Verified};
use logos_ledger::Event;
fn promote(event: Event) -> SignedEvent<Verified> { event.into() }
fn main() {}
