//! Decodificador estrito e contextual de `obligation/v1` para checagem.
//!
//! Contrato normativo: `spec/obligation.md` §2; ADR 0016; etapa 2.3.
//!
//! Transforma bytes serializados de uma obrigação e uma expectativa de hash
//! em um [`CheckedObligation`] validado contra um [`Vocabulary`].
//! Não confere autoridade de proveniência nem cria [`Obligation`] selada.

use core::fmt;

use logos_air::{AirError, Formula, Limits, LogicId, SortError, Vocabulary};
use logos_canon::{CanonError, Decoder, Value};
use logos_core::Exhausted;
use logos_crypto::{Hash, hash_raw};

use crate::types::{Obligation, PremiseRef};

/// Limites de decodificação para proteção contra esgotamento de recursos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeLimits {
    /// Tamanho máximo em bytes do sequente serializado.
    pub max_bytes: usize,
    /// Quantidade máxima de premissas admitidas no sequente.
    pub max_premises: usize,
    /// Limites da representação intermediária AIR (profundidade por fórmula e nós agregados).
    pub air: Limits,
}

impl DecodeLimits {
    /// Limite padrão de bytes: 1 MiB (1.048.576 bytes).
    pub const DEFAULT_MAX_BYTES: usize = 1024 * 1024;
    /// Limite padrão de premissas: 4.096.
    pub const DEFAULT_MAX_PREMISES: usize = 4096;
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            max_bytes: Self::DEFAULT_MAX_BYTES,
            max_premises: Self::DEFAULT_MAX_PREMISES,
            air: Limits::default(),
        }
    }
}

/// Sequente verificado contextual e estruturalmente para checagem lógica.
///
/// Objeto não autoritativo de consumo interno do checker: não implementa
/// [`logos_canon::Canon`], [`logos_canon::CanonDecode`], [`logos_crypto::Domain`]
/// nem conversão para [`crate::Obligation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedObligation {
    vocab: Hash<Vocabulary>,
    premises: Vec<(PremiseRef, Formula)>,
    goal: Formula,
}

impl CheckedObligation {
    /// Digest de 32 bytes do vocabulário alvo no domínio `logos/vocab/v1\0`.
    #[must_use]
    pub fn vocab(&self) -> Hash<Vocabulary> {
        self.vocab
    }

    /// Premissas aceitas em ordem estritamente crescente de referência de evento.
    #[must_use]
    pub fn premises(&self) -> &[(PremiseRef, Formula)] {
        &self.premises
    }

    /// Meta (objetivo) a ser demonstrada.
    #[must_use]
    pub fn goal(&self) -> &Formula {
        &self.goal
    }
}

/// Motivo de recusa de decodificação de uma obrigação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Esgotamento de recurso computacional (tamanho de bytes, premissas, memória ou nós/profundidade AIR).
    Unknown(Exhausted),
    /// Hash recalculado dos bytes originais diverge do hash esperado pelo chamador.
    HashMismatch,
    /// Hash do vocabulário codificado no envelope diverge do vocabulário alvo fornecido.
    VocabularyMismatch,
    /// Violação da codificação canônica determinística CBOR (`canon/v1`).
    Canon(CanonError),
    /// Violação da sintaxe abstrata ou envelope AIR (`FOL-ms-eq/v1`).
    Air(AirError),
    /// Falha de tipagem/fechamento das fórmulas contra o vocabulário alvo.
    Sort(SortError),
    /// Referências de premissas fora de ordem estritamente crescente ou duplicadas.
    InvalidPremises,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(exhausted) => write!(f, "resource exhausted: {exhausted:?}"),
            Self::HashMismatch => f.write_str("obligation hash mismatch"),
            Self::VocabularyMismatch => f.write_str("vocabulary hash mismatch"),
            Self::Canon(err) => write!(f, "canonical encoding error: {err}"),
            Self::Air(err) => write!(f, "AIR error: {err}"),
            Self::Sort(err) => write!(f, "sort check error: {err}"),
            Self::InvalidPremises => f.write_str("invalid premises ordering or duplicates"),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Canon(err) => Some(err),
            Self::Air(err) => Some(err),
            Self::Sort(err) => Some(err),
            _ => None,
        }
    }
}

impl From<CanonError> for DecodeError {
    fn from(err: CanonError) -> Self {
        Self::Canon(err)
    }
}

impl From<SortError> for DecodeError {
    fn from(err: SortError) -> Self {
        Self::Sort(err)
    }
}

impl From<Exhausted> for DecodeError {
    fn from(e: Exhausted) -> Self {
        Self::Unknown(e)
    }
}

/// Decodifica e valida estritamente uma obrigação em bytes contra um vocabulário.
///
/// # Algoritmo
/// 1. Confere limite de bytes `limits.max_bytes` -> [`DecodeError::Unknown(Exhausted::Size)`].
/// 2. Confere `hash_raw::<Obligation>(bytes) == expected` -> [`DecodeError::HashMismatch`].
/// 3. Decodifica valor canônico CBOR via [`logos_canon::Decoder`] e exige fim exato de entrada.
/// 4. Confere envelope exato `[1, 0, "FOL-ms-eq/v1", 1, bytes32_hV, premises, goal]`.
/// 5. Confere hash do vocabulário alvo contra `logos_crypto::hash(vocabulary)`.
/// 6. Confere limites e ordenação estritamente crescente das referências de premissas.
/// 7. Decodifica e faz sort-check de cada premissa e da meta, agregando nós de fórmulas.
///
/// # Errors
/// Retorna [`DecodeError`] se qualquer conferência falhar.
pub fn decode_for_check(
    bytes: &[u8],
    expected: Hash<Obligation>,
    vocabulary: &Vocabulary,
    limits: DecodeLimits,
) -> Result<CheckedObligation, DecodeError> {
    // 1. Limite de tamanho de entrada verificado antes de qualquer processamento
    if bytes.len() > limits.max_bytes {
        return Err(DecodeError::Unknown(Exhausted::Size));
    }

    // 2. Hash dos bytes originais conferido antes de CBOR decoding
    let actual_hash = hash_raw::<Obligation>(bytes);
    if actual_hash != expected {
        return Err(DecodeError::HashMismatch);
    }

    // 3. Decodificação estrita de canon/v1 sem bytes extras
    let mut decoder = Decoder::new(bytes);
    let value = decoder.value().map_err(DecodeError::Canon)?;
    decoder.finish().map_err(DecodeError::Canon)?;

    // 4. Envelope exato: array de 7 elementos
    let Value::Array(arr) = value else {
        return Err(DecodeError::Canon(CanonError::WrongType));
    };
    let items = arr.items();
    if items.len() != 7 {
        return Err(DecodeError::Canon(CanonError::WrongType));
    }

    // Item 0: Versão de obligation/v1 == 1
    match items[0] {
        Value::Unsigned(1) => {}
        _ => return Err(DecodeError::Canon(CanonError::WrongType)),
    }

    // Item 1: Tag de tipo Obligation == 0
    match items[1] {
        Value::Unsigned(0) => {}
        _ => return Err(DecodeError::Canon(CanonError::WrongType)),
    }

    // Item 2: LogicId == "FOL-ms-eq/v1"
    let logic_text = match &items[2] {
        Value::Text(t) => t.as_str(),
        _ => return Err(DecodeError::Canon(CanonError::WrongType)),
    };
    let _logic = LogicId::from_name(logic_text).map_err(DecodeError::Air)?;

    // Item 3: Encoder version == 1
    match items[3] {
        Value::Unsigned(1) => {}
        _ => return Err(DecodeError::Canon(CanonError::WrongType)),
    }

    // Item 4: Digest do vocabulário alvo == 32 bytes
    let vocab_hash_bytes = match &items[4] {
        Value::Bytes(b) => <[u8; 32]>::try_from(b.as_slice())
            .map_err(|_| DecodeError::Canon(CanonError::WrongType))?,
        _ => return Err(DecodeError::Canon(CanonError::WrongType)),
    };
    let vocab_hash = Hash::<Vocabulary>::from_bytes(vocab_hash_bytes);
    let actual_vocab_hash = logos_crypto::hash(vocabulary).map_err(DecodeError::Canon)?;
    if vocab_hash != actual_vocab_hash {
        return Err(DecodeError::VocabularyMismatch);
    }

    // Item 5: Premissas: array de pares [ref32, Formula]
    let raw_premises = match &items[5] {
        Value::Array(a) => a.items(),
        _ => return Err(DecodeError::Canon(CanonError::WrongType)),
    };
    if raw_premises.len() > limits.max_premises {
        return Err(DecodeError::Unknown(Exhausted::Size));
    }

    let mut premises = Vec::new();
    premises
        .try_reserve_exact(raw_premises.len())
        .map_err(|_| DecodeError::Unknown(Exhausted::Memory))?;

    let mut prev_ref: Option<[u8; 32]> = None;
    let mut remaining_nodes = limits.air.max_nodes;

    for raw_premise in raw_premises {
        let pair = match raw_premise {
            Value::Array(a) => a.items(),
            _ => return Err(DecodeError::Canon(CanonError::WrongType)),
        };
        if pair.len() != 2 {
            return Err(DecodeError::Canon(CanonError::WrongType));
        }

        let ref_bytes = match &pair[0] {
            Value::Bytes(b) => <[u8; 32]>::try_from(b.as_slice())
                .map_err(|_| DecodeError::Canon(CanonError::WrongType))?,
            _ => return Err(DecodeError::Canon(CanonError::WrongType)),
        };

        // Referências devem ser estritamente crescentes e deduplicadas
        if let Some(prev) = prev_ref
            && ref_bytes <= prev
        {
            return Err(DecodeError::InvalidPremises);
        }
        prev_ref = Some(ref_bytes);
        let premise_ref = PremiseRef::from_event_hash(ref_bytes);

        // Fórmulas consomem orçamento agregado de nós; profundidade é por fórmula
        let formula_limits = Limits {
            max_depth: limits.air.max_depth,
            max_nodes: remaining_nodes,
        };
        let formula = match Formula::from_value_with_limits(&pair[1], formula_limits) {
            Ok(f) => f,
            Err(AirError::DepthExceeded { .. } | AirError::NodesExceeded { .. }) => {
                return Err(DecodeError::Unknown(Exhausted::Size));
            }
            Err(AirError::Canon(ce)) => return Err(DecodeError::Canon(ce)),
            Err(e) => return Err(DecodeError::Air(e)),
        };
        remaining_nodes = remaining_nodes
            .checked_sub(formula.node_count())
            .ok_or(DecodeError::Unknown(Exhausted::Size))?;

        logos_air::check(vocabulary, &formula).map_err(DecodeError::Sort)?;
        premises.push((premise_ref, formula));
    }

    // Item 6: Meta: Formula
    let goal_limits = Limits {
        max_depth: limits.air.max_depth,
        max_nodes: remaining_nodes,
    };
    let goal = match Formula::from_value_with_limits(&items[6], goal_limits) {
        Ok(f) => f,
        Err(AirError::DepthExceeded { .. } | AirError::NodesExceeded { .. }) => {
            return Err(DecodeError::Unknown(Exhausted::Size));
        }
        Err(AirError::Canon(ce)) => return Err(DecodeError::Canon(ce)),
        Err(e) => return Err(DecodeError::Air(e)),
    };

    logos_air::check(vocabulary, &goal).map_err(DecodeError::Sort)?;

    Ok(CheckedObligation {
        vocab: vocab_hash,
        premises,
        goal,
    })
}
