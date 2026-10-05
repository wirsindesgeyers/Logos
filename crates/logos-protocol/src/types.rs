//! Tipos candidatos; nenhum deles representa autoridade ou prova.

use logos_air::{Formula, Vocabulary};
use logos_crypto::Hash;
use logos_obligation::PremiseRef;

/// Texto arbitrário do agente, explicitamente não confiável.
///
/// Sem normalização ou interpretação; sem Canon, Domain ou Display implícito.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UntrustedText(String);

impl UntrustedText {
    /// Marca texto como dado não confiável sem alterar seus bytes UTF-8.
    #[must_use]
    pub fn new(text: String) -> Self {
        Self(text)
    }

    /// Acesso explícito ao texto não confiável, nunca a uma instrução/política.
    #[must_use]
    pub fn as_untrusted_str(&self) -> &str {
        &self.0
    }
}

/// Identificador opaco de claim; não é digest de evento citável.
///
/// Não atribui identidade autenticada nem prova existência em ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClaimId([u8; 32]);

impl ClaimId {
    /// Representa os bytes transmitidos, sem buscar ou autenticar o claim.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Bytes opacos do identificador.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Única mensagem candidata do agente definida em `proposal/v1`.
///
/// Não confundir com o input emprestado/resolvido de logos-obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// Texto português tainted, excluído da obrigação lógica.
    pub text_pt: UntrustedText,
    /// Objetivo sintaticamente validado; tipagem/verdade não são garantidas aqui.
    pub goal: Formula,
    /// Digests de eventos citados, preservando ordem e duplicatas do agente.
    pub cites: Vec<PremiseRef>,
    /// Claims especulativos apenas representados; não tornam Pending citável.
    pub speculative_on: Vec<ClaimId>,
    /// Identidade pretendida de Vocabulary; ainda não resolvida/autenticada.
    pub vocab: Hash<Vocabulary>,
}
