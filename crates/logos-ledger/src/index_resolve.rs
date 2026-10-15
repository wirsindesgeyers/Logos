//! Objetos de premissa: bytes CAS conferidos, contexto e sort-check reais.

use std::collections::BTreeMap;
use std::sync::Arc;

use logos_air::{Formula, Limits, Vocabulary};
use logos_crypto::Hash;
use logos_obligation::{DecodeLimits, Obligation, decode_for_check, vocabulary_for_check};
use rusqlite::{Connection, OptionalExtension};

use crate::{Cas, ClaimStatus, LedgerError};

pub(crate) struct ResolvedEntry {
    pub(crate) status: ClaimStatus,
    pub(crate) vocabulary: Arc<Vocabulary>,
    pub(crate) formula: Arc<Formula>,
}

#[derive(Default)]
pub(crate) struct Resolver {
    vocabularies: BTreeMap<Hash<Vocabulary>, Arc<Vocabulary>>,
    obligations: BTreeMap<Hash<Obligation>, (Arc<Vocabulary>, Arc<Formula>)>,
}

impl Resolver {
    fn vocabulary(
        &mut self,
        cas: &Cas,
        expected: Hash<Vocabulary>,
    ) -> Result<Arc<Vocabulary>, LedgerError> {
        if let Some(value) = self.vocabularies.get(&expected) {
            return Ok(Arc::clone(value));
        }
        let bytes = cas
            .get(&expected)?
            .ok_or(LedgerError::Invalid("vocabulary missing from CAS"))?;
        let value = Arc::new(
            Vocabulary::decode_with_limits(&bytes, Limits::default())
                .map_err(|_| LedgerError::Invalid("invalid CAS vocabulary"))?,
        );
        self.vocabularies.insert(expected, Arc::clone(&value));
        Ok(value)
    }

    pub(crate) fn resolve(
        &mut self,
        connection: &Connection,
        cas: &Cas,
        id: &[u8; 32],
        status: ClaimStatus,
    ) -> Result<ResolvedEntry, LedgerError> {
        let fields = connection
            .prepare("SELECT h_o,vocab,formula FROM claims WHERE id=?1")?
            .query_row([id.as_slice()], |r| {
                Ok((
                    r.get::<_, Option<Vec<u8>>>(0)?,
                    r.get::<_, Option<Vec<u8>>>(1)?,
                    r.get::<_, Option<Vec<u8>>>(2)?,
                ))
            })
            .optional()?
            .ok_or(LedgerError::Invalid(
                "claim disappeared from private snapshot",
            ))?;
        let (vocabulary, formula) = if let (Some(vocabulary), Some(formula)) = (fields.1, fields.2)
        {
            let digest = <[u8; 32]>::try_from(vocabulary.as_slice())
                .map_err(|_| LedgerError::Invalid("vocabulary digest length"))?;
            let vocabulary = self.vocabulary(cas, Hash::from_bytes(digest))?;
            let formula: Formula = logos_canon::from_canon(&formula)?;
            logos_air::check(&vocabulary, &formula)
                .map_err(|_| LedgerError::Invalid("claim formula sort"))?;
            (vocabulary, Arc::new(formula))
        } else if let Some(obligation) = fields.0 {
            let digest = <[u8; 32]>::try_from(obligation.as_slice())
                .map_err(|_| LedgerError::Invalid("obligation digest length"))?;
            let expected = Hash::<Obligation>::from_bytes(digest);
            if let Some((vocabulary, formula)) = self.obligations.get(&expected) {
                (Arc::clone(vocabulary), Arc::clone(formula))
            } else {
                let bytes = cas
                    .get(&expected)?
                    .ok_or(LedgerError::Invalid("obligation missing from CAS"))?;
                let vocabulary_hash =
                    vocabulary_for_check(&bytes, expected, DecodeLimits::default())?;
                let vocabulary = self.vocabulary(cas, vocabulary_hash)?;
                let checked =
                    decode_for_check(&bytes, expected, &vocabulary, DecodeLimits::default())?;
                let formula = Arc::new(checked.goal().clone());
                self.obligations
                    .insert(expected, (Arc::clone(&vocabulary), Arc::clone(&formula)));
                (vocabulary, formula)
            }
        } else {
            return Err(LedgerError::Invalid("claim has no formula or obligation"));
        };
        Ok(ResolvedEntry {
            status,
            vocabulary,
            formula,
        })
    }
}
