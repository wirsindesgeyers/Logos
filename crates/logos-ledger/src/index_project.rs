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

/// Premissa viva: assumption não morta, ou nó h(O) com alguma rota Verified.
fn premise_alive(connection: &Connection, premise: &[u8]) -> Result<bool, LedgerError> {
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
fn invalidate(connection: &Connection, killed: &[Vec<u8>]) -> Result<(), LedgerError> {
    let mut work = Vec::new();
    for id in killed {
        if let Some(node) = node_of(connection, id)? {
            kill(connection, id)?;
            work.push(node);
        }
    }
    let mut out = std::collections::HashSet::new();
    while let Some(node) = work.pop() {
        if out.contains(&node) {
            continue;
        }
        let members = match &node {
            Node::Obligation(h) => {
                let routes = ids(
                    connection,
                    "SELECT id FROM claims WHERE h_o=?1 AND checker IS NOT NULL AND status=2",
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
                "SELECT claim FROM deps JOIN claims ON claims.id=deps.claim WHERE premise=?1 AND status=2",
                member,
            )? {
                if let Some(next) = node_of(connection, &dependent)? {
                    work.push(next);
                }
            }
        }
    }
    Ok(())
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
            invalidate(connection, &[target.as_bytes().to_vec()])?;
            if matches!(event.body, EventBody::Retracted { .. }) {
                connection
                    .prepare("UPDATE claims SET retracted=1 WHERE id=?1")?
                    .execute([target.as_bytes().as_slice()])?;
            }
            return Ok(());
        }
        EventBody::CheckerRevoked { checker, .. } => {
            let routes = ids(
                connection,
                "SELECT id FROM claims WHERE checker=?1 AND status=2",
                checker.as_bytes(),
            )?;
            invalidate(connection, &routes)?;
            connection
                .prepare("INSERT OR IGNORE INTO revoked(checker) VALUES (?1)")?
                .execute([checker.as_bytes().as_slice()])?;
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
    Ok(())
}
