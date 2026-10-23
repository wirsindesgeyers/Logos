//! Especulação sobre Pending (Etapa 5.8, ADR0058): bases, escopo e cascata.
//!
//! Uma proposta pode listar em `speculative_on` instâncias Pending não concluídas
//! (eventos ObligationCreated). A obrigação inclui a meta de cada base; o CERTIFY
//! só aceita quando toda base tem rota Verified ativa, que substitui a base em
//! `used`. Quando uma base falha (refutada, morta ou com especulação falha), o
//! Pending especulativo recebe `Invalidated { causes: [raiz, base] }`
//! (`SpeculationFailed`). Nada aqui torna Pending citável.

use std::collections::{BTreeSet, HashMap, HashSet};

use logos_crypto::{Hash, Signer, hash};
use logos_ledger::{Event, EventBody, EventProposal, Ledger, LedgerLiveIndex};
use logos_obligation::{EntryKind, Obligation, PremiseRef};

use crate::{TransportError as Error, replay::replay, transitions::Transitions};

/// Instância Proposed+ObligationCreated, identificada pelo ObligationCreated.
pub(crate) struct Instance {
    pub(crate) obligation: Hash<Obligation>,
    pub(crate) proposal: EventProposal,
}

/// Relações reconstruídas por um replay autenticado do ledger.
#[derive(Default)]
pub(crate) struct History {
    instances: HashMap<Hash<Event>, Instance>,
    /// Instâncias em ordem de seq; bases são sempre anteriores ao especulativo.
    order: Vec<Hash<Event>>,
    /// Obrigações com Verified ou Refuted: suas instâncias não são mais Pending.
    finished: HashSet<Hash<Obligation>>,
    /// Primeiro Refuted (por seq) de cada obrigação.
    refuted: HashMap<Hash<Obligation>, Hash<Event>>,
    /// Alvo de Invalidated -> raiz registrada (`causes[0]`).
    recorded: HashMap<Hash<Event>, Hash<Event>>,
}

impl History {
    pub(crate) fn read<S: Signer>(ledger: &Ledger<'_, S>) -> Result<Self, Error> {
        let mut history = Self::default();
        let mut proposed: Option<EventProposal> = None;
        replay(ledger, |event| {
            match &event.body {
                EventBody::Proposed { proposal, .. } => proposed = Some(proposal.clone()),
                EventBody::ObligationCreated { obligation, .. } => {
                    let proposal = proposed
                        .take()
                        .ok_or(Error::Invalid("Pending without proposal"))?;
                    let id = hash(event)?;
                    history.order.push(id);
                    history.instances.insert(
                        id,
                        Instance {
                            obligation: *obligation,
                            proposal,
                        },
                    );
                }
                EventBody::Verified { check, .. } => {
                    history.finished.insert(check.obligation);
                }
                EventBody::Refuted { check, .. } => {
                    history.finished.insert(check.obligation);
                    if let std::collections::hash_map::Entry::Vacant(slot) =
                        history.refuted.entry(check.obligation)
                    {
                        slot.insert(hash(event)?);
                    }
                }
                EventBody::Invalidated { claim, causes } => {
                    if let Some(root) = causes.first() {
                        history.recorded.entry(*claim).or_insert(*root);
                    }
                }
                _ => {}
            }
            Ok(())
        })?;
        Ok(history)
    }

    pub(crate) fn instance(&self, pending: Hash<Event>) -> Option<&Instance> {
        self.instances.get(&pending)
    }

    /// Confere as bases de uma proposta nova: instâncias Pending não concluídas,
    /// não invalidadas nem retratadas, da época corrente. Devolve as referências.
    pub(crate) fn bases(
        &self,
        index: &LedgerLiveIndex<'_>,
        speculative_on: &[[u8; 32]],
        epoch: u64,
    ) -> Result<Vec<PremiseRef>, Error> {
        let mut out = Vec::with_capacity(speculative_on.len());
        for id in speculative_on {
            let pending = Hash::from_bytes(*id);
            let instance = self
                .instances
                .get(&pending)
                .ok_or(Error::Invalid("speculative base is not a Pending instance"))?;
            if self.finished.contains(&instance.obligation) {
                return Err(Error::Invalid("speculative base is already finished"));
            }
            let status = index
                .status(pending)?
                .ok_or(Error::Invalid("speculative base missing from index"))?;
            if status.kind != EntryKind::Pending || status.retracted || status.epoch != epoch {
                return Err(Error::Invalid(
                    "speculative base is not an eligible Pending",
                ));
            }
            out.push(PremiseRef::from_event_hash(*id));
        }
        Ok(out)
    }

    /// Citações que definem o escopo derivado (ADR0054): as da proposta e as das
    /// bases, fechadas transitivamente sobre bases especulativas.
    pub(crate) fn scope_cites(&self, proposal: &EventProposal) -> Vec<Hash<Event>> {
        let mut cites: BTreeSet<Hash<Event>> = proposal.cites.iter().copied().collect();
        let mut seen = HashSet::new();
        let mut work: Vec<Hash<Event>> = proposal
            .speculative_on
            .iter()
            .map(|id| Hash::from_bytes(*id))
            .collect();
        while let Some(base) = work.pop() {
            if !seen.insert(base) {
                continue;
            }
            if let Some(instance) = self.instances.get(&base) {
                cites.extend(instance.proposal.cites.iter().copied());
                work.extend(
                    instance
                        .proposal
                        .speculative_on
                        .iter()
                        .map(|id| Hash::from_bytes(*id)),
                );
            }
        }
        cites.into_iter().collect()
    }
}

/// Pares (base, rota Verified que a resolve).
pub(crate) type Settled = Vec<(Hash<Event>, Hash<Event>)>;

/// Rota Verified ativa escolhida para cada base, em ordem de base; recusa se
/// alguma base ainda não tem rota ativa citável (o especulativo segue Pending).
pub(crate) fn settle_bases(
    history: &History,
    index: &LedgerLiveIndex<'_>,
    proposal: &EventProposal,
) -> Result<Settled, Error> {
    let bases: BTreeSet<Hash<Event>> = proposal
        .speculative_on
        .iter()
        .map(|id| Hash::from_bytes(*id))
        .collect();
    let mut out = Vec::with_capacity(bases.len());
    for base in bases {
        let instance = history
            .instance(base)
            .ok_or(Error::Invalid("speculative base is not a Pending instance"))?;
        let route = index
            .active_route(instance.obligation)?
            .ok_or(Error::Invalid("speculative base is not Verified"))?;
        out.push((base, route));
    }
    Ok(out)
}

/// Grava `SpeculationFailed` para todo Pending especulativo não concluído com
/// base falha. Idempotente; uma passagem em ordem de seq basta porque as bases
/// são anteriores. Devolve os Invalidated gravados.
pub(crate) fn sweep<S: Signer>(ledger: &mut Ledger<'_, S>) -> Result<Vec<Hash<Event>>, Error> {
    let history = History::read(ledger)?;
    let mut failed: HashMap<Hash<Event>, Hash<Event>> = HashMap::new();
    let mut pending = Vec::new();
    {
        let index = ledger.live_index()?;
        for id in &history.order {
            if let Some(root) = history.recorded.get(id) {
                failed.insert(*id, *root);
                continue;
            }
            let Some(instance) = history.instances.get(id) else {
                continue;
            };
            if history.finished.contains(&instance.obligation) {
                continue;
            }
            let bases: BTreeSet<Hash<Event>> = instance
                .proposal
                .speculative_on
                .iter()
                .map(|b| Hash::from_bytes(*b))
                .collect();
            let mut cause = None;
            for base in bases {
                let root = if let Some(root) = failed.get(&base) {
                    Some(*root)
                } else if let Some(origin) = history.instances.get(&base) {
                    match history.refuted.get(&origin.obligation) {
                        Some(refuted) => Some(*refuted),
                        None => index
                            .dead_routes(origin.obligation)?
                            .first()
                            .map(|route| *history.recorded.get(route).unwrap_or(route)),
                    }
                } else {
                    None
                };
                if let Some(root) = root {
                    cause = Some((root, base));
                    break;
                }
            }
            if let Some((root, base)) = cause {
                failed.insert(*id, root);
                pending.push((*id, vec![root, base]));
            }
        }
    }
    let mut transitions = Transitions::new(ledger);
    let mut out = Vec::with_capacity(pending.len());
    for (id, causes) in pending {
        let claim = transitions.recover_pending_at(id)?;
        out.push(transitions.abandon(claim, causes)?.receipt().hash());
    }
    Ok(out)
}
