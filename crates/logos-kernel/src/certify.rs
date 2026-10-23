//! CERTIFY/REFUTE: somente resultados de checkers ativos viram transições.

use logos_air::{Limits, SortDomain, Vocabulary};
use logos_canon::{Array, Canon, Text, Value, to_canon};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::{Hash, Signer, hash_raw};
use logos_ground::{GroundError, GroundLimits, clausify, ground_for_check};
use logos_ledger::{Certificate, CheckRecord, EventProposal, Ledger, ModelObject};
use logos_lrat::{LratChecker, UsedError, used_premises};
use logos_model::{FiniteModel, ModelChecker};
use logos_obligation::{DecodeLimits, Obligation, PremiseRef, Proposal, generate_speculative};

use crate::{
    TransportError as Error,
    registry::{CAKE, CheckerRegistry, LRAT, MODEL},
    server::{Handlers, Peer},
    transitions::Transitions,
};

/// Configuração confiada ao operador, nunca campos do protocolo.
pub struct CertificationHandlers {
    pub(crate) registry: CheckerRegistry,
    pub(crate) budget: Budget,
}

impl CertificationHandlers {
    /// Orçamento por etapa de checker/projeção; pré-processamento tem limites próprios.
    #[must_use]
    pub fn new(registry: CheckerRegistry, budget: Budget) -> Self {
        Self { registry, budget }
    }

    fn finish<S: Signer>(
        &self,
        transitions: &mut Transitions<'_, '_, S>,
        claim: crate::Claim<crate::state::Pending>,
        check: CheckRecord,
        cited: Vec<Hash<logos_ledger::Event>>,
        used: Vec<Hash<logos_ledger::Event>>,
        model: Option<Hash<ModelObject>>,
    ) -> Result<Value, Error> {
        let (tag, event) = match &check.outcome {
            CheckOutcome::Accepted => match model {
                Some(model) => (1, transitions.refute(claim, check, model)?.receipt().hash()),
                None => {
                    let consistency =
                        self.pending_consistency(transitions.ledger(), check.obligation)?;
                    (
                        0,
                        transitions
                            .accept(claim, check, cited, used, consistency)?
                            .receipt()
                            .hash(),
                    )
                }
            },
            CheckOutcome::Rejected(_) => (2, transitions.reject(claim, check)?.receipt().hash()),
            CheckOutcome::Unknown(_) => (
                3,
                transitions
                    .unknown(claim, check, Text::new(std::env::consts::ARCH)?)?
                    .receipt()
                    .hash(),
            ),
        };
        check_result(tag, event)
    }
}

pub(crate) fn check_result(tag: u64, event: Hash<logos_ledger::Event>) -> Result<Value, Error> {
    Ok(Value::Array(Array::new(vec![
        Value::Unsigned(1),
        Value::Unsigned(tag),
        event.to_value()?,
    ])?))
}

pub(crate) fn context<S: Signer>(
    ledger: &Ledger<'_, S>,
    obligation: Hash<Obligation>,
    proposal: &EventProposal,
    speculation: Speculation,
) -> Result<(Vec<u8>, Vocabulary), Error> {
    let bytes = ledger
        .cas()
        .get(&obligation)?
        .ok_or(Error::Invalid("missing obligation"))?;
    let vocab_bytes = ledger
        .cas()
        .get(&proposal.vocabulary)?
        .ok_or(Error::Invalid("missing vocabulary"))?;
    let vocabulary = Vocabulary::decode_with_limits(&vocab_bytes, Limits::default())
        .map_err(|_| Error::Invalid("invalid obligation vocabulary"))?;
    // Fact/Policy DomainSource ainda são opacos; nem o hash nem um cast os autentica.
    if vocabulary
        .sorts()
        .iter()
        .any(|s| matches!(s.domain, SortDomain::Enumerated { .. }))
    {
        return Err(Error::Invalid("unresolved enumeration source"));
    }
    if !proposal.speculative_on.is_empty() && speculation == Speculation::Refused {
        return Err(Error::Invalid("speculative consistency witness"));
    }
    let references: Vec<_> = proposal
        .cites
        .iter()
        .map(|h| PremiseRef::from_event_hash(*h.as_bytes()))
        .collect();
    let speculative: Vec<_> = proposal
        .speculative_on
        .iter()
        .map(|h| PremiseRef::from_event_hash(*h))
        .collect();
    let index = ledger.live_index()?;
    let epoch = ledger
        .head()?
        .ok_or(Error::Invalid("Pending without head"))?
        .epoch;
    // Mesmo escopo do PROPOSE (5.8): citações das bases, fechadas transitivamente.
    let scope_cites = if speculative.is_empty() {
        proposal.cites.clone()
    } else {
        crate::speculation::History::read(ledger)?.scope_cites(proposal)
    };
    let scope = crate::scope::for_proposal(&index, &scope_cites, epoch)?;
    let mut snapshot_refs = references.clone();
    snapshot_refs.extend_from_slice(&speculative);
    let view = index.snapshot(ledger.cas(), &snapshot_refs, &scope)?;
    // Base invalidada deixa de ser Pending: a regeneração falha e nada é gravado.
    let regenerated = generate_speculative(
        Proposal {
            vocabulary: &vocabulary,
            goal: &proposal.goal,
            cites: &references,
        },
        &speculative,
        &view,
    )
    .map_err(|_| Error::Invalid("Pending context no longer eligible"))?;
    if to_canon(&regenerated)? != bytes || hash_raw::<Obligation>(&bytes) != obligation {
        return Err(Error::Invalid(
            "Pending obligation differs from regenerated proposal",
        ));
    }
    Ok((bytes, vocabulary))
}

/// Se a regeneração aceita bases especulativas (CERTIFY/REFUTE) ou não (WITNESS).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Speculation {
    Allowed,
    Refused,
}

impl<S: Signer> Handlers<S> for CertificationHandlers {
    fn propose(
        &self,
        ledger: &mut Ledger<'_, S>,
        peer: &Peer,
        proposal: EventProposal,
    ) -> Result<Value, Error> {
        let _ = peer;
        crate::propose::create(ledger, proposal, true)
    }

    fn certify(
        &self,
        ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        obligation: Hash<Obligation>,
        format: Text,
        certificate: Hash<Certificate>,
    ) -> Result<Value, Error> {
        if !matches!(format.as_str(), LRAT | CAKE) {
            return Err(Error::Invalid("unsupported proof format"));
        }
        let checker = self.registry.active(ledger, format.as_str())?;
        // Used é derivado por Rust, que também precisa estar ativo no modo cake.
        self.registry.active(ledger, LRAT)?;
        let mut transitions = Transitions::new(ledger);
        let (claim, proposal) = transitions.recover_pending(obligation)?;
        let ledger = transitions.ledger();
        let (bytes, vocabulary) = context(ledger, obligation, &proposal, Speculation::Allowed)?;
        // 5.8: especulativo só é checado com toda base Verified ativa; senão nada é gravado.
        let settled = if proposal.speculative_on.is_empty() {
            Vec::new()
        } else {
            let history = crate::speculation::History::read(ledger)?;
            crate::speculation::settle_bases(&history, &ledger.live_index()?, &proposal)?
        };
        let proof = ledger
            .cas()
            .get(&certificate)?
            .ok_or(Error::Invalid("missing certificate"))?;
        let mut check = CheckRecord {
            obligation,
            certificate,
            checker,
            budget: self.budget,
            outcome: CheckOutcome::Accepted,
        };
        let mut used = Vec::new();
        let derived = ground_for_check(
            &bytes,
            obligation,
            &vocabulary,
            DecodeLimits::default(),
            GroundLimits::default(),
        )
        .and_then(|ground| clausify(&ground));
        match derived {
            Err(GroundError::Unknown(reason)) => check.outcome = CheckOutcome::Unknown(reason),
            Err(_) => return Err(Error::Invalid("obligation outside layer 1")),
            Ok((cnf, map)) => {
                let report = LratChecker::new()?.check_with_used(
                    cnf.dimacs(),
                    hash_raw(cnf.dimacs()),
                    &proof,
                    &self.budget,
                );
                check.outcome = report.outcome().clone();
                if format.as_str() == CAKE {
                    let cake_outcome = self
                        .registry
                        .cake()?
                        .check_bytes(
                            cnf.dimacs(),
                            hash_raw(cnf.dimacs()),
                            &proof,
                            hash_raw(&proof),
                            &self.budget,
                        )
                        .map_err(|_| {
                            Error::Invalid("cake checker operational or identity failure")
                        })?;
                    if cake_outcome != CheckOutcome::Accepted {
                        check.outcome = cake_outcome;
                    }
                }
                if check.outcome == CheckOutcome::Accepted {
                    match used_premises(&report, &map, obligation, &self.budget) {
                        Ok(projected) => {
                            // Base especulativa usada vira a rota Verified que a resolve.
                            used = projected
                                .premises()
                                .iter()
                                .map(|r| {
                                    let id = Hash::from_bytes(*r.event_hash());
                                    settled
                                        .iter()
                                        .find(|(base, _)| *base == id)
                                        .map_or(id, |(_, route)| *route)
                                })
                                .collect();
                            used.sort_unstable();
                            used.dedup();
                        }
                        Err(UsedError::Unknown(reason)) => {
                            check.outcome = CheckOutcome::Unknown(reason)
                        }
                        Err(_) => return Err(Error::Invalid("used projection context")),
                    }
                }
            }
        }
        let routes: Vec<_> = settled.iter().map(|(_, route)| *route).collect();
        // 5.8: rotas que resolvem bases especulativas também são citadas.
        let claim = if routes.is_empty() {
            claim
        } else {
            transitions.settle(claim, &routes)?
        };
        let mut cited = proposal.cites;
        cited.extend_from_slice(&routes);
        cited.sort_unstable();
        cited.dedup();
        self.finish(&mut transitions, claim, check, cited, used, None)
    }

    fn witness(
        &self,
        ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        obligation: Hash<Obligation>,
        model: Hash<ModelObject>,
    ) -> Result<Value, Error> {
        self.record_witness(ledger, obligation, model)
    }

    fn assume(
        &self,
        ledger: &mut Ledger<'_, S>,
        _peer: &Peer,
        vocabulary: Hash<Vocabulary>,
        formula: logos_air::Formula,
        scope: Text,
    ) -> Result<Value, Error> {
        crate::assume::declare(ledger, vocabulary, formula, scope)
    }

    fn refute(
        &self,
        writer: &mut Ledger<'_, S>,
        _peer: &Peer,
        obligation: Hash<Obligation>,
        model: Hash<ModelObject>,
    ) -> Result<Value, Error> {
        let checker = self.registry.active(writer, MODEL)?;
        let mut transitions = Transitions::new(writer);
        let (claim, proposal) = transitions.recover_pending(obligation)?;
        let ledger = transitions.ledger();
        let (bytes, vocabulary) = context(ledger, obligation, &proposal, Speculation::Allowed)?;
        let model_bytes = ledger
            .cas()
            .get(&model)?
            .ok_or(Error::Invalid("missing countermodel"))?;
        let certificate = ledger.cas().put::<Certificate>(&model_bytes)?;
        let model_hash = Hash::<FiniteModel>::from_bytes(*model.as_bytes());
        let outcome = ModelChecker::new(&vocabulary)?.check_refutation_from_source(
            &bytes,
            obligation,
            |_| ledger.cas().get(&model),
            model_hash,
            &self.budget,
        )?;
        let check = CheckRecord {
            obligation,
            certificate,
            checker,
            budget: self.budget,
            outcome,
        };
        let result = self.finish(
            &mut transitions,
            claim,
            check,
            proposal.cites,
            Vec::new(),
            Some(model),
        )?;
        // 5.8: refutação derruba especulativos pendentes sobre esta obrigação.
        crate::speculation::sweep(writer)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    #[test]
    fn independent_check_result_vectors_fix_every_outcome() {
        let root: serde_json::Value = serde_json::from_str(include_str!(
            "../../../spec/vectors/kernel-protocol/check-result-v1.json"
        ))
        .unwrap();
        for v in root["vectors"].as_array().unwrap() {
            let unhex = |s: &str| -> Vec<u8> {
                s.as_bytes()
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
                    .collect()
            };
            let digest: [u8; 32] = unhex(v["event_hash_hex"].as_str().unwrap())
                .try_into()
                .unwrap();
            let value = check_result(v["tag"].as_u64().unwrap(), Hash::from_bytes(digest)).unwrap();
            assert_eq!(
                to_canon(&value).unwrap(),
                unhex(v["value_hex"].as_str().unwrap())
            );
        }
    }
}
