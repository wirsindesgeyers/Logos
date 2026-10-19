//! PROPOSE: somente o kernel gera O a partir de citações autenticadas.

use crate::{
    TransportError as Error,
    server::{Handlers, Peer},
    transitions::Transitions,
};
use logos_air::{Limits, Vocabulary};
use logos_canon::{Array, Canon, Value, to_canon};
use logos_crypto::Signer;
use logos_ledger::{EventProposal, Ledger, Route};
use logos_obligation::{DecodeLimits, Obligation, PremiseRef, Proposal, generate};

/// Handler sem estado lógico próprio; identidade e contexto vêm do ledger.
/// A autoria de Proposed é a do kernel, não uma assinatura do agente.
pub struct ProposalHandlers;

impl<S: Signer> Handlers<S> for ProposalHandlers {
    fn propose(
        &self,
        ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        proposal: EventProposal,
    ) -> Result<Value, Error> {
        if !proposal.speculative_on.is_empty() {
            return Err(Error::Invalid("speculation requires 5.8"));
        }
        let author = ledger
            .events_from(0, 1)?
            .first()
            .ok_or(Error::Invalid("PROPOSE requires Genesis"))?
            .kernel_key;
        let bytes = ledger
            .cas()
            .get(&proposal.vocabulary)?
            .ok_or(Error::Invalid("missing proposal vocabulary"))?;
        let vocabulary = Vocabulary::decode_with_limits(&bytes, Limits::default())
            .map_err(|_| Error::Invalid("invalid proposal vocabulary"))?;
        let references: Vec<_> = proposal
            .cites
            .iter()
            .map(|h| PremiseRef::from_event_hash(*h.as_bytes()))
            .collect();
        let obligation = {
            let index = ledger.index()?;
            let view = index.snapshot(ledger.cas(), &references, "")?;
            generate(
                Proposal {
                    vocabulary: &vocabulary,
                    goal: &proposal.goal,
                    cites: &references,
                },
                &view,
            )
            .map_err(|error| {
                eprintln!("kernel PROPOSE rejected: {error}");
                Error::Invalid("proposal rejected by obligation generator")
            })?
        };
        let bytes = to_canon(&obligation)?;
        if bytes.len() > DecodeLimits::DEFAULT_MAX_BYTES {
            return Err(Error::Limit("generated obligation bytes"));
        }
        let hash = ledger.cas().put::<Obligation>(&bytes)?;
        let response = Value::Array(Array::new(vec![
            Value::Unsigned(1),
            hash.to_value()?,
            Value::Bytes(bytes),
        ])?);
        let mut transitions = Transitions::new(ledger);
        let draft = transitions.propose(proposal, author)?;
        let _pending = transitions.submit(draft, hash, Route::Model, *hash.as_bytes(), None)?;
        Ok(response)
    }
}
