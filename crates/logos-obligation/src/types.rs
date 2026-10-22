//! Tipos centrais do gerador de obrigações canônicas.
//!
//! Contrato normativo: `spec/obligation.md` §1 e §2; ADR 0012.

use core::fmt;
use logos_air::{Formula, LogicId, Vocabulary};
use logos_canon::CanonError;
use logos_crypto::{Domain, Hash, tags};

/// Referência opaca ao digest de 32 bytes do evento citável.
///
/// Conforme `spec/obligation.md` §2, `PremiseRef` contém diretamente o digest do
/// evento original no ledger, sem re-hasheamento ou inclusão de época/status.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PremiseRef([u8; 32]);

impl PremiseRef {
    /// Constrói uma referência a partir do digest de 32 bytes de um evento citável.
    #[must_use]
    pub const fn from_event_hash(hash: [u8; 32]) -> Self {
        Self(hash)
    }

    /// Retorna uma referência aos 32 bytes do digest do evento citável.
    #[must_use]
    pub const fn event_hash(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PremiseRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PremiseRef({self})")
    }
}

impl fmt::Display for PremiseRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// Categoria de uma entrada no ledger.
///
/// Apenas [`Fact`](EntryKind::Fact), [`Policy`](EntryKind::Policy),
/// [`Verified`](EntryKind::Verified) e [`Assumption`](EntryKind::Assumption)
/// são citáveis como premissas de uma obrigação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryKind {
    /// Fato atestado. Citável.
    Fact,
    /// Política do sistema. Citável.
    Policy,
    /// Obrigação formalmente provada e verificada. Citável.
    Verified,
    /// Hipótese com escopo (assunção explícita em $\Gamma$). Citável.
    Assumption,
    /// Proposta ou obrigação pendente de verificação. Não citável.
    Pending,
    /// Obrigação refutada por contramodelo. Não citável.
    Refuted,
    /// Resultado indeterminado / timeout. Não citável.
    Unknown,
    /// Obrigação rejeitada sintática ou semanticamente. Não citável.
    Rejected,
    /// Entrada invalidada retroativamente por retratação. Não citável.
    Invalidated,
    /// Proposta submetida pelo agente. Não citável.
    Proposal,
    /// Rota sustentada só por checker revogado e ainda não rechecada (5.10). Não citável.
    Suspended,
}

impl EntryKind {
    /// Indica se a categoria da entrada pode ser citada como premissa válida.
    #[must_use]
    pub const fn is_citable(self) -> bool {
        matches!(
            self,
            Self::Fact | Self::Policy | Self::Verified | Self::Assumption
        )
    }
}

impl fmt::Display for EntryKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Fact => "Fact",
            Self::Policy => "Policy",
            Self::Verified => "Verified",
            Self::Assumption => "Assumption",
            Self::Pending => "Pending",
            Self::Refuted => "Refuted",
            Self::Unknown => "Unknown",
            Self::Rejected => "Rejected",
            Self::Invalidated => "Invalidated",
            Self::Proposal => "Proposal",
            Self::Suspended => "Suspended",
        };
        f.write_str(name)
    }
}

/// Erro retornado pela consulta ao snapshot do ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LedgerError {
    /// Snapshot ou serviço do ledger indisponível.
    Unavailable,
    /// Registro no ledger inválido ou corrompido.
    InvalidRecord,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable => write!(f, "ledger snapshot unavailable"),
            Self::InvalidRecord => write!(f, "invalid ledger record"),
        }
    }
}

impl std::error::Error for LedgerError {}

/// Entrada consultada em um snapshot confiável do ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntry<'a> {
    /// Categoria do registro.
    pub kind: EntryKind,
    /// Época de aplicabilidade resolvida pelo snapshot.
    pub epoch: u64,
    /// Indica se a entrada foi retratada.
    pub retracted: bool,
    /// Vocabulário associado à entrada.
    pub vocabulary: &'a Vocabulary,
    /// Fórmula da premissa atestada/provada no ledger.
    pub formula: &'a Formula,
}

/// Snapshot abstrato confiável do ledger para verificação de premissas.
///
/// Conforme `spec/obligation.md` §1, representa uma fronteira de confiança:
/// a implementação associa referências a eventos e valida autenticidade/proveniência.
pub trait LedgerView {
    /// Época corrente do snapshot.
    fn current_epoch(&self) -> u64;

    /// Consulta uma referência de evento citada no snapshot.
    ///
    /// Retorna `Ok(Some(entry))` se encontrada e válida, `Ok(None)` se inexistente,
    /// ou `Err(...)` se houver falha de I/O ou registro corrompido.
    fn lookup(&self, reference: PremiseRef) -> Result<Option<LedgerEntry<'_>>, LedgerError>;
}

/// Proposta mínima de obrigação submetida para canonização.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal<'a> {
    /// Vocabulário alvo pretendido.
    pub vocabulary: &'a Vocabulary,
    /// Objetivo (meta) a ser provado.
    pub goal: &'a Formula,
    /// Citações das premissas a serem incluídas em $\Gamma$.
    pub cites: &'a [PremiseRef],
}

/// Obrigação canônica selada $\Gamma \vdash \varphi$ no formato `obligation/v1`.
///
/// Construída exclusivamente através de [`crate::generate`]. Não possui
/// construtores públicos, decodificador canônico nem conversores que permitam
/// a ingestão de obrigações externas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Obligation {
    pub(crate) logic: LogicId,
    pub(crate) encoder_version: u64,
    pub(crate) vocab: Hash<Vocabulary>,
    pub(crate) premises: Vec<(PremiseRef, Formula)>,
    pub(crate) goal: Formula,
}

impl Obligation {
    pub(crate) fn new(
        logic: LogicId,
        encoder_version: u64,
        vocab: Hash<Vocabulary>,
        premises: Vec<(PremiseRef, Formula)>,
        goal: Formula,
    ) -> Self {
        Self {
            logic,
            encoder_version,
            vocab,
            premises,
            goal,
        }
    }

    /// Identificador da lógica normativa do sequente (`FOL-ms-eq/v1`).
    #[must_use]
    pub const fn logic(&self) -> LogicId {
        self.logic
    }

    /// Versão do codificador canônico (`1`).
    #[must_use]
    pub const fn encoder_version(&self) -> u64 {
        self.encoder_version
    }

    /// Digest de 32 bytes do vocabulário alvo no domínio `logos/vocab/v1\0`.
    #[must_use]
    pub const fn vocab(&self) -> Hash<Vocabulary> {
        self.vocab
    }

    /// Premissas aceitas em ordem estritamente crescente de referência de evento.
    #[must_use]
    pub fn premises(&self) -> &[(PremiseRef, Formula)] {
        &self.premises
    }

    /// Meta (objetivo) a ser demonstrada.
    #[must_use]
    pub const fn goal(&self) -> &Formula {
        &self.goal
    }

    /// Calcula o hash da obrigação no domínio reservado `logos/obligation/v1\0`.
    ///
    /// Equivalente a `logos_crypto::hash(self)`.
    ///
    /// # Errors
    /// Retorna [`CanonError`] se a serialização canônica falhar.
    pub fn hash(&self) -> Result<Hash<Self>, CanonError> {
        logos_crypto::hash(self)
    }
}

impl Domain for Obligation {
    const TAG: &'static [u8] = tags::OBLIGATION;
}
