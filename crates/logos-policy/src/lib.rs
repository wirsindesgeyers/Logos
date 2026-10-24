//! Linguagem de política, árvore Merkle e avaliador direto.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.
//!
//! Etapa 7.1 (ADR 0069): objeto `policy/v1`, regras AIR com classe (`allow`, `deny`,
//! `require`, `derive`), forma de Horn com negação estratificada para o fragmento decidível,
//! sort-check contra o vocabulário e semântica deny-overrides. O texto de superfície vive em
//! `logos-policy-text` (fora da TCB): o kernel só consome bytes canônicos validados aqui.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod error;
mod merkle;
mod policy;
mod shape;
mod stratify;

pub use error::PolicyError;
pub use merkle::{
    InclusionProof, IndexKey, Leaf, Neighbour, NoRuleProof, NonMembership, PolicyTree, empty_root,
    verify_inclusion, verify_no_rule, verify_non_membership, verify_present,
};
pub use policy::{
    ALLOWED, Class, MAX_COMMENT_BYTES, MAX_POLICY_BYTES, MAX_RULE_ID_BYTES, MAX_RULES, Policy, Rule,
};
pub use shape::Shape;
