//! Projeção do fecho aceito para hipóteses explícitas, ligada ao contexto.
use logos_core::{Budget, CheckOutcome, Exhausted, StepMeter};
use logos_crypto::Hash;
use logos_ground::{ClauseOrigin, Cnf, ProvenanceMap};
use logos_obligation::{Obligation, PremiseRef};

use crate::resources::{Failure, Resources};
use crate::{LratCertificate, LratReport};

/// Falha de projeção: não é refutação nem claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsedError {
    /// Não há fecho inteiro aceito pelo checker.
    Unaccepted,
    /// Obrigação, CNF ou metadados não correspondem.
    ContextMismatch,
    /// ID original zero, fora do mapa ou sem ordem estritamente crescente.
    InvalidClauseId,
    /// Limite de passos, memória ou tamanho excedido; sem saída parcial.
    Unknown(Exhausted),
}
impl std::fmt::Display for UsedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "premise projection: {self:?}")
    }
}
impl std::error::Error for UsedError {}
impl From<Failure> for UsedError {
    fn from(value: Failure) -> Self {
        match value {
            Failure::Invalid => Self::ContextMismatch,
            Failure::Unknown(reason) => Self::Unknown(reason),
        }
    }
}

/// Conjunto ordenado de hipóteses da prova/tradução; não concede autoridade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsedPremises {
    obligation: Hash<Obligation>,
    cnf: Hash<Cnf>,
    certificate: Hash<LratCertificate>,
    premises: Vec<PremiseRef>,
}
impl UsedPremises {
    /// Obrigação esperada conferida contra o mapa rederivado.
    #[must_use]
    pub fn obligation(&self) -> Hash<Obligation> {
        self.obligation
    }
    /// CNF exata da prova e do mapa.
    #[must_use]
    pub fn cnf_hash(&self) -> Hash<Cnf> {
        self.cnf
    }
    /// Certificado inteiro aceito que produziu este fecho.
    #[must_use]
    pub fn certificate_hash(&self) -> Hash<LratCertificate> {
        self.certificate
    }
    /// Referências únicas/crescentes; sem cópia das citações do agente.
    #[must_use]
    pub fn premises(&self) -> &[PremiseRef] {
        &self.premises
    }
}

/// Extrai hipóteses de um relatório privado aceito e do mapa rederivado.
///
/// O Budget é próprio da projeção, separado daquele da checagem LRAT.
/// Limites internos lógicos; wall_ms não é imposto, nem há teto de RSS.
/// # Errors
/// Relatório inconclusivo/rejeitado, contexto trocado, ID inválido ou limite.
pub fn used_premises(
    report: &LratReport,
    map: &ProvenanceMap,
    expected: Hash<Obligation>,
    budget: &Budget,
) -> Result<UsedPremises, UsedError> {
    if report.outcome() != &CheckOutcome::Accepted {
        return Err(UsedError::Unaccepted);
    }
    let ids = report
        .used_original_clauses()
        .ok_or(UsedError::Unaccepted)?;
    let certificate = report.certificate_hash().ok_or(UsedError::Unaccepted)?;
    if map.obligation() != expected
        || report.cnf_hash() != Some(map.cnf_hash())
        || map.origins().len() != map.clause_support().len()
    {
        return Err(UsedError::ContextMismatch);
    }
    let mut result = Err(UsedError::ContextMismatch);
    let outcome = StepMeter::run(budget, |meter| {
        let mut resources = Resources::new(meter, budget.mem_bytes);
        result = project(ids, map, budget, &mut resources).map(|premises| UsedPremises {
            obligation: expected,
            cnf: map.cnf_hash(),
            certificate,
            premises,
        });
        match &result {
            Err(UsedError::Unknown(reason)) => CheckOutcome::Unknown(*reason),
            _ => CheckOutcome::Accepted,
        }
    });
    if let CheckOutcome::Unknown(reason) = outcome {
        return Err(UsedError::Unknown(reason));
    }
    result
}

fn project(
    ids: &[u64],
    map: &ProvenanceMap,
    budget: &Budget,
    resources: &mut Resources<'_>,
) -> Result<Vec<PremiseRef>, UsedError> {
    resources.step(map.origins().len())?;
    resources.step(ids.len())?;
    let size = || UsedError::Unknown(Exhausted::Size);
    let mut bytes = ids
        .len()
        .checked_mul(8)
        .ok_or_else(size)?
        .checked_add(map.origins().len().checked_mul(33).ok_or_else(size)?)
        .ok_or_else(size)?;
    for support in map.clause_support() {
        bytes = bytes
            .checked_add(support.len().checked_mul(32).ok_or_else(size)?)
            .ok_or_else(size)?;
    }
    if u64::try_from(bytes).map_err(|_| size())? > budget.size_bytes {
        return Err(size());
    }
    let mut premises = Vec::new();
    let mut previous = 0;
    for &id in ids {
        resources.step(1)?;
        if id <= previous {
            return Err(UsedError::InvalidClauseId);
        }
        previous = id;
        let index = usize::try_from(id - 1).map_err(|_| UsedError::InvalidClauseId)?;
        match map.origins().get(index).ok_or(UsedError::InvalidClauseId)? {
            ClauseOrigin::Premise(r) | ClauseOrigin::DomainAxiom(r) => {
                resources.push(&mut premises, *r)?
            }
            ClauseOrigin::NegatedGoal | ClauseOrigin::Definitional => {}
        }
        let support = map
            .clause_support()
            .get(index)
            .ok_or(UsedError::InvalidClauseId)?;
        resources.step(support.len())?;
        for r in support {
            resources.push(&mut premises, *r)?;
        }
    }
    resources.step(
        premises
            .len()
            .checked_mul(premises.len().checked_ilog2().unwrap_or(0) as usize + 1)
            .ok_or(UsedError::Unknown(Exhausted::Steps))?,
    )?;
    premises.sort_unstable();
    premises.dedup();
    Ok(premises)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use logos_air::{Formula, FormulaNode, Limits, Vocabulary};
    use logos_crypto::{hash, hash_raw};
    use logos_ground::{clausify, ground};
    use logos_obligation::{LedgerEntry, LedgerError, LedgerView, Proposal, generate};
    struct Empty;
    impl LedgerView for Empty {
        fn current_epoch(&self) -> u64 {
            0
        }
        fn lookup(&self, _: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError> {
            Ok(None)
        }
    }
    #[test]
    fn invalid_original_ids_and_incomplete_private_reports_never_project() {
        let v = Vocabulary::new(1, vec![], vec![]).unwrap();
        let f = Formula::new(vec![FormulaNode::True], Limits::default()).unwrap();
        let o = generate(
            Proposal {
                vocabulary: &v,
                goal: &f,
                cites: &[],
            },
            &Empty,
        )
        .unwrap();
        let (cnf, map) = clausify(&ground(&o, &v).unwrap()).unwrap();
        let negative = cnf.clauses().iter().position(|c| c == &[-2]).unwrap() + 1;
        let positive = cnf.clauses().iter().position(|c| c == &[2]).unwrap() + 1;
        let proof = format!("{} 0 {positive} {negative} 0\n", cnf.clauses().len() + 1);
        let budget = Budget::new(100_000, 100_000, 100_000, 0);
        let accepted = crate::LratChecker::new().unwrap().check_with_used(
            cnf.dimacs(),
            hash_raw(cnf.dimacs()),
            proof.as_bytes(),
            &budget,
        );
        assert_eq!(accepted.outcome(), &CheckOutcome::Accepted);
        assert!(
            used_premises(&accepted, &map, hash(&o).unwrap(), &budget)
                .unwrap()
                .premises()
                .is_empty()
        );
        for ids in [
            vec![0],
            vec![map.origins().len() as u64 + 1],
            vec![1, 1],
            vec![2, 1],
            vec![u64::MAX],
        ] {
            let mut report = accepted.clone();
            report.used = Some(ids);
            assert_eq!(
                used_premises(&report, &map, hash(&o).unwrap(), &budget),
                Err(UsedError::InvalidClauseId)
            );
        }
        for outcome in [
            CheckOutcome::Unknown(Exhausted::Steps),
            CheckOutcome::Rejected(crate::LratChecker::new().unwrap().invalid),
        ] {
            let mut report = accepted.clone();
            report.outcome = outcome;
            assert_eq!(
                used_premises(&report, &map, hash(&o).unwrap(), &budget),
                Err(UsedError::Unaccepted)
            );
        }
        let mut report = accepted.clone();
        report.used = None;
        assert_eq!(
            used_premises(&report, &map, hash(&o).unwrap(), &budget),
            Err(UsedError::Unaccepted)
        );
        let mut report = accepted;
        report.certificate = None;
        assert_eq!(
            used_premises(&report, &map, hash(&o).unwrap(), &budget),
            Err(UsedError::Unaccepted)
        );
    }
}
