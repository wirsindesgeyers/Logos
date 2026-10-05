//! Estrutura de dados explícita do modelo finito.

use logos_air::Vocabulary;
use logos_crypto::Hash;

/// Dados explícitos de um modelo finito, não confiáveis até validação semântica.
///
/// Os índices correspondem à posição na ordem declarada exata em `vocabulary.sorts()`
/// e `vocabulary.symbols()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelData {
    /// Hash canônico do vocabulário ao qual este modelo se vincula.
    pub vocab: Hash<Vocabulary>,
    /// Cardinalidades dos domínios, ordenadas pela declaração dos sorts.
    pub domains: Vec<u32>,
    /// Interpretações de constantes `(id, valor)`, ordenadas estritamente por ID crescente.
    pub consts: Vec<(u32, u32)>,
    /// Interpretações de funções `(id, saídas)`, ordenadas estritamente por ID crescente.
    pub funcs: Vec<(u32, Vec<u32>)>,
    /// Interpretações de predicados `(id, tuplas)`, ordenadas estritamente por ID crescente.
    pub preds: Vec<(u32, Vec<Vec<u32>>)>,
}
