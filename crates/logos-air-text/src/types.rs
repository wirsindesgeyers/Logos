//! Tipos de documento, claims e referências de superfície AIR.

use logos_air::{Formula, Vocabulary};
use logos_canon::Text;
use std::collections::BTreeMap;

/// Mapa de referências associando apelidos a digests de 32 bytes de fatos ou políticas.
pub type References = BTreeMap<String, [u8; 32]>;

/// Estrutura de claim em memória contendo nome, fórmula e premissas citadas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    /// Nome canônico do claim.
    pub name: Text,
    /// Fórmula AIR do claim.
    pub formula: Formula,
    /// Nomes das premissas ou claims citados, preservando ordem e repetição.
    pub cites: Vec<Text>,
}

/// Documento AIR de superfície contendo vocabulário e lista de claims declarados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Vocabulário imutável validado.
    pub vocabulary: Vocabulary,
    /// Claims declarados no documento, preservando ordem.
    pub claims: Vec<Claim>,
}
