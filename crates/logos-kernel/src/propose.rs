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
use logos_obligation::{DecodeLimits, Obligation, PremiseRef, Proposal, generate_speculative};

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
        create(ledger, proposal, false)
    }
}

pub(crate) fn create<S: Signer>(
    ledger: &mut Ledger<'_, S>,
    proposal: EventProposal,
    layer1: bool,
) -> Result<Value, Error> {
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
        let index = ledger.live_index()?;
        let epoch = ledger
            .head()?
            .ok_or(Error::Invalid("PROPOSE requires Genesis"))?
            .epoch;
        // 5.8 (ADR0058): bases especulativas são instâncias Pending não concluídas;
        // entram em Γ com a meta delas e no cálculo do escopo derivado.
        let (speculative, scope_cites) = if proposal.speculative_on.is_empty() {
            (Vec::new(), proposal.cites.clone())
        } else {
            let history = crate::speculation::History::read(ledger)?;
            (
                history.bases(&index, &proposal.speculative_on, epoch)?,
                history.scope_cites(&proposal),
            )
        };
        let scope = crate::scope::for_proposal(&index, &scope_cites, epoch)?;
        let mut snapshot_refs = references.clone();
        snapshot_refs.extend_from_slice(&speculative);
        let view = index.snapshot(ledger.cas(), &snapshot_refs, &scope)?;
        generate_speculative(
            Proposal {
                vocabulary: &vocabulary,
                goal: &proposal.goal,
                cites: &references,
            },
            &speculative,
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
    let hash = logos_crypto::hash_raw::<Obligation>(&bytes);
    // S4 (5.6): antes de publicar O, recusar obrigação já refutada.
    let refuted = ledger.live_index()?.refuted(hash)?;
    if let Some(refuted) = refuted {
        let mut model = None;
        crate::replay::replay(ledger, |event| {
            if let logos_ledger::EventBody::Refuted { model: m, .. } = &event.body
                && logos_crypto::hash(event)? == refuted
            {
                model = Some(*m);
            }
            Ok(())
        })?;
        return Err(Error::AlreadyRefuted {
            obligation: hash,
            refuted,
            model: model.ok_or(Error::Invalid("Refuted event missing from replay"))?,
        });
    }
    if ledger.cas().put::<Obligation>(&bytes)? != hash {
        return Err(Error::Invalid("obligation CAS identity"));
    }
    let response = Value::Array(Array::new(vec![
        Value::Unsigned(1),
        hash.to_value()?,
        Value::Bytes(bytes),
    ])?);
    let mut route = Route::Model;
    let mut artifact = *hash.as_bytes();
    let mut record = None;
    if layer1
        && !vocabulary
            .sorts()
            .iter()
            .any(|s| matches!(s.domain, logos_air::SortDomain::Enumerated { .. }))
    {
        match logos_ground::ground(&obligation, &vocabulary)
            .and_then(|g| logos_ground::clausify(&g))
        {
            Ok((cnf, _)) => {
                let h = ledger.cas().put::<logos_ledger::CnfObject>(cnf.dimacs())?;
                route = Route::Lrat;
                artifact = *logos_crypto::ExternalDigest::of(cnf.dimacs()).as_bytes();
                record = Some(logos_ledger::CnfRecord {
                    hash: h,
                    encoder_version: 1,
                });
            }
            Err(logos_ground::GroundError::GeneralFragment) => {}
            Err(logos_ground::GroundError::Unknown(_)) => {
                return Err(Error::Limit("proposal grounding"));
            }
            Err(_) => return Err(Error::Invalid("proposal grounding")),
        }
    }
    let mut transitions = Transitions::new(ledger);
    let draft = transitions.propose(proposal, author)?;
    let _pending = transitions.submit(draft, hash, route, artifact, record)?;
    Ok(response)
}
