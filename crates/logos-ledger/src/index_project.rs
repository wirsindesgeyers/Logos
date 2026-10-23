//! Projeção de eventos já autenticados; dependências são exclusivamente used.

use logos_canon::to_canon;
use rusqlite::{Connection, OptionalExtension, params};

use crate::{Event, EventBody, LedgerError, LedgerHead};

// Mesmo layout/codebook publicado em spec/ledger-index.md.
pub(crate) const PROPOSAL: i64 = 0;
pub(crate) const PENDING: i64 = 1;
pub(crate) const VERIFIED: i64 = 2;
pub(crate) const REFUTED: i64 = 3;
pub(crate) const UNKNOWN: i64 = 4;
pub(crate) const ASSUMPTION: i64 = 5;
pub(crate) const INVALIDATED: i64 = 6;
pub(crate) const SUSPENDED: i64 = 7;

/// Nó ATMS de uma linha: rotas Verified (h_o e checker presentes) pertencem ao
/// nó h(O); as demais linhas (assumptions, propostas) são nós de si mesmas.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Node {
    Obligation(Vec<u8>),
    Single(Vec<u8>),
}

fn node_of(connection: &Connection, id: &[u8]) -> Result<Option<Node>, LedgerError> {
    Ok(connection
        .prepare("SELECT h_o, checker IS NOT NULL FROM claims WHERE id=?1")?
        .query_row([id], |row| {
            Ok((row.get::<_, Option<Vec<u8>>>(0)?, row.get::<_, bool>(1)?))
        })
        .optional()?
        .map(|(obligation, route)| match obligation {
            Some(h) if route => Node::Obligation(h),
            _ => Node::Single(id.to_vec()),
        }))
}

/// Premissa viva: assumption não morta, ou nó h(O) com alguma rota Verified ou
/// Suspended (suspensão por revogação não é morte, 5.10).
fn premise_alive(connection: &Connection, premise: &[u8]) -> Result<bool, LedgerError> {
    Ok(match node_of(connection, premise)? {
        Some(Node::Obligation(h)) => connection
            .prepare(
                "SELECT 1 FROM claims WHERE h_o=?1 AND checker IS NOT NULL AND status IN (2,7)",
            )?
            .query_row([h], |_| Ok(()))
            .optional()?
            .is_some(),
        Some(Node::Single(id)) => connection
            .prepare("SELECT 1 FROM claims WHERE id=?1 AND status IN (2,5)")?
            .query_row([id], |_| Ok(()))
            .optional()?
            .is_some(),
        None => false,
    })
}

/// Premissa ativa (citável): assumption viva ou nó h(O) com rota Verified ativa.
fn premise_active(connection: &Connection, premise: &[u8]) -> Result<bool, LedgerError> {
    Ok(match node_of(connection, premise)? {
        Some(Node::Obligation(h)) => connection
            .prepare("SELECT 1 FROM claims WHERE h_o=?1 AND checker IS NOT NULL AND status=2")?
            .query_row([h], |_| Ok(()))
            .optional()?
            .is_some(),
        Some(Node::Single(id)) => connection
            .prepare("SELECT 1 FROM claims WHERE id=?1 AND status IN (2,5)")?
            .query_row([id], |_| Ok(()))
            .optional()?
            .is_some(),
        None => false,
    })
}

/// Recalcula atividade dos nós h(O) dados e propaga aos dependentes (5.10).
///
/// Um nó vivo está ativo se alguma rota não morta, de checker não revogado, tem
/// todas as premissas used ativas; senão suas rotas vivas ficam Suspended (não
/// citáveis, mas não mortas). Ativo: todas as rotas vivas voltam a Verified e
/// citáveis na época de emissão. Cada evento move atividade num só sentido e o
/// grafo é acíclico, então a worklist termina no ponto fixo.
fn refresh(connection: &Connection, seeds: Vec<Vec<u8>>) -> Result<(), LedgerError> {
    let mut work = seeds;
    while let Some(h) = work.pop() {
        let routes = ids(
            connection,
            "SELECT id FROM claims WHERE h_o=?1 AND checker IS NOT NULL AND status IN (2,7)",
            &h,
        )?;
        let mut active = false;
        for route in &routes {
            let revoked = connection
                .prepare("SELECT 1 FROM claims JOIN revoked ON revoked.checker=claims.checker WHERE claims.id=?1")?
                .query_row([route], |_| Ok(()))
                .optional()?
                .is_some();
            if revoked {
                continue;
            }
            let mut all = true;
            for premise in ids(connection, "SELECT premise FROM deps WHERE claim=?1", route)? {
                if !premise_active(connection, &premise)? {
                    all = false;
                    break;
                }
            }
            if all {
                active = true;
                break;
            }
        }
        let (from, to) = if active {
            (SUSPENDED, VERIFIED)
        } else {
            (VERIFIED, SUSPENDED)
        };
        let mut changed = Vec::new();
        for route in &routes {
            let updated = connection
                .prepare("UPDATE claims SET status=?2 WHERE id=?1 AND status=?3")?
                .execute(params![route, to, from])?;
            if updated == 0 {
                continue;
            }
            if active {
                connection
                    .prepare("INSERT OR IGNORE INTO citable(ref,epoch) SELECT id,epoch FROM claims WHERE id=?1")?
                    .execute([route])?;
            } else {
                connection
                    .prepare("DELETE FROM citable WHERE ref=?1")?
                    .execute([route])?;
            }
            changed.push(route.clone());
        }
        for route in &changed {
            for dependent in ids(
                connection,
                "SELECT h_o FROM deps JOIN claims ON claims.id=deps.claim WHERE premise=?1 AND status IN (2,7) AND checker IS NOT NULL",
                route,
            )? {
                if !work.contains(&dependent) {
                    work.push(dependent);
                }
            }
        }
    }
    Ok(())
}

fn ids(connection: &Connection, sql: &str, key: &[u8]) -> Result<Vec<Vec<u8>>, LedgerError> {
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map([key], |row| row.get::<_, Vec<u8>>(0))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

fn kill(connection: &Connection, id: &[u8]) -> Result<(), LedgerError> {
    connection
        .prepare("UPDATE claims SET status=6 WHERE id=?1")?
        .execute([id])?;
    connection
        .prepare("DELETE FROM citable WHERE ref=?1")?
        .execute([id])?;
    Ok(())
}

/// Mata as linhas dadas e propaga até ponto fixo (ADR0052, spec/ledger-index.md §3).
///
/// Um nó h(O) cai quando nenhuma rota Verified restante tem todas as premissas
/// used vivas; então todas as suas rotas viram Invalidated (terminal) e seus
/// dependentes são reavaliados. Rotas independentes mantêm o nó e os dependentes.
fn invalidate(connection: &Connection, killed: &[Vec<u8>]) -> Result<Vec<Vec<u8>>, LedgerError> {
    let mut work = Vec::new();
    for id in killed {
        if let Some(node) = node_of(connection, id)? {
            kill(connection, id)?;
            work.push(node);
        }
    }
    let mut out = std::collections::HashSet::new();
    let mut touched = Vec::new();
    while let Some(node) = work.pop() {
        if out.contains(&node) {
            continue;
        }
        let members = match &node {
            Node::Obligation(h) => {
                let routes = ids(
                    connection,
                    "SELECT id FROM claims WHERE h_o=?1 AND checker IS NOT NULL AND status IN (2,7)",
                    h,
                )?;
                let mut supported = false;
                for route in &routes {
                    let premises =
                        ids(connection, "SELECT premise FROM deps WHERE claim=?1", route)?;
                    let mut all = true;
                    for premise in &premises {
                        if !premise_alive(connection, premise)? {
                            all = false;
                            break;
                        }
                    }
                    if all {
                        supported = true;
                        break;
                    }
                }
                if supported {
                    // Sobrevive, mas pode ter perdido a rota ativa: reavaliar.
                    touched.push(h.clone());
                    continue;
                }
                for route in &routes {
                    kill(connection, route)?;
                }
                ids(
                    connection,
                    "SELECT id FROM claims WHERE h_o=?1 AND checker IS NOT NULL",
                    h,
                )?
            }
            Node::Single(id) => {
                if premise_alive(connection, id)? {
                    continue;
                }
                vec![id.clone()]
            }
        };
        out.insert(node);
        for member in &members {
            for dependent in ids(
                connection,
                "SELECT claim FROM deps JOIN claims ON claims.id=deps.claim WHERE premise=?1 AND status IN (2,7)",
                member,
            )? {
                if let Some(next) = node_of(connection, &dependent)? {
                    work.push(next);
                }
            }
        }
    }
    Ok(touched)
}

pub(crate) fn apply(
    connection: &Connection,
    event: &Event,
    head: LedgerHead,
) -> Result<(), LedgerError> {
    let id = head.hash.as_bytes().as_slice();
    let epoch = event.epoch.to_be_bytes();
    let mut obligation = None;
    let mut vocabulary = None;
    let mut formula = None;
    let mut scope = None;
    let mut checker = None;
    let mut used: &[logos_crypto::Hash<Event>] = &[];
    let status = match &event.body {
        EventBody::Proposed { proposal, .. } => {
            vocabulary = Some(proposal.vocabulary.as_bytes().as_slice());
            formula = Some(to_canon(&proposal.goal)?);
            PROPOSAL
        }
        EventBody::ObligationCreated { obligation: h, .. } => {
            obligation = Some(h.as_bytes().as_slice());
            PENDING
        }
        EventBody::Verified {
            check,
            used: references,
            ..
        } => {
            obligation = Some(check.obligation.as_bytes().as_slice());
            checker = Some(check.checker.as_bytes().as_slice());
            used = references;
            VERIFIED
        }
        EventBody::Refuted { check, .. } => {
            obligation = Some(check.obligation.as_bytes().as_slice());
            checker = Some(check.checker.as_bytes().as_slice());
            REFUTED
        }
        EventBody::CertificateRejected { check } => {
            obligation = Some(check.obligation.as_bytes().as_slice());
            PENDING
        }
        EventBody::UnknownRecorded { check, .. } => {
            obligation = Some(check.obligation.as_bytes().as_slice());
            checker = Some(check.checker.as_bytes().as_slice());
            UNKNOWN
        }
        EventBody::AssumptionDeclared {
            vocabulary: h,
            formula: value,
            scope: label,
            ..
        } => {
            vocabulary = Some(h.as_bytes().as_slice());
            formula = Some(to_canon(value)?);
            scope = Some(label.as_str());
            ASSUMPTION
        }
        EventBody::Retracted { target, .. } | EventBody::Invalidated { claim: target, .. } => {
            let exists = connection
                .prepare("SELECT 1 FROM claims WHERE id=?1")?
                .query_row([target.as_bytes().as_slice()], |_| Ok(()))
                .optional()?
                .is_some();
            if !exists {
                return Err(LedgerError::Invalid(
                    "invalidation target is not an earlier claim",
                ));
            }
            let touched = invalidate(connection, &[target.as_bytes().to_vec()])?;
            refresh(connection, touched)?;
            if matches!(event.body, EventBody::Retracted { .. }) {
                connection
                    .prepare("UPDATE claims SET retracted=1 WHERE id=?1")?
                    .execute([target.as_bytes().as_slice()])?;
            }
            return Ok(());
        }
        EventBody::CheckerRevoked { checker, .. } => {
            // 5.10: revogar suspende (não mata) as rotas desse checker efetivo.
            connection
                .prepare("INSERT OR IGNORE INTO revoked(checker) VALUES (?1)")?
                .execute([checker.as_bytes().as_slice()])?;
            let nodes = ids(
                connection,
                "SELECT DISTINCT h_o FROM claims WHERE checker=?1 AND status IN (2,7)",
                checker.as_bytes(),
            )?;
            refresh(connection, nodes)?;
            return Ok(());
        }
        EventBody::Rechecked { route, check } => {
            let row = connection
                .prepare("SELECT h_o, status FROM claims WHERE id=?1 AND checker IS NOT NULL")?
                .query_row([route.as_bytes().as_slice()], |r| {
                    Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?))
                })
                .optional()?;
            let Some((node, status)) = row else {
                return Err(LedgerError::Invalid(
                    "recheck target is not a Verified route",
                ));
            };
            if status != VERIFIED && status != SUSPENDED {
                return Err(LedgerError::Invalid("recheck target is not a live route"));
            }
            if node.as_slice() != check.obligation.as_bytes() {
                return Err(LedgerError::Invalid(
                    "recheck obligation differs from route",
                ));
            }
            match check.outcome {
                logos_core::CheckOutcome::Accepted => {
                    let revoked = connection
                        .prepare("SELECT 1 FROM revoked WHERE checker=?1")?
                        .query_row([check.checker.as_bytes().as_slice()], |_| Ok(()))
                        .optional()?
                        .is_some();
                    if revoked {
                        return Err(LedgerError::Invalid("recheck by a revoked checker"));
                    }
                    connection
                        .prepare("UPDATE claims SET checker=?2 WHERE id=?1")?
                        .execute(params![
                            route.as_bytes().as_slice(),
                            check.checker.as_bytes().as_slice()
                        ])?;
                    refresh(connection, vec![node])?;
                }
                logos_core::CheckOutcome::Rejected(_) => {
                    let touched = invalidate(connection, &[route.as_bytes().to_vec()])?;
                    refresh(connection, touched)?;
                }
                logos_core::CheckOutcome::Unknown(_) => {}
            }
            return Ok(());
        }
        _ => return Ok(()),
    };
    for premise in used {
        let eligible = connection
            .prepare("SELECT 1 FROM citable WHERE ref=?1 AND epoch=?2")?
            .query_row(
                params![premise.as_bytes().as_slice(), epoch.as_slice()],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !eligible {
            return Err(LedgerError::Invalid(
                "used premise is not an earlier citable claim in this epoch",
            ));
        }
    }
    let revoked = checker.is_some()
        && connection
            .prepare("SELECT 1 FROM revoked WHERE checker=?1")?
            .query_row([checker], |_| Ok(()))
            .optional()?
            .is_some();
    let status = if status == VERIFIED && revoked {
        INVALIDATED
    } else {
        status
    };
    connection.prepare("INSERT INTO claims(id,h_o,status,epoch,retracted,vocab,formula,scope,checker) VALUES (?1,?2,?3,?4,0,?5,?6,?7,?8)")?
        .execute(params![id, obligation, status, epoch.as_slice(), vocabulary, formula, scope, checker])?;
    for premise in used {
        connection
            .prepare("INSERT INTO deps(claim,premise) VALUES (?1,?2)")?
            .execute(params![id, premise.as_bytes().as_slice()])?;
    }
    if status == ASSUMPTION || status == VERIFIED {
        connection
            .prepare("INSERT INTO citable(ref,epoch) VALUES (?1,?2)")?
            .execute(params![id, epoch.as_slice()])?;
    }
    if status == VERIFIED
        && let Some(h) = obligation
    {
        // Rota nova ativa reativa rotas suspensas do mesmo nó e seus dependentes. Ela
        // nasce ativa (premissas citáveis acima, checker não revogado), então só muda
        // o nó se ele tiver rota Suspended; sem esse filtro cada Verified relia todas
        // as rotas do nó (O(n²) com 1M rotas de uma obrigação; ADR0057).
        let suspended = connection
            .prepare("SELECT 1 FROM claims WHERE h_o=?1 AND status=7 LIMIT 1")?
            .query_row([h], |_| Ok(()))
            .optional()?
            .is_some();
        if suspended {
            refresh(connection, vec![h.to_vec()])?;
        }
    }
    Ok(())
}
