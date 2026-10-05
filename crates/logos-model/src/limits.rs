//! Limites de recursos estruturais e computacionais para modelos finitos.

/// Limites de recursos aplicados à construção, codificação e decodificação de modelos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelLimits {
    /// Tamanho máximo em bytes da representação serializada do modelo.
    pub max_bytes: usize,
    /// Cardinalidade máxima permitida para o domínio de qualquer sort individual.
    pub max_domain_size: u32,
    /// Limite agregado no número de células ocupadas por constantes, saídas de funções e tuplas de predicados.
    pub max_cells: usize,
}

impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            max_bytes: 1_048_576,
            max_domain_size: 65_536,
            max_cells: 100_000,
        }
    }
}
