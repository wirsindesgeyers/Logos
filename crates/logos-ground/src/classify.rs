//! Fragmento estrutural de todo o contexto, sem resolver fontes ou emitir rota.

use std::collections::BTreeMap;

use logos_air::{
    Formula, FormulaNode, Grounding, Sort, SortDomain, SymbolKind, SymbolResult, Vocabulary, check,
    check_with_binders,
};
use logos_core::Exhausted;
use logos_crypto::hash;
use logos_obligation::Obligation;

use crate::GroundError;

/// Elegibilidade estrutural conservadora; não certifica prova ou origem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fragment {
    /// Quantificação e valores restritos a enumerações declaradas.
    FiniteDomain,
    /// Contexto contém domínio Open relevante à política conservadora global.
    General,
}

/// Limites lógicos da entrada; não medem RSS/tempo/bytes de nomes e glosas.
#[derive(Debug, Clone, Copy)]
pub struct AnalysisLimits {
    /// Soma de declarações de sorts e símbolos.
    pub max_declarations: usize,
    /// Soma de nós de O/Defined, aridades e constantes de enumeração.
    pub max_nodes: usize,
}
impl Default for AnalysisLimits {
    fn default() -> Self {
        Self {
            max_declarations: 10_000,
            max_nodes: 100_000,
        }
    }
}

/// Classifica o contexto sob limites padrão, sem modificar O.
///
/// # Errors
/// Recusa h(V) divergente, contexto mal tipado, assinatura ou limites inválidos.
pub fn classify(o: &Obligation, v: &Vocabulary) -> Result<Fragment, GroundError> {
    classify_with_limits(o, v, AnalysisLimits::default())
}

/// Classifica todo V/Defined e todas as premissas/goal de O.
///
/// Const é função de aridade zero; resultado Open exige General, como Func. Parâmetros
/// Defined também são implicitamente quantificados. Valida o contexto completo
/// antes de devolver General: erro posterior não é escondido por domínio Open.
///
/// # Errors
/// Os de [`classify`], com limites inclusivos fornecidos pelo chamador.
pub fn classify_with_limits(
    o: &Obligation,
    v: &Vocabulary,
    limits: AnalysisLimits,
) -> Result<Fragment, GroundError> {
    let size = || GroundError::Unknown(Exhausted::Size);
    if v.sorts()
        .len()
        .checked_add(v.symbols().len())
        .ok_or_else(size)?
        > limits.max_declarations
    {
        return Err(size());
    }
    let mut count = 0usize;
    let mut add = |n: usize| -> Result<(), GroundError> {
        count = count.checked_add(n).ok_or_else(size)?;
        if count > limits.max_nodes {
            return Err(size());
        }
        Ok(())
    };
    add(o.goal().node_count())?;
    for (_, f) in o.premises() {
        add(f.node_count())?;
    }
    for sort in v.sorts() {
        if let SortDomain::Enumerated { constants, .. } = &sort.domain {
            add(constants.len())?;
        }
    }
    for symbol in v.symbols() {
        add(symbol.signature.arity.len())?;
        if let Grounding::Defined(body) = &symbol.grounding {
            add(body.node_count())?;
        }
    }
    if hash(v)? != o.vocab() {
        return Err(GroundError::VocabularyMismatch);
    }
    let sorts = v
        .sorts()
        .iter()
        .map(|s| (&s.name, &s.domain))
        .collect::<BTreeMap<_, _>>();
    let open = |s: &Sort| -> Result<bool, GroundError> {
        match sorts.get(&s.name) {
            Some(SortDomain::Open) => Ok(true),
            Some(SortDomain::Enumerated { .. }) => Ok(false),
            None => Err(GroundError::UnknownSort),
        }
    };
    let mut general = false;
    for declaration in v.symbols() {
        let signature = &declaration.signature;
        match (&signature.kind, &signature.result) {
            (SymbolKind::Const, SymbolResult::Sort(s)) if signature.arity.is_empty() => {
                general |= open(s)?;
            }
            (SymbolKind::Func, SymbolResult::Sort(s)) => {
                general |= open(s)?;
            }
            (SymbolKind::Pred, SymbolResult::Bool) => {}
            _ => return Err(GroundError::InvalidSignature),
        }
        if let Grounding::Defined(body) = &declaration.grounding {
            let mut binders = Vec::new();
            let count = signature.arity.len().checked_add(1).ok_or_else(size)?;
            binders
                .try_reserve_exact(count)
                .map_err(|_| GroundError::Unknown(Exhausted::Memory))?;
            binders.extend(signature.arity.iter().rev());
            if let SymbolResult::Sort(s) = &signature.result {
                binders.push(s);
            }
            for s in &binders {
                general |= open(s)?;
            }
            check_with_binders(v, body, &binders)?;
            general |= open_quantifier(body, &open)?;
        }
    }
    for f in o
        .premises()
        .iter()
        .map(|(_, f)| f)
        .chain(std::iter::once(o.goal()))
    {
        check(v, f)?;
        general |= open_quantifier(f, &open)?;
    }
    Ok(if general {
        Fragment::General
    } else {
        Fragment::FiniteDomain
    })
}

fn open_quantifier(
    f: &Formula,
    open: &impl Fn(&Sort) -> Result<bool, GroundError>,
) -> Result<bool, GroundError> {
    let mut result = false;
    for node in f.nodes() {
        if let FormulaNode::Forall(s) | FormulaNode::Exists(s) = node {
            result |= open(s)?;
        }
    }
    Ok(result)
}
