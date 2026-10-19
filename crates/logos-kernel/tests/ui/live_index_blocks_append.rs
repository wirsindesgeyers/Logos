use logos_crypto::FileSigner;
use logos_ledger::{EventBody, Ledger};

fn blocked(ledger: &mut Ledger<'_, FileSigner>, body: EventBody) {
    let view = ledger.live_index().unwrap();
    ledger.append(body).unwrap();
    drop(view);
}

fn main() {}
