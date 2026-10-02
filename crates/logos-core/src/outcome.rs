//! Tipos de resultado de checagem, motivos de exaustão e razões de rejeição.

use logos_canon::Text;

/// Motivo de esgotamento de recurso computacional ou físico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Exhausted {
    /// Limite de tempo de relógio esgotado.
    Time,
    /// Limite de passos lógicos esgotado.
    Steps,
    /// Limite de memória esgotado.
    Memory,
    /// Limite de tamanho de objeto ou certificado esgotado.
    Size,
}

/// Razão canônica para a rejeição de uma checagem.
///
/// Encapsula um [`Text`] canônico já validado (UTF-8 em NFC, code points atribuídos).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reason {
    pub(crate) text: Text,
}

impl Reason {
    /// Constrói uma nova razão a partir de um texto canônico validado.
    #[must_use]
    pub const fn new(text: Text) -> Self {
        Self { text }
    }

    /// Retorna a razão como fatia de string UTF-8.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.text.as_str()
    }
}

impl core::fmt::Display for Reason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Resultado bruto retornado por um checker.
///
/// Não é um veredicto: a conversão para [`Verdict`](crate::Verdict) depende
/// do contexto de avaliação (prova formal vs. contramodelo) e é realizada
/// exclusivamente pelas funções nomeadas [`verified_from_proof`](crate::verified_from_proof)
/// e [`refuted_from_countermodel`](crate::refuted_from_countermodel).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    /// Checagem aceita com sucesso pelo checker.
    Accepted,
    /// Checagem rejeitada com uma razão diagnóstica.
    Rejected(Reason),
    /// Checagem inconclusiva por esgotamento de recurso.
    Unknown(Exhausted),
}
