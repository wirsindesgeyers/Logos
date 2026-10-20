//! Replay paginado autenticado; limites e continuidade independentes do cache.

use logos_crypto::{Signer, hash};
use logos_ledger::{Event, Ledger};

use crate::TransportError as Error;

pub(crate) fn replay<S: Signer>(
    ledger: &Ledger<'_, S>,
    mut visit: impl FnMut(&Event) -> Result<(), Error>,
) -> Result<(), Error> {
    let head = ledger.head()?.ok_or(Error::Invalid("missing Genesis"))?;
    if head.seq >= 1_000_000 {
        return Err(Error::Limit("certification replay events"));
    }
    let mut next = 0;
    let mut previous = None;
    while next <= head.seq {
        let page = ledger.events_from(next, 256)?;
        if page.is_empty() {
            return Err(Error::Invalid("incomplete certification replay"));
        }
        for event in page {
            if event.seq != next || event.prev != previous {
                return Err(Error::Invalid("certification replay continuity"));
            }
            previous = Some(hash(&event)?);
            visit(&event)?;
            next += 1;
        }
    }
    if previous != Some(head.hash) || ledger.head()? != Some(head) {
        return Err(Error::Invalid("certification replay head"));
    }
    Ok(())
}
