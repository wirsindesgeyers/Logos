use logos_crypto::FileSigner;
use logos_ledger::{EventBody, Ledger};

fn allowed(ledger: &mut Ledger<'_, FileSigner>, body: EventBody) {
    let view = ledger.live_index().unwrap();
    let second = ledger.live_index().unwrap();
    assert_eq!(view.head(), second.head());
    drop(second);
    drop(view);
    ledger.append(body).unwrap();
}

fn main() {}
