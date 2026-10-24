//! Retratação assinada e registro da cascata ATMS (Etapa 5.9, ADR0052).
//!
//! A projeção do ledger já deriva a cascata do próprio evento Retracted: um nó
//! h(O) cai quando nenhuma rota Verified restante tem premissas vivas. Este módulo
//! autoriza a retratação e grava um Invalidated por rota derrubada, com
//! `causes = [h(Retracted raiz), premissa morta imediata]`. Um crash entre os
//! appends não deixa dependente citável; `reconcile` completa os registros.

use std::collections::{HashMap, HashSet};

use logos_canon::Text;
use logos_crypto::{Hash, PublicKey, Signer, hash};
use logos_ledger::{Authority, Event, EventBody, Ledger};
use logos_obligation::{EntryKind, Obligation};

use crate::{TransportError as Error, replay::replay, transitions::Transitions};

/// Resultado de uma retratação: o evento Retracted e os Invalidated gravados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retraction {
    /// Evento Retracted confirmado.
    pub retracted: Hash<Event>,
    /// Invalidated gravados nesta chamada, em ordem de append.
    pub invalidated: Vec<Hash<Event>>,
}

/// Operação de operador fora do protocolo do agente; nunca exposta por socket.
pub struct Retractions {
    authority: PublicKey,
}

struct Route {
    obligation: Hash<Obligation>,
    used: Vec<Hash<Event>>,
}

impl Retractions {
    /// `authority` é a chave administrativa configurada pelo operador.
    #[must_use]
    pub fn new(authority: PublicKey) -> Self {
        Self { authority }
    }

    /// Retrata uma assumption (admin ou autor) ou rota Verified (admin) e grava a cascata.
    ///
    /// # Errors
    /// Alvo inexistente, já retratado ou de outro tipo; signer sem autoridade;
    /// assinatura, replay ou append.
    pub fn retract<S: Signer, A: Signer>(
        &self,
        ledger: &mut Ledger<'_, S>,
        signer: &A,
        target: Hash<Event>,
        reason: Text,
    ) -> Result<Retraction, Error> {
        let status = ledger
            .live_index()?
            .status(target)?
            .ok_or(Error::Invalid("unknown retraction target"))?;
        if status.retracted {
            return Err(Error::Invalid("target already retracted"));
        }
        let mut author = None;
        replay(ledger, |event| {
            if let EventBody::AssumptionDeclared { author: key, .. }
            | EventBody::Extracted { extractor: key, .. } = &event.body
                && hash(event)? == target
            {
                author = Some(*key);
            }
            Ok(())
        })?;
        let admin = signer.public_key() == &self.authority;
        let key = signer
            .key_id()
            .map_err(|_| Error::Invalid("authority key"))?;
        let allowed = match status.kind {
            EntryKind::Verified | EntryKind::Suspended => admin,
            EntryKind::Assumption => admin || author.as_ref() == Some(&key),
            // 6.4: fato só pelo admin (o emissor não assina eventos do kernel).
            EntryKind::Fact => admin,
            _ => {
                return Err(Error::Invalid(
                    "only Verified, Assumption or Fact can be retracted",
                ));
            }
        };
        if !allowed {
            return Err(Error::Unauthorized);
        }
        let mut body = EventBody::Retracted {
            target,
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
            .ok_or(Error::Invalid("retraction authority"))?;
        let EventBody::Retracted { authority, .. } = &mut body else {
            return Err(Error::Invalid("retraction body"));
        };
        authority.sig = signer
            .sign::<Event>(&message)
            .map_err(|_| Error::Invalid("authority signature"))?;
        let retracted = hash(&ledger.append(body)?)?;
        let invalidated = self.reconcile(ledger)?;
        Ok(Retraction {
            retracted,
            invalidated,
        })
    }

    /// Grava Invalidated faltantes para rotas derrubadas por retratação ou por
    /// recheck rejeitado (5.10): a raiz é o Retracted ou o Rechecked do alvo.
    ///
    /// Termina com a varredura de especulação (5.8, ADR0058).
    ///
    /// Idempotente: rodar de novo não grava nada. Revogação só suspende (5.10);
    /// Verified emitida por checker já revogado nasce morta e não tem registro.
    ///
    /// # Errors
    /// Replay, índice ou append.
    pub fn reconcile<S: Signer>(
        &self,
        ledger: &mut Ledger<'_, S>,
    ) -> Result<Vec<Hash<Event>>, Error> {
        let mut order = Vec::new();
        let mut routes: HashMap<Hash<Event>, Route> = HashMap::new();
        let mut roots: HashMap<Hash<Event>, Hash<Event>> = HashMap::new();
        let mut recorded: HashMap<Hash<Event>, Option<Hash<Event>>> = HashMap::new();
        let mut revoked = HashSet::new();
        let mut born_dead = HashSet::new();
        let mut facts = Vec::new();
        let cas = ledger.cas();
        replay(ledger, |event| {
            let id = hash(event)?;
            match &event.body {
                EventBody::Verified { check, used, .. } => {
                    order.push(id);
                    if revoked.contains(&check.checker) {
                        born_dead.insert(id);
                    }
                    routes.insert(
                        id,
                        Route {
                            obligation: check.obligation,
                            used: used.clone(),
                        },
                    );
                }
                EventBody::AssumptionDeclared { .. } | EventBody::Extracted { .. } => {
                    order.push(id)
                }
                // 6.4: fato morto (retratado ou de emissor revogado) é raiz da cascata.
                EventBody::FactRegistered { fact } => {
                    order.push(id);
                    let bytes = cas
                        .get(fact)?
                        .ok_or(Error::Invalid("registered fact missing from CAS"))?;
                    let decoded: logos_ledger::Fact = logos_canon::from_canon(&bytes)
                        .map_err(|_| Error::Invalid("registered fact"))?;
                    let issuer = decoded
                        .issuer
                        .key_id()
                        .map_err(|_| Error::Invalid("fact issuer key"))?;
                    facts.push((id, issuer, event.epoch));
                }
                EventBody::IssuerRevoked { issuer, since, .. } => {
                    for (fact, by, epoch) in &facts {
                        if by == issuer && epoch >= since {
                            roots.entry(*fact).or_insert(id);
                        }
                    }
                }
                EventBody::Retracted { target, .. } => {
                    roots.entry(*target).or_insert(id);
                }
                EventBody::Invalidated { claim, causes } => {
                    recorded.insert(*claim, causes.first().copied());
                }
                EventBody::CheckerRevoked { checker, .. } => {
                    revoked.insert(*checker);
                }
                EventBody::Rechecked { route, check }
                    if matches!(check.outcome, logos_core::CheckOutcome::Rejected(_)) =>
                {
                    roots.entry(*route).or_insert(id);
                }
                _ => {}
            }
            Ok(())
        })?;
        let mut pending = Vec::new();
        {
            let index = ledger.live_index()?;
            let mut dead = HashSet::new();
            for id in &order {
                let status = index
                    .status(*id)?
                    .ok_or(Error::Invalid("replayed claim missing from index"))?;
                if status.kind == EntryKind::Invalidated {
                    dead.insert(*id);
                }
            }
            // Nó h(O) vivo se alguma rota ainda é Verified.
            let mut alive_nodes = HashSet::new();
            for (id, route) in &routes {
                if !dead.contains(id) {
                    alive_nodes.insert(route.obligation);
                }
            }
            let node_dead = |premise: &Hash<Event>| match routes.get(premise) {
                Some(route) => !alive_nodes.contains(&route.obligation),
                None => dead.contains(premise),
            };
            let mut root_of: HashMap<Hash<Event>, Hash<Event>> = HashMap::new();
            for id in &order {
                if !dead.contains(id) {
                    continue;
                }
                // Raiz já gravada é definitiva: recalcular com mais mortos mudaria a cadeia.
                if let Some(root) = roots
                    .get(id)
                    .or_else(|| recorded.get(id).and_then(Option::as_ref))
                {
                    root_of.insert(*id, *root);
                    continue;
                }
                let Some(route) = routes.get(id) else {
                    continue;
                };
                if born_dead.contains(id) {
                    continue;
                }
                let cause = route
                    .used
                    .iter()
                    .filter(|p| node_dead(p))
                    .filter_map(|p| root_of.get(p).map(|root| (*p, *root)))
                    .min_by_key(|(p, _)| *p);
                let Some((premise, root)) = cause else {
                    continue;
                };
                root_of.insert(*id, root);
                if !recorded.contains_key(id) {
                    pending.push((*id, vec![root, premise]));
                }
            }
        }
        let mut transitions = Transitions::new(ledger);
        let mut out = Vec::new();
        for (id, causes) in pending {
            let claim = transitions.recover_verified(id)?;
            out.push(transitions.invalidate(claim, causes)?.receipt().hash());
        }
        // 5.8: rota morta ou base invalidada derruba especulativos pendentes.
        out.extend(crate::speculation::sweep(ledger)?);
        Ok(out)
    }
}
