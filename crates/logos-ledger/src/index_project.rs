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

const AFFECTED: &str = "WITH RECURSIVE affected(id) AS (SELECT id FROM claims WHERE id=?1 OR (checker=?2 AND status=2) UNION SELECT deps.claim FROM deps JOIN affected ON deps.premise=affected.id)";

fn invalidate(
    connection: &Connection,
    target: Option<&[u8]>,
    checker: Option<&[u8]>,
) -> Result<(), LedgerError> {
    connection
        .prepare(&format!(
            "{AFFECTED} DELETE FROM citable WHERE ref IN (SELECT id FROM affected)"
        ))?
        .execute(params![target, checker])?;
    connection
        .prepare(&format!(
            "{AFFECTED} UPDATE claims SET status=6 WHERE id IN (SELECT id FROM affected)"
        ))?
        .execute(params![target, checker])?;
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
            invalidate(connection, Some(target.as_bytes()), None)?;
            if matches!(event.body, EventBody::Retracted { .. }) {
                connection
                    .prepare("UPDATE claims SET retracted=1 WHERE id=?1")?
                    .execute([target.as_bytes().as_slice()])?;
            }
            return Ok(());
        }
        EventBody::CheckerRevoked { checker, .. } => {
            invalidate(connection, None, Some(checker.as_bytes()))?;
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
