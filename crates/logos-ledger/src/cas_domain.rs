//! Resolução estática de domínios e limites padrão para o CAS.
//!
//! Contrato normativo conforme `spec/cas.md` e `spec/domains.md`.
//! Este módulo é puramente determinístico e não depende de I/O ou alocações.

use logos_crypto::tags;

/// Entrada estática descrevendo um domínio CAS registrado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DomainEntry {
    /// Tag binária terminada em byte nulo (`logos/<nome>/v<k>\0`).
    pub(crate) tag: &'static [u8],
    /// Nome do subdiretório sob a raiz CAS.
    pub(crate) directory: &'static str,
    /// Limite padrão em bytes (16 MiB para canônicos, 256 MiB para crus).
    pub(crate) limit: u64,
}

/// Limite padrão de 16 MiB (16.777.216 bytes) para domínios canônicos.
const CANONICAL_LIMIT: u64 = 16 * 1024 * 1024;

/// Limite padrão de 256 MiB (268.435.456 bytes) para domínios crus (raw).
const RAW_LIMIT: u64 = 256 * 1024 * 1024;

/// Tabela fixa com todos os 16 domínios reservados na ordem de `spec/domains.md`.
pub(crate) const DOMAINS: [DomainEntry; 16] = [
    DomainEntry {
        tag: tags::KEY,
        directory: "key-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::FORMULA,
        directory: "formula-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::VOCAB,
        directory: "vocab-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::PROPOSAL,
        directory: "proposal-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::OBLIGATION,
        directory: "obligation-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::CERTIFICATE,
        directory: "certificate-v1",
        limit: RAW_LIMIT,
    },
    DomainEntry {
        tag: tags::CHECKER,
        directory: "checker-v1",
        limit: RAW_LIMIT,
    },
    DomainEntry {
        tag: tags::MODEL,
        directory: "model-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::EVENT,
        directory: "event-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::FACT,
        directory: "fact-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::POLICY,
        directory: "policy-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::CONTRACT,
        directory: "contract-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::ACTION,
        directory: "action-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::PARAPHRASE,
        directory: "paraphrase-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::TOKEN,
        directory: "token-v1",
        limit: CANONICAL_LIMIT,
    },
    DomainEntry {
        tag: tags::ASSUMPTION,
        directory: "assumption-v1",
        limit: CANONICAL_LIMIT,
    },
];

/// Resolve uma tag binária exata para o índice correspondente na tabela [`DOMAINS`].
///
/// Retorna `Some(index)` se a tag for exatamente uma das 16 tags reservadas,
/// ou `None` caso contrário (inclusive se for prefixo, sufixo ou malformada).
#[must_use]
pub(crate) fn resolve(tag: &[u8]) -> Option<usize> {
    for (i, entry) in DOMAINS.iter().enumerate() {
        if entry.tag == tag {
            return Some(i);
        }
    }
    None
}
