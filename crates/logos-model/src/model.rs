//! Estrutura do modelo finito validado e seus métodos públicos.

use logos_air::Vocabulary;
use logos_canon::{Canon, CanonError, Value};
use logos_core::Exhausted;
use logos_crypto::{Domain, tags};

use crate::codec::{decode_model_with_limits, model_data_to_value};
use crate::data::ModelData;
use crate::error::ModelError;
use crate::exact_size::exact_wire_len;
use crate::limits::ModelLimits;
use crate::validate::validate_model_data;

/// Modelo finito validado sobre uma lógica de primeira ordem clássica multi-sortida.
///
/// Garante semanticamente as regras de fechamento de domínios, totalidade de funções,
/// unicidade e ordenação estrita de tuplas de predicados e vinculação canônica ao vocabulário.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteModel {
    data: ModelData,
}

impl FiniteModel {
    /// Constrói e valida semanticamente um novo modelo a partir de dados explícitos.
    ///
    /// # Errors
    /// Retorna erro semântico caso os dados violem assinaturas, cardinalidades,
    /// totalidade, bounds ou excedam os limites de recursos informados.
    pub fn new(
        data: ModelData,
        vocabulary: &Vocabulary,
        limits: ModelLimits,
    ) -> Result<Self, ModelError> {
        validate_model_data(&data, vocabulary, limits)?;
        Ok(Self { data })
    }

    /// Decodifica e valida um envelope binário canônico `model/v1`.
    ///
    /// # Errors
    /// Retorna erro caso a codificação seja não-canônica, viole limites estruturais
    /// ou falhe na validação semântica contra o vocabulário.
    pub fn decode_with_limits(
        bytes: &[u8],
        vocabulary: &Vocabulary,
        limits: ModelLimits,
    ) -> Result<Self, ModelError> {
        let data = decode_model_with_limits(bytes, vocabulary, limits)?;
        Ok(Self { data })
    }

    /// Codifica o modelo finito no formato canônico binário respeitando os limites informados.
    ///
    /// # Errors
    /// Retorna Unknown(Size) para excesso de bytes, cardinalidade ou células,
    /// Unknown(Memory) para falha de reserva, ou erro canônico de codificação.
    pub fn encode_with_limits(&self, limits: ModelLimits) -> Result<Vec<u8>, ModelError> {
        let wire_len = exact_wire_len(&self.data)?;
        if wire_len > limits.max_bytes {
            return Err(ModelError::Unknown(Exhausted::Size));
        }
        if self
            .data
            .domains
            .iter()
            .any(|&size| size > limits.max_domain_size)
        {
            return Err(ModelError::Unknown(Exhausted::Size));
        }
        let mut cells = self.data.consts.len();
        for (_, outputs) in &self.data.funcs {
            cells = cells
                .checked_add(outputs.len())
                .ok_or(ModelError::Unknown(Exhausted::Size))?;
        }
        for (_, tuples) in &self.data.preds {
            for tuple in tuples {
                cells = cells
                    .checked_add(tuple.len().max(1))
                    .ok_or(ModelError::Unknown(Exhausted::Size))?;
            }
        }
        if cells > limits.max_cells {
            return Err(ModelError::Unknown(Exhausted::Size));
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(wire_len)
            .map_err(|_| ModelError::Unknown(Exhausted::Memory))?;
        self.encode(&mut bytes)?;
        if bytes.len() > limits.max_bytes {
            return Err(ModelError::Unknown(Exhausted::Size));
        }
        Ok(bytes)
    }

    /// Retorna uma referência imutável aos dados explícitos do modelo.
    #[must_use]
    pub const fn data(&self) -> &ModelData {
        &self.data
    }
}

impl Domain for FiniteModel {
    const TAG: &'static [u8] = tags::MODEL;
}

impl Canon for FiniteModel {
    fn to_value(&self) -> Result<Value, CanonError> {
        model_data_to_value(&self.data)
    }
}
