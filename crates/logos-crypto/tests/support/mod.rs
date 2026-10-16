//! Utilitários compartilhados pelos testes (incluído por `mod support;`).
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_crypto::{Domain, SigDomain};

pub fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "hex ímpar: {s}");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn vectors(rel: &str) -> Vec<Vec<String>> {
    let path = format!("{}/../../spec/vectors/{rel}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

/// Domínio de assinatura com tag vazia: expõe o Ed25519 puro para os vetores do RFC 8032.
pub struct RawSig;
impl SigDomain for RawSig {
    const TAG: &'static [u8] = b"";
}

macro_rules! hash_domains {
    ($($ident:ident => $tag:expr),* $(,)?) => {
        $(pub struct $ident; impl Domain for $ident { const TAG: &'static [u8] = $tag; })*
    };
}
use logos_crypto::{sig_tags, tags};
hash_domains! {
    DKey => tags::KEY, DFormula => tags::FORMULA, DVocab => tags::VOCAB, DProposal => tags::PROPOSAL,
    DObligation => tags::OBLIGATION, DCertificate => tags::CERTIFICATE, DChecker => tags::CHECKER,
    DModel => tags::MODEL, DGround => tags::GROUND, DEvent => tags::EVENT, DFact => tags::FACT, DPolicy => tags::POLICY,
    DContract => tags::CONTRACT, DAction => tags::ACTION, DParaphrase => tags::PARAPHRASE,
    DToken => tags::TOKEN, DAssumption => tags::ASSUMPTION,
}

/// `hash_raw` no domínio chamado `name` (nome da tabela de spec/domains.md).
pub fn hash_by_name(name: &str, msg: &[u8]) -> String {
    use logos_crypto::hash_raw;
    match name {
        "key" => hash_raw::<DKey>(msg).to_hex(),
        "formula" => hash_raw::<DFormula>(msg).to_hex(),
        "vocab" => hash_raw::<DVocab>(msg).to_hex(),
        "proposal" => hash_raw::<DProposal>(msg).to_hex(),
        "obligation" => hash_raw::<DObligation>(msg).to_hex(),
        "certificate" => hash_raw::<DCertificate>(msg).to_hex(),
        "checker" => hash_raw::<DChecker>(msg).to_hex(),
        "model" => hash_raw::<DModel>(msg).to_hex(),
        "ground" => hash_raw::<DGround>(msg).to_hex(),
        "event" => hash_raw::<DEvent>(msg).to_hex(),
        "fact" => hash_raw::<DFact>(msg).to_hex(),
        "policy" => hash_raw::<DPolicy>(msg).to_hex(),
        "contract" => hash_raw::<DContract>(msg).to_hex(),
        "action" => hash_raw::<DAction>(msg).to_hex(),
        "paraphrase" => hash_raw::<DParaphrase>(msg).to_hex(),
        "token" => hash_raw::<DToken>(msg).to_hex(),
        "assumption" => hash_raw::<DAssumption>(msg).to_hex(),
        other => panic!("domínio desconhecido nos vetores: {other}"),
    }
}

macro_rules! sig_domains {
    ($($ident:ident => $tag:expr),* $(,)?) => {
        $(pub struct $ident; impl SigDomain for $ident { const TAG: &'static [u8] = $tag; })*
    };
}
sig_domains! {
    SEvent => sig_tags::EVENT, SToken => sig_tags::TOKEN, SFact => sig_tags::FACT,
    SPolicy => sig_tags::POLICY, SContract => sig_tags::CONTRACT,
    SAuthorization => sig_tags::AUTHORIZATION, SOverride => sig_tags::OVERRIDE,
    SFidelity => sig_tags::FIDELITY,
}

/// Assina no domínio de assinatura chamado `name`.
pub fn sign_by_name(
    signer: &logos_crypto::FileSigner,
    name: &str,
    msg: &[u8],
) -> logos_crypto::Signature {
    use logos_crypto::Signer;
    match name {
        "event" => signer.sign::<SEvent>(msg),
        "token" => signer.sign::<SToken>(msg),
        "fact" => signer.sign::<SFact>(msg),
        "policy" => signer.sign::<SPolicy>(msg),
        "contract" => signer.sign::<SContract>(msg),
        "authorization" => signer.sign::<SAuthorization>(msg),
        "override" => signer.sign::<SOverride>(msg),
        "fidelity" => signer.sign::<SFidelity>(msg),
        other => panic!("domínio de assinatura desconhecido: {other}"),
    }
    .unwrap()
}
