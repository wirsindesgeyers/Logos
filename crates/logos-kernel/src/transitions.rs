//! Única construção de claims: consumo do anterior, validação e append durável.

use core::marker::PhantomData;
use std::sync::Arc;

use logos_canon::Text;
use logos_core::{CheckOutcome, refuted_from_countermodel, verified_from_proof};
use logos_crypto::{Hash, KeyId, Signer, hash};
use logos_ledger::{
    CheckRecord, CnfRecord, Consistency, Event, EventBody, EventProposal, Ledger, LedgerError,
    ModelObject, Route,
};
use logos_obligation::Obligation;

use crate::state::{Draft, Invalidated, Pending, Refuted, Unknown, Verified};

/// Recibo tipado e imutável de um evento confirmado pelo appender.
///
/// Não pode ser construído de Event/assinatura/CheckOutcome externos.
#[derive(Debug)]
pub struct SignedEvent<S> {
    event: Event,
    hash: Hash<Event>,
    state: PhantomData<fn() -> S>,
}

impl<S> SignedEvent<S> {
    /// Evento completo confirmado; não prova citabilidade atual.
    #[must_use]
    pub fn event(&self) -> &Event {
        &self.event
    }

    /// Identidade do evento completo, incluindo assinatura.
    #[must_use]
    pub fn hash(&self) -> Hash<Event> {
        self.hash
    }
}

#[derive(Debug)]
struct Origin {
    session: Arc<()>,
    proposal: Hash<Event>,
    epoch: u64,
    obligation: Option<Hash<Obligation>>,
    cited: Vec<Hash<Event>>,
}

/// Claim linear cujo estado só é criado pelo módulo de transições.
///
/// Sem Clone/Copy/Default/decoder. O tipo não substitui a consulta de
/// citabilidade do ledger nem constitui autorização para efeitos.
#[derive(Debug)]
pub struct Claim<S> {
    origin: Origin,
    receipt: SignedEvent<S>,
}

impl<S> Claim<S> {
    /// Proposta que iniciou esta instância do claim.
    #[must_use]
    pub fn proposal(&self) -> Hash<Event> {
        self.origin.proposal
    }

    /// Época de origem; uma invalidação posterior não a reescreve.
    #[must_use]
    pub fn epoch(&self) -> u64 {
        self.origin.epoch
    }

    /// Obrigação atribuída; ausente somente em Draft.
    #[must_use]
    pub fn obligation(&self) -> Option<Hash<Obligation>> {
        self.origin.obligation
    }

    /// Recibo confirmado da última transição.
    #[must_use]
    pub fn receipt(&self) -> &SignedEvent<S> {
        &self.receipt
    }
}

// Os handlers5.3/5.4 consumirão esta API interna. Os testes exercitam todas
// as operações agora; não expor emissão ao agente para evitar dead_code.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "integração nos handlers5.3/5.4")
)]
pub(crate) struct Transitions<'writer, 'signer, S: Signer> {
    ledger: &'writer mut Ledger<'signer, S>,
    session: Arc<()>,
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "integração nos handlers5.3/5.4")
)]
impl<'writer, 'signer, S: Signer> Transitions<'writer, 'signer, S> {
    pub(crate) fn new(ledger: &'writer mut Ledger<'signer, S>) -> Self {
        Self {
            ledger,
            session: Arc::new(()),
        }
    }

    fn append<T>(&mut self, body: EventBody) -> Result<SignedEvent<T>, LedgerError> {
        let event = self.ledger.append(body)?;
        let hash = hash(&event)?;
        Ok(SignedEvent {
            event,
            hash,
            state: PhantomData,
        })
    }

    fn validate<T>(&self, claim: &Claim<T>, current_epoch: bool) -> Result<(), LedgerError> {
        if !Arc::ptr_eq(&self.session, &claim.origin.session) {
            return Err(LedgerError::Invalid(
                "claim from another transition session",
            ));
        }
        let head = self
            .ledger
            .head()?
            .ok_or(LedgerError::Invalid("claim without ledger head"))?;
        if current_epoch && head.epoch != claim.epoch() {
            return Err(LedgerError::Invalid("claim epoch is no longer current"));
        }
        Ok(())
    }

    fn validate_check(
        &self,
        claim: &Claim<Pending>,
        check: &CheckRecord,
    ) -> Result<(), LedgerError> {
        self.validate(claim, true)?;
        if claim.obligation() != Some(check.obligation) {
            return Err(LedgerError::Invalid("check obligation differs from claim"));
        }
        Ok(())
    }

    pub(crate) fn propose(
        &mut self,
        proposal: EventProposal,
        author: KeyId,
    ) -> Result<Claim<Draft>, LedgerError> {
        let mut cited = proposal.cites.clone();
        cited.sort_unstable();
        cited.dedup();
        let receipt = self.append(EventBody::Proposed { proposal, author })?;
        Ok(Claim {
            origin: Origin {
                session: Arc::clone(&self.session),
                proposal: receipt.hash(),
                epoch: receipt.event().epoch,
                obligation: None,
                cited,
            },
            receipt,
        })
    }

    // O handler5.3 deve passar h(O) de generate, nunca bytes do agente.
    pub(crate) fn submit(
        &mut self,
        claim: Claim<Draft>,
        obligation: Hash<Obligation>,
        route: Route,
        artifact: [u8; 32],
        cnf: Option<CnfRecord>,
    ) -> Result<Claim<Pending>, LedgerError> {
        self.validate(&claim, true)?;
        let receipt = self.append(EventBody::ObligationCreated {
            obligation,
            route,
            artifact,
            cnf,
        })?;
        let mut origin = claim.origin;
        origin.obligation = Some(obligation);
        Ok(Claim { origin, receipt })
    }

    // Precondição interna: prova/used conferidos por checker ativo (handler5.4).
    // Accepted público, sozinho, não é uma credencial para chamar esta função.
    pub(crate) fn accept(
        &mut self,
        claim: Claim<Pending>,
        check: CheckRecord,
        cited: Vec<Hash<Event>>,
        used: Vec<Hash<Event>>,
        consistency: Consistency,
    ) -> Result<Claim<Verified>, LedgerError> {
        self.validate_check(&claim, &check)?;
        verified_from_proof(check.outcome.clone())
            .map_err(|_| LedgerError::Invalid("Verified requires accepted proof"))?;
        if cited != claim.origin.cited {
            return Err(LedgerError::Invalid("Verified cites differ from proposal"));
        }
        let receipt = self.append(EventBody::Verified {
            check,
            cited,
            used,
            consistency,
        })?;
        Ok(Claim {
            origin: claim.origin,
            receipt,
        })
    }

    // Precondição interna: contramodelo conferido por checker ativo (handler5.4).
    pub(crate) fn refute(
        &mut self,
        claim: Claim<Pending>,
        check: CheckRecord,
        model: Hash<ModelObject>,
    ) -> Result<Claim<Refuted>, LedgerError> {
        self.validate_check(&claim, &check)?;
        refuted_from_countermodel(check.outcome.clone())
            .map_err(|_| LedgerError::Invalid("Refuted requires accepted countermodel"))?;
        let receipt = self.append(EventBody::Refuted { check, model })?;
        Ok(Claim {
            origin: claim.origin,
            receipt,
        })
    }

    pub(crate) fn unknown(
        &mut self,
        claim: Claim<Pending>,
        check: CheckRecord,
        machine: Text,
    ) -> Result<Claim<Unknown>, LedgerError> {
        self.validate_check(&claim, &check)?;
        if !matches!(check.outcome, CheckOutcome::Unknown(_)) {
            return Err(LedgerError::Invalid("Unknown requires exhausted check"));
        }
        let receipt = self.append(EventBody::UnknownRecorded { check, machine })?;
        Ok(Claim {
            origin: claim.origin,
            receipt,
        })
    }

    // A seleção das causas/cascata pertence ao handler5.9.
    pub(crate) fn invalidate(
        &mut self,
        claim: Claim<Verified>,
        causes: Vec<Hash<Event>>,
    ) -> Result<Claim<Invalidated>, LedgerError> {
        self.validate(&claim, false)?;
        let receipt = self.append(EventBody::Invalidated {
            claim: claim.receipt().hash(),
            causes,
        })?;
        Ok(Claim {
            origin: claim.origin,
            receipt,
        })
    }
}

#[cfg(test)]
#[path = "../tests/support/transitions.rs"]
mod tests;
