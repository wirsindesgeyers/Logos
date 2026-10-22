//! Revogação de checker e recheck de certificados (Etapa 5.10, ADR0056).
//!
//! Revogar suspende as rotas cujo checker efetivo é o revogado; a projeção do
//! índice faz a suspensão transitiva. O recheck roda o certificado original de
//! cada rota em outro checker ativo e grava `Rechecked`: Accepted reativa,
//! Rejected invalida e abre a cascata (registros via `Retractions::reconcile`),
//! Unknown mantém a suspensão. Falha operacional do checker é erro, nunca Rejected.

use std::collections::HashMap;

use logos_canon::{Text, from_canon};
use logos_core::{Budget, CheckOutcome};
use logos_crypto::{Hash, PublicKey, Signer, hash, hash_raw};
use logos_ground::{GroundError, GroundLimits, clausify, ground_for_check};
use logos_ledger::{Authority, CheckRecord, Checker, Event, EventBody, Ledger};
use logos_lrat::LratChecker;
use logos_obligation::{DecodeLimits, EntryKind, vocabulary_for_check};

use crate::{
    TransportError as Error,
    registry::{CAKE, CheckerRegistry, LRAT},
    replay::replay,
    retract::Retractions,
};

/// Resultado de um recheck em lote.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RecheckReport {
    /// Eventos Rechecked gravados, em ordem.
    pub rechecked: Vec<Hash<Event>>,
    /// Quantos foram aceitos pelo checker novo.
    pub accepted: usize,
    /// Quantos foram rejeitados (rota invalidada).
    pub rejected: usize,
    /// Quantos ficaram inconclusivos (rota segue suspensa).
    pub unknown: usize,
    /// Invalidated gravados pela cascata dos rejeitados.
    pub invalidated: Vec<Hash<Event>>,
}

/// Operação de operador fora do protocolo do agente; nunca exposta por socket.
pub struct Rechecks {
    authority: PublicKey,
}

impl Rechecks {
    /// `authority` é a chave administrativa configurada pelo operador.
    #[must_use]
    pub fn new(authority: PublicKey) -> Self {
        Self { authority }
    }

    /// Grava CheckerRevoked assinado pela autoridade administrativa.
    ///
    /// # Errors
    /// Signer sem autoridade, checker nunca registrado ou já revogado, replay
    /// ou append.
    pub fn revoke<S: Signer, A: Signer>(
        &self,
        ledger: &mut Ledger<'_, S>,
        signer: &A,
        checker: Hash<Checker>,
        reason: Text,
    ) -> Result<Hash<Event>, Error> {
        if signer.public_key() != &self.authority {
            return Err(Error::Unauthorized);
        }
        let (mut registered, mut revoked) = (false, false);
        replay(ledger, |event| {
            match &event.body {
                EventBody::CheckerRegistered { checker: c, .. } if *c == checker => {
                    event.body.verify_authority(&self.authority)?;
                    registered = true;
                }
                EventBody::CheckerRevoked { checker: c, .. } if *c == checker => revoked = true,
                _ => {}
            }
            Ok(())
        })?;
        if !registered {
            return Err(Error::Invalid("checker was never registered"));
        }
        if revoked {
            return Err(Error::Invalid("checker already revoked"));
        }
        let key = signer
            .key_id()
            .map_err(|_| Error::Invalid("authority key"))?;
        let mut body = EventBody::CheckerRevoked {
            checker,
            reason,
            authority: Authority {
                key,
                sig: signer
                    .sign::<Event>(b"")
                    .map_err(|_| Error::Invalid("authority signature"))?,
            },
        };
        let message = body
            .authority_message()?
            .ok_or(Error::Invalid("revocation authority"))?;
        let EventBody::CheckerRevoked { authority, .. } = &mut body else {
            return Err(Error::Invalid("revocation body"));
        };
        authority.sig = signer
            .sign::<Event>(&message)
            .map_err(|_| Error::Invalid("authority signature"))?;
        Ok(hash(&ledger.append(body)?)?)
    }

    /// Recheca, com o checker ativo de `format`, cada rota viva cujo checker
    /// efetivo é `from`; grava um Rechecked por rota e a cascata dos rejeitados.
    ///
    /// # Errors
    /// Formato desconhecido, checker novo inativo/igual ao antigo, objeto
    /// ausente no CAS, falha operacional do checker, replay ou append.
    pub fn recheck<S: Signer>(
        &self,
        ledger: &mut Ledger<'_, S>,
        registry: &CheckerRegistry,
        from: Hash<Checker>,
        format: &str,
        budget: Budget,
    ) -> Result<RecheckReport, Error> {
        if !matches!(format, LRAT | CAKE) {
            return Err(Error::Invalid("recheck needs an LRAT checker format"));
        }
        let with = registry.active(ledger, format)?;
        if with == from {
            return Err(Error::Invalid("recheck needs a different checker"));
        }
        // Checker efetivo: o do Verified, trocado por Rechecked aceito.
        let mut order = Vec::new();
        let mut effective: HashMap<Hash<Event>, CheckRecord> = HashMap::new();
        replay(ledger, |event| {
            match &event.body {
                EventBody::Verified { check, .. } => {
                    let id = hash(event)?;
                    order.push(id);
                    effective.insert(id, check.clone());
                }
                EventBody::Rechecked { route, check }
                    if check.outcome == CheckOutcome::Accepted =>
                {
                    if let Some(record) = effective.get_mut(route) {
                        record.checker = check.checker;
                    }
                }
                _ => {}
            }
            Ok(())
        })?;
        let mut report = RecheckReport::default();
        for route in order {
            let Some(original) = effective.get(&route) else {
                continue;
            };
            if original.checker != from {
                continue;
            }
            let live = ledger
                .live_index()?
                .status(route)?
                .is_some_and(|s| matches!(s.kind, EntryKind::Verified | EntryKind::Suspended));
            if !live {
                continue;
            }
            let outcome = self.run(ledger, registry, format, original, &budget)?;
            match outcome {
                CheckOutcome::Accepted => report.accepted += 1,
                CheckOutcome::Rejected(_) => report.rejected += 1,
                CheckOutcome::Unknown(_) => report.unknown += 1,
            }
            let check = CheckRecord {
                obligation: original.obligation,
                certificate: original.certificate,
                checker: with,
                budget,
                outcome,
            };
            report.rechecked.push(hash(
                &ledger.append(EventBody::Rechecked { route, check })?,
            )?);
        }
        report.invalidated = Retractions::new(self.authority.clone()).reconcile(ledger)?;
        Ok(report)
    }

    fn run<S: Signer>(
        &self,
        ledger: &Ledger<'_, S>,
        registry: &CheckerRegistry,
        format: &str,
        original: &CheckRecord,
        budget: &Budget,
    ) -> Result<CheckOutcome, Error> {
        let cas = ledger.cas();
        let bytes = cas
            .get(&original.obligation)?
            .ok_or(Error::Invalid("missing obligation"))?;
        let hv = vocabulary_for_check(&bytes, original.obligation, DecodeLimits::default())
            .map_err(|_| Error::Invalid("obligation vocabulary"))?;
        let vocabulary = from_canon(&cas.get(&hv)?.ok_or(Error::Invalid("missing vocabulary"))?)?;
        let proof = cas
            .get(&original.certificate)?
            .ok_or(Error::Invalid("missing certificate"))?;
        let derived = ground_for_check(
            &bytes,
            original.obligation,
            &vocabulary,
            DecodeLimits::default(),
            GroundLimits::default(),
        )
        .and_then(|ground| clausify(&ground));
        let cnf = match derived {
            Ok((cnf, _)) => cnf,
            Err(GroundError::Unknown(reason)) => return Ok(CheckOutcome::Unknown(reason)),
            Err(_) => return Err(Error::Invalid("route obligation outside layer 1")),
        };
        let h_cnf = hash_raw(cnf.dimacs());
        if format == CAKE {
            registry
                .cake()?
                .check_bytes(cnf.dimacs(), h_cnf, &proof, hash_raw(&proof), budget)
                .map_err(|_| Error::Invalid("cake checker operational or identity failure"))
        } else {
            Ok(LratChecker::new()?.check_lrat(cnf.dimacs(), h_cnf, &proof, budget))
        }
    }
}
