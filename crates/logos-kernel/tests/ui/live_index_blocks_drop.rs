use logos_crypto::FileSigner;
use logos_ledger::Ledger;

fn blocked(ledger: Ledger<'_, FileSigner>) {
    let view = ledger.live_index().unwrap();
    drop(ledger);
    drop(view);
}

fn main() {}
