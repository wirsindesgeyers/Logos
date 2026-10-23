//! Testemunhas anexas: replay autenticado, instância exata e recheck contextual.

use logos_canon::Value;
use logos_core::CheckOutcome;
use logos_crypto::{Hash, Signer, hash};
use logos_ledger::{
    CheckRecord, Consistency, Event, EventBody, EventProposal, Ledger, ModelObject,
};
use logos_model::{FiniteModel, ModelChecker};
use logos_obligation::{EntryKind, Obligation};

use crate::{
    TransportError as Error,
    certify::{CertificationHandlers, check_result, context},
    registry::MODEL,
    replay::replay,
};

/// Janela lógica inclusiva; reiniciar o servidor não renova o prazo.
pub const WITNESS_WINDOW_EVENTS: u64 = 1024;

/// Uso explícito: citabilidade lógica não implica elegibilidade para ação.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PremiseUse {
    /// Consulta e raciocínio lógico ordinário.
    Logical,
    /// Premissa a ser consumida pela autorização da Fase8.
    Action,
}

struct Instance {
    target: Event,
    pending_seq: u64,
    proposal_hash: Hash<Event>,
    proposal: EventProposal,
    verified: bool,
    consistency: Consistency,
}

fn instance<S: Signer>(
    ledger: &Ledger<'_, S>,
    obligation: Hash<Obligation>,
    reference: Option<Hash<Event>>,
) -> Result<Instance, Error> {
    let mut proposed: Option<Event> = None;
    let mut current: Option<Instance> = None;
    let mut selected = false;
    replay(ledger, |event| {
        match &event.body {
            EventBody::Proposed { .. } => proposed = Some(event.clone()),
            EventBody::ObligationCreated { obligation: h, .. } if *h == obligation => {
                if selected {
                    return Ok(());
                }
                let draft = proposed
                    .take()
                    .ok_or(Error::Invalid("witness without proposal"))?;
                if event.seq != draft.seq + 1
                    || event.prev != Some(hash(&draft)?)
                    || event.epoch != draft.epoch
                {
                    return Err(Error::Invalid("witness proposal adjacency"));
                }
                let proposal_hash = hash(&draft)?;
                let EventBody::Proposed { proposal, .. } = draft.body else {
                    return Err(Error::Invalid("witness proposal"));
                };
                current = Some(Instance {
                    target: event.clone(),
                    pending_seq: event.seq,
                    proposal_hash,
                    proposal,
                    verified: false,
                    consistency: Consistency::Unknown { check: None },
                });
            }
            EventBody::Verified {
                check, consistency, ..
            } if check.obligation == obligation => {
                if selected {
                    return Ok(());
                }
                let c = current
                    .as_mut()
                    .ok_or(Error::Invalid("Verified without Pending"))?;
                c.target = event.clone();
                c.verified = true;
                c.consistency = consistency.clone();
                selected = reference == Some(hash(event)?);
            }
            EventBody::Refuted { check, .. } if check.obligation == obligation && !selected => {
                current = None
            }
            EventBody::ConsistencyRecorded {
                target,
                check,
                model,
            } => {
                if let Some(c) = &mut current
                    && *target == hash(&c.target)?
                {
                    if check.obligation != obligation
                        || event.epoch != c.target.epoch
                        || event.seq > c.pending_seq.saturating_add(WITNESS_WINDOW_EVENTS)
                    {
                        return Err(Error::Invalid("witness record context/deadline"));
                    }
                    if check.outcome == CheckOutcome::Accepted {
                        c.consistency = Consistency::Witness {
                            model: *model,
                            check: check.clone(),
                        };
                    } else if !matches!(c.consistency, Consistency::Witness { .. }) {
                        c.consistency = Consistency::Unknown {
                            check: Some(check.clone()),
                        };
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let c = current.ok_or(Error::Invalid("no witness instance"))?;
    let index = ledger.live_index()?;
    let head = ledger
        .head()?
        .ok_or(Error::Invalid("witness without head"))?;
    let eligible = |h| -> Result<bool, Error> {
        Ok(index.status(h)?.is_some_and(|s| {
            !s.retracted
                && s.epoch == head.epoch
                && matches!(
                    s.kind,
                    EntryKind::Proposal | EntryKind::Pending | EntryKind::Verified
                )
        }))
    };
    if c.target.epoch != head.epoch || !eligible(c.proposal_hash)? || !eligible(hash(&c.target)?)? {
        return Err(Error::Invalid("witness instance no longer eligible"));
    }
    Ok(c)
}

impl CertificationHandlers {
    fn recheck_witness<S: Signer>(
        &self,
        ledger: &Ledger<'_, S>,
        obligation: Hash<Obligation>,
        proposal: &EventProposal,
        consistency: Consistency,
    ) -> Result<Consistency, Error> {
        if let Consistency::Witness { model, check } = &consistency {
            if check.obligation != obligation
                || check.checker != self.registry.active(ledger, MODEL)?
            {
                return Err(Error::Invalid("witness checker/context changed"));
            }
            let (bytes, vocabulary) = context(
                ledger,
                obligation,
                proposal,
                crate::certify::Speculation::Refused,
            )?;
            let model_bytes = ledger
                .cas()
                .get(model)?
                .ok_or(Error::Invalid("missing witness model"))?;
            if ledger.cas().get(&check.certificate)?.as_ref() != Some(&model_bytes) {
                return Err(Error::Invalid("witness certificate/model mismatch"));
            }
            let result = ModelChecker::new(&vocabulary)?.check_consistency_from_source(
                &bytes,
                obligation,
                |_| ledger.cas().get(model),
                Hash::<FiniteModel>::from_bytes(*model.as_bytes()),
                &self.budget,
            )?;
            if result != CheckOutcome::Accepted {
                return Err(Error::Invalid("witness recheck not accepted"));
            }
        }
        Ok(consistency)
    }

    pub(crate) fn pending_consistency<S: Signer>(
        &self,
        ledger: &Ledger<'_, S>,
        obligation: Hash<Obligation>,
    ) -> Result<Consistency, Error> {
        let c = instance(ledger, obligation, None)?;
        if c.verified {
            return Err(Error::Invalid("consistency requires Pending"));
        }
        self.recheck_witness(ledger, obligation, &c.proposal, c.consistency)
    }

    pub(crate) fn record_witness<S: Signer>(
        &self,
        ledger: &mut Ledger<'_, S>,
        obligation: Hash<Obligation>,
        model: Hash<ModelObject>,
    ) -> Result<Value, Error> {
        let checker = self.registry.active(ledger, MODEL)?;
        let c = instance(ledger, obligation, None)?;
        let next = ledger
            .head()?
            .ok_or(Error::Invalid("witness head"))?
            .seq
            .checked_add(1)
            .ok_or(Error::Limit("witness sequence"))?;
        if next > c.pending_seq.saturating_add(WITNESS_WINDOW_EVENTS) {
            return Err(Error::Invalid("witness deadline expired"));
        }
        let (bytes, vocabulary) = context(
            ledger,
            obligation,
            &c.proposal,
            crate::certify::Speculation::Refused,
        )?;
        let model_bytes = ledger
            .cas()
            .get(&model)?
            .ok_or(Error::Invalid("missing witness model"))?;
        let outcome = ModelChecker::new(&vocabulary)?.check_consistency_from_source(
            &bytes,
            obligation,
            |_| ledger.cas().get(&model),
            Hash::<FiniteModel>::from_bytes(*model.as_bytes()),
            &self.budget,
        )?;
        let tag = match outcome {
            CheckOutcome::Accepted => 0,
            CheckOutcome::Rejected(_) => 1,
            CheckOutcome::Unknown(_) => 2,
        };
        let certificate = ledger.cas().put(&model_bytes)?;
        let check = CheckRecord {
            obligation,
            certificate,
            checker,
            budget: self.budget,
            outcome,
        };
        let event = ledger.append(EventBody::ConsistencyRecorded {
            target: hash(&c.target)?,
            check,
            model,
        })?;
        // witness-result/v1 is distinct from check-result/v1; never emits Verified.
        check_result(tag, hash(&event)?)
    }

    /// Consistência efetiva de um Verified corrente, rechecada sem reescrever assinatura.
    /// # Errors
    /// Referência/contexto/época inválidos, checker revogado, CAS ou recheck falho.
    pub fn effective_consistency<S: Signer>(
        &self,
        ledger: &Ledger<'_, S>,
        reference: Hash<Event>,
    ) -> Result<Consistency, Error> {
        let mut obligation = None;
        replay(ledger, |e| {
            if hash(e)? == reference
                && let EventBody::Verified { check, .. } = &e.body
            {
                obligation = Some(check.obligation);
            }
            Ok(())
        })?;
        let obligation = obligation.ok_or(Error::Invalid("not a Verified reference"))?;
        let c = instance(ledger, obligation, Some(reference))?;
        if !c.verified || hash(&c.target)? != reference {
            return Err(Error::Invalid("Verified instance mismatch"));
        }
        self.recheck_witness(ledger, obligation, &c.proposal, c.consistency)
    }

    /// Consulta para o consumidor da Fase8; não concede autorização nem token.
    /// # Errors
    /// Erros de armazenamento/checker/contexto propagados, nunca aceitação por padrão.
    pub fn premise_eligible<S: Signer>(
        &self,
        ledger: &Ledger<'_, S>,
        reference: Hash<Event>,
        usage: PremiseUse,
    ) -> Result<bool, Error> {
        let index = ledger.live_index()?;
        let Some(head) = index.head() else {
            return Ok(false);
        };
        if !index.is_citable(reference, head.epoch)? {
            return Ok(false);
        }
        if usage == PremiseUse::Logical {
            return Ok(true);
        }
        if index
            .status(reference)?
            .is_none_or(|s| s.kind != EntryKind::Verified)
        {
            return Ok(false);
        }
        Ok(matches!(
            self.effective_consistency(ledger, reference)?,
            Consistency::Witness { .. }
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    #[test]
    fn independent_witness_result_vectors() {
        use logos_canon::to_canon;
        let data: serde_json::Value = serde_json::from_str(include_str!(
            "../../../spec/vectors/kernel-protocol/witness-result-v1.json"
        ))
        .unwrap();
        for v in data["vectors"].as_array().unwrap() {
            let event = Hash::from_bytes(core::array::from_fn(|i| i as u8));
            let actual =
                to_canon(&check_result(v["tag"].as_u64().unwrap(), event).unwrap()).unwrap();
            let expected = v["value_hex"].as_str().unwrap();
            assert_eq!(
                actual
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
                expected
            );
        }
    }
}
