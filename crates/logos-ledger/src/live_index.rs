//! Projeção privada vinculada ao writer: replay inicial e sufixos autenticados.

use std::cell::{Ref, RefCell};
use std::path::Path;

use logos_air::Formula;
use logos_crypto::{Hash, PublicKey};
use logos_obligation::PremiseRef;
use rusqlite::Connection;

use crate::{
    Cas, ClaimStatus, Event, LedgerError, LedgerHead, LedgerIndex, LedgerSnapshot, index,
    index_project, ledger,
};

/// Leitura imutável que empresta o writer e impede append/Drop durante seu uso.
/// Não publica cache nem oferece relatório de rebuild a cada consulta.
pub struct LedgerLiveIndex<'a> {
    index: Ref<'a, LedgerIndex<'static>>,
}

pub(crate) fn borrow<'a>(
    source: &Connection,
    path: &Path,
    key: &PublicKey,
    target: Option<LedgerHead>,
    cache: &'a RefCell<Option<LedgerIndex<'static>>>,
) -> Result<LedgerLiveIndex<'a>, LedgerError> {
    let changed = cache
        .try_borrow()
        .map_err(|_| LedgerError::Invalid("live index borrow"))?
        .as_ref()
        .is_none_or(|index| index.head != target);
    if changed {
        let mut slot = cache
            .try_borrow_mut()
            .map_err(|_| LedgerError::Invalid("live index borrow"))?;
        let result = match slot.as_mut() {
            Some(index) => refresh(index, source, path, key, target),
            None => {
                let index = LedgerIndex::build(path, key, None, None, Some(target))?;
                *slot = Some(index);
                Ok(())
            }
        };
        if let Err(error) = result {
            // A falha não deixa uma projeção parcial disponível à próxima consulta.
            *slot = None;
            return Err(error);
        }
    }
    let slot = cache
        .try_borrow()
        .map_err(|_| LedgerError::Invalid("live index borrow"))?;
    let index = Ref::filter_map(slot, Option::as_ref)
        .map_err(|_| LedgerError::Invalid("live index absent"))?;
    Ok(LedgerLiveIndex { index })
}

fn refresh(
    index: &mut LedgerIndex<'static>,
    source: &Connection,
    path: &Path,
    key: &PublicKey,
    target: Option<LedgerHead>,
) -> Result<(), LedgerError> {
    index.connection.execute_batch("PRAGMA query_only=OFF")?;
    let result = (|| {
        let cas = crate::Cas::open(path.join("cas"), crate::Limits::default())?;
        let transaction = index.connection.transaction()?;
        let head = ledger::scan_after(source, key, index.head, |event, head| {
            index_project::apply(&transaction, &cas, event, head)
        })?;
        if head != target {
            return Err(LedgerError::Invalid("live index head"));
        }
        index::write_head(&transaction, head)?;
        transaction.commit()?;
        Ok(())
    })();
    let seal = index.connection.execute_batch("PRAGMA query_only=ON");
    result?;
    seal?;
    index.head = target;
    Ok(())
}

impl LedgerLiveIndex<'_> {
    /// Cabeça autenticada exata do writer no empréstimo.
    #[must_use]
    pub fn head(&self) -> Option<LedgerHead> {
        self.index.head()
    }

    /// Categoria/época/retratação reais; erro não vira ausência.
    /// # Errors
    /// SQL ou projeção inválida.
    pub fn status(&self, reference: Hash<Event>) -> Result<Option<ClaimStatus>, LedgerError> {
        self.index.status(reference)
    }

    /// Primeiro Refuted da obrigação (S4); ausência não é prova de nada.
    /// # Errors
    /// SQL ou projeção inválida.
    pub fn refuted(
        &self,
        obligation: Hash<logos_obligation::Obligation>,
    ) -> Result<Option<Hash<Event>>, LedgerError> {
        self.index.refuted(obligation)
    }

    /// Evento FactRegistered de um fato, se registrado (6.1).
    /// # Errors
    /// SQL ou projeção inválida.
    pub fn fact_event(&self, fact: Hash<crate::Fact>) -> Result<Option<Hash<Event>>, LedgerError> {
        self.index.fact_event(fact)
    }

    /// Menor rota Verified ativa e citável do nó h(O) (5.8).
    /// # Errors
    /// SQL ou projeção inválida.
    pub fn active_route(
        &self,
        obligation: Hash<logos_obligation::Obligation>,
    ) -> Result<Option<Hash<Event>>, LedgerError> {
        self.index.active_route(obligation)
    }

    /// Rotas do nó h(O) quando todas estão mortas; vazio se alguma vive (5.8).
    /// # Errors
    /// SQL ou projeção inválida.
    pub fn dead_routes(
        &self,
        obligation: Hash<logos_obligation::Obligation>,
    ) -> Result<Vec<Hash<Event>>, LedgerError> {
        self.index.dead_routes(obligation)
    }

    /// Assumptions abertas no fecho used (5.7); `None` se a referência não existe.
    /// # Errors
    /// SQL ou projeção inválida.
    pub fn open_assumptions(
        &self,
        reference: Hash<Event>,
    ) -> Result<Option<crate::index::OpenAssumptions>, LedgerError> {
        self.index.open_assumptions(reference)
    }

    /// Citabilidade de estado na época atual, sem promover épocas anteriores.
    /// # Errors
    /// SQL ou projeção inválida.
    pub fn is_citable(&self, reference: Hash<Event>, epoch: u64) -> Result<bool, LedgerError> {
        self.index.is_citable(reference, epoch)
    }

    /// Dependentes diretos derivados exclusivamente de used.
    /// # Errors
    /// SQL ou digest inválido.
    pub fn dependents(&self, reference: Hash<Event>) -> Result<Vec<Hash<Event>>, LedgerError> {
        self.index.dependents(reference)
    }

    /// Resolve fórmula com conferência integral dos objetos CAS.
    /// # Errors
    /// CAS, envelope, tipagem ou SQL inválidos.
    pub fn formula_of(
        &self,
        reference: Hash<Event>,
        cas: &Cas,
    ) -> Result<Option<Formula>, LedgerError> {
        self.index.formula_of(reference, cas)
    }

    /// Prepara a LedgerView contextual usando as mesmas regras do rebuild.
    /// # Errors
    /// CAS, tipagem, scope ou SQL inválidos.
    pub fn snapshot<'a>(
        &'a self,
        cas: &Cas,
        references: &[PremiseRef],
        scope: &str,
    ) -> Result<LedgerSnapshot<'a>, LedgerError> {
        self.index.snapshot(cas, references, scope)
    }
}
