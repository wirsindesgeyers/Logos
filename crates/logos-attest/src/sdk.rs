//! SDK de atestador (6.3): observar, assinar um fato fact/v1 e enviá-lo ao ledger.
//!
//! O atestador tem a própria chave; o agente/LLM nunca a vê. Assinar não torna o fato
//! citável: isso é da configuração de confiança do ledger (trust/v1, 6.2).

use std::fmt;

use logos_air::{
    AirError, AttestorId, Formula, FormulaNode, Grounding, Limits, Sort, Symbol, SymbolDecl,
    SymbolKind, SymbolResult, Term, TermNode, Vocabulary, VocabularyError,
};
use logos_canon::{CanonError, Text, to_canon};
use logos_crypto::{Hash, Signer};
use logos_kernel::{TransportError, facts::register};
use logos_ledger::{CasError, Event, Fact, Ledger, LedgerError, ResourceVersion, Validity};

/// Falha ao observar, assinar ou enviar. Nunca vira um fato "parecido".
#[derive(Debug)]
pub enum AttestError {
    /// Configuração ou observação inválida (mensagem estática).
    Invalid(&'static str),
    /// A observação falhou ou contradiz o contrato do atestador.
    Observation(String),
    /// E/S local.
    Io(std::io::Error),
    /// SQLite do monitor.
    Sqlite(String),
    /// Erro de AIR, vocabulário ou codificação canônica.
    Encoding(String),
    /// O ledger/kernel recusou o fato.
    Rejected(TransportError),
    /// CAS ou ledger.
    Storage(String),
}

impl fmt::Display for AttestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(m) => write!(f, "invalid attester setup: {m}"),
            Self::Observation(m) => write!(f, "observation failed: {m}"),
            Self::Io(e) => write!(f, "i/o: {e}"),
            Self::Sqlite(m) => write!(f, "sqlite: {m}"),
            Self::Encoding(m) => write!(f, "encoding: {m}"),
            Self::Rejected(e) => write!(f, "fact refused by the kernel: {e}"),
            Self::Storage(m) => write!(f, "storage: {m}"),
        }
    }
}

impl std::error::Error for AttestError {}

impl From<std::io::Error> for AttestError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<AirError> for AttestError {
    fn from(e: AirError) -> Self {
        Self::Encoding(format!("{e:?}"))
    }
}
impl From<VocabularyError> for AttestError {
    fn from(e: VocabularyError) -> Self {
        Self::Encoding(format!("{e:?}"))
    }
}
impl From<CanonError> for AttestError {
    fn from(e: CanonError) -> Self {
        Self::Encoding(format!("{e:?}"))
    }
}
impl From<CasError> for AttestError {
    fn from(e: CasError) -> Self {
        Self::Storage(format!("{e:?}"))
    }
}
impl From<LedgerError> for AttestError {
    fn from(e: LedgerError) -> Self {
        Self::Storage(format!("{e:?}"))
    }
}
impl From<TransportError> for AttestError {
    fn from(e: TransportError) -> Self {
        Self::Rejected(e)
    }
}

/// O que um atestador observou: a fórmula, o vocabulário em que ela vale e a janela.
#[derive(Debug, Clone)]
pub struct Observation {
    /// Vocabulário da fórmula (vai ao CAS; `h(V)` entra no fato).
    pub vocabulary: Vocabulary,
    /// Afirmação AIR observada; nunca contém o que o atestador não viu.
    pub formula: Formula,
    /// Recurso e versão observados (hash, mtime, instante do backup…), se houver.
    pub version: Option<ResourceVersion>,
    /// Janela de aplicabilidade do fato.
    pub valid: Validity,
}

/// Um atestador: observa o mundo e devolve uma [`Observation`].
pub trait Attester {
    /// Identificador canônico (`AttestorId` dos símbolos `Anchored`).
    fn id(&self) -> &str;
    /// Tópico atestado (a confiança é por tópico, 6.2).
    fn topic(&self) -> &str;
    /// Identificador do procedimento, descrito em `docs/attesters/<id>.md`.
    fn procedure(&self) -> &'static str;
    /// Observa agora (`now` = instante Unix em segundos).
    ///
    /// # Errors
    /// Se a observação não puder ser feita; nunca devolve valor padrão no lugar.
    fn observe(&mut self, now: u64) -> Result<Observation, AttestError>;
}

/// Fato assinado e objetos que ele precisa no CAS.
#[derive(Debug, Clone)]
pub struct Attestation {
    /// Fato assinado pelo atestador.
    pub fact: Fact,
    /// Vocabulário citado pelo fato.
    pub vocabulary: Vocabulary,
}

/// Observa e assina. A chave do atestador é a única autoridade do fato.
///
/// # Errors
/// Falha da observação ou da assinatura.
pub fn attest<A: Attester + ?Sized, S: Signer>(
    attester: &mut A,
    signer: &S,
    now: u64,
) -> Result<Attestation, AttestError> {
    let observation = attester.observe(now)?;
    let vocabulary_hash: Hash<Vocabulary> = logos_crypto::hash(&observation.vocabulary)?;
    let fact = Fact::sign(
        signer,
        observation.formula,
        vocabulary_hash,
        Text::new(attester.topic())?,
        now,
        observation.valid,
        observation.version,
    )
    .map_err(|e| AttestError::Encoding(format!("{e:?}")))?;
    Ok(Attestation {
        fact,
        vocabulary: observation.vocabulary,
    })
}

/// Resultado de um envio aceito.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Submitted {
    /// `h(Fact)` no CAS.
    pub fact: Hash<Fact>,
    /// Evento `FactRegistered` gravado: a referência citável.
    pub event: Hash<Event>,
}

/// Coloca vocabulário e fato no CAS e pede o registro ao kernel (6.1/6.2).
///
/// # Errors
/// Recusa do kernel (emissor sem `Trusted(emissor, tópico)`, assinatura, janela, repetição)
/// ou falha do CAS/ledger. Recusa não grava evento.
pub fn submit<S: Signer>(
    ledger: &mut Ledger<'_, S>,
    attestation: &Attestation,
) -> Result<Submitted, AttestError> {
    ledger
        .cas()
        .put::<Vocabulary>(&to_canon(&attestation.vocabulary)?)?;
    let fact = ledger.cas().put::<Fact>(&to_canon(&attestation.fact)?)?;
    let event = register(ledger, fact)?;
    Ok(Submitted { fact, event })
}

// ----- construção de vocabulário e fórmulas -----------------------------------------------

/// Sort nominal.
///
/// # Errors
/// Nome inválido como texto canônico.
pub fn sort(name: &str) -> Result<Sort, AttestError> {
    Ok(Sort {
        name: Text::new(name)?,
    })
}

/// Predicado sobre sorts.
///
/// # Errors
/// Nome inválido.
pub fn predicate(name: &str, arity: Vec<Sort>) -> Result<Symbol, AttestError> {
    Ok(Symbol {
        name: Text::new(name)?,
        kind: SymbolKind::Pred,
        arity,
        result: SymbolResult::Bool,
    })
}

/// Constante de um sort.
///
/// # Errors
/// Nome inválido.
pub fn constant(name: &str, of: &Sort) -> Result<Symbol, AttestError> {
    Ok(Symbol {
        name: Text::new(name)?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(of.clone()),
    })
}

/// Declaração `Anchored` ao atestador `attestor`, com o procedimento e a glosa em português.
///
/// # Errors
/// Textos inválidos.
pub fn anchored(
    signature: Symbol,
    attestor: &str,
    procedure: &str,
    gloss_pt: &str,
) -> Result<SymbolDecl, AttestError> {
    Ok(SymbolDecl {
        signature,
        grounding: Grounding::Anchored {
            attestor: AttestorId(Text::new(attestor)?),
            procedure: Text::new(procedure)?,
        },
        gloss_pt: Text::new(gloss_pt)?,
    })
}

/// Átomo `symbol(constantes…)`.
///
/// # Errors
/// Limites de AIR.
pub fn atom(symbol: &Symbol, args: &[&Symbol]) -> Result<Formula, AttestError> {
    let terms = args
        .iter()
        .map(|c| {
            Term::new(
                vec![TermNode::App {
                    symbol: (*c).clone(),
                    args: 0,
                }],
                Limits::default(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Formula::new(
        vec![FormulaNode::Pred {
            symbol: symbol.clone(),
            args: terms,
        }],
        Limits::default(),
    )?)
}

/// `¬φ`. Um atestador que observa a ausência afirma a negação, não omite o fato.
///
/// # Errors
/// Limites de AIR.
pub fn negate(formula: &Formula) -> Result<Formula, AttestError> {
    let mut nodes = vec![FormulaNode::Not];
    nodes.extend_from_slice(formula.nodes());
    Ok(Formula::new(nodes, Limits::default())?)
}

/// Átomo ou sua negação, conforme `holds`.
///
/// # Errors
/// Limites de AIR.
pub fn literal(symbol: &Symbol, args: &[&Symbol], holds: bool) -> Result<Formula, AttestError> {
    let positive = atom(symbol, args)?;
    if holds {
        Ok(positive)
    } else {
        negate(&positive)
    }
}

/// Validade `Until(now + ttl)`.
///
/// # Errors
/// Estouro de `u64`.
pub fn until(now: u64, ttl_secs: u64) -> Result<Validity, AttestError> {
    now.checked_add(ttl_secs)
        .map(Validity::Until)
        .ok_or(AttestError::Invalid("validity overflows u64"))
}

/// Par recurso/versão como texto canônico.
///
/// # Errors
/// Textos vazios ou grandes demais.
pub fn resource_version(resource: &str, version: &str) -> Result<ResourceVersion, AttestError> {
    Ok(ResourceVersion {
        resource: Text::new(resource)?,
        version: Text::new(version)?,
    })
}
