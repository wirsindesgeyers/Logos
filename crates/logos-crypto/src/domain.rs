//! Hash com separação de domínio: `H_T(x) = SHA-256(tag_T ‖ bytes)`.
//!
//! Cada tipo de objeto tem sua própria tag ([`Domain`]); o tipo [`Hash<T>`] carrega o domínio
//! no sistema de tipos, então um hash de fórmula nunca é aceito onde se espera um hash de
//! obrigação. As tags reservadas estão em `spec/domains.md` e em [`tags`].
//!
//! ```compile_fail,E0308
//! use logos_crypto::{Domain, Hash, hash_raw};
//! struct A; impl Domain for A { const TAG: &'static [u8] = b"logos/a/v1\0"; }
//! struct B; impl Domain for B { const TAG: &'static [u8] = b"logos/b/v1\0"; }
//! fn wants_b(_: Hash<B>) {}
//! wants_b(hash_raw::<A>(b"x")); // Hash<A> onde se espera Hash<B>: não compila
//! ```
//!
//! ```compile_fail,E0308
//! use logos_crypto::{Domain, ExternalDigest, Hash};
//! struct A; impl Domain for A { const TAG: &'static [u8] = b"logos/a/v1\0"; }
//! fn wants_hash(_: Hash<A>) {}
//! wants_hash(ExternalDigest::of(b"x")); // digest sem tag onde se espera Hash<A>: não compila
//! ```

use core::fmt;
use core::marker::PhantomData;
use logos_canon::{Bytes, Canon, CanonDecode, CanonError, Value, to_canon};
use sha2::{Digest, Sha256};

/// Domínio de hash: um tipo de objeto com sua tag.
///
/// A tag DEVE ser `logos/<nome>/v<k>` seguida de um byte `0x00`; assim nenhuma tag é prefixo
/// de outra e `tag ‖ bytes` nunca é ambíguo (ver `spec/domains.md`).
pub trait Domain {
    /// Bytes da tag, incluindo o `0x00` final.
    const TAG: &'static [u8];
}

/// Tags de domínio reservadas (espelham a tabela de `spec/domains.md`; um teste confere).
pub mod tags {
    /// `KeyId`: hash da codificação canônica de uma chave pública.
    pub const KEY: &[u8] = b"logos/key/v1\0";
    /// Fórmulas da AIR (Etapa 1.1).
    pub const FORMULA: &[u8] = b"logos/formula/v1\0";
    /// Vocabulário versionado (1.2).
    pub const VOCAB: &[u8] = b"logos/vocab/v1\0";
    /// Proposta do agente (1.6).
    pub const PROPOSAL: &[u8] = b"logos/proposal/v1\0";
    /// Obrigação canônica `Γ ⊢ φ` (1.5).
    pub const OBLIGATION: &[u8] = b"logos/obligation/v1\0";
    /// Certificado de prover, bytes crus (2.3, 4.4).
    pub const CERTIFICATE: &[u8] = b"logos/certificate/v1\0";
    /// Binário de checker, bytes crus (4.5, 5.4).
    pub const CHECKER: &[u8] = b"logos/checker/v1\0";
    /// Modelo finito / contramodelo (2.1).
    pub const MODEL: &[u8] = b"logos/model/v1\0";
    /// Saída proposicional canônica do grounder (ADR0027).
    pub const GROUND: &[u8] = b"logos/ground/v1\0";
    /// Evento do ledger (3.1).
    pub const EVENT: &[u8] = b"logos/event/v1\0";
    /// Fato atestado (6.1).
    pub const FACT: &[u8] = b"logos/fact/v1\0";
    /// Política e raiz Merkle (7.1, 7.2).
    pub const POLICY: &[u8] = b"logos/policy/v1\0";
    /// Contrato de ferramenta (8.1).
    pub const CONTRACT: &[u8] = b"logos/contract/v1\0";
    /// Termo formal de uma ação `a` (8.2).
    pub const ACTION: &[u8] = b"logos/action/v1\0";
    /// Paráfrase determinística (9.1).
    pub const PARAPHRASE: &[u8] = b"logos/paraphrase/v1\0";
    /// Token de ação (8.3).
    pub const TOKEN: &[u8] = b"logos/token/v1\0";
    /// Assumption com escopo (5.7).
    pub const ASSUMPTION: &[u8] = b"logos/assumption/v1\0";

    /// `(nome, tag, codificação)` de todos os domínios reservados; `"raw"` = bytes crus.
    pub const RESERVED: [(&str, &[u8], &str); 17] = [
        ("key", KEY, "canonical"),
        ("formula", FORMULA, "canonical"),
        ("vocab", VOCAB, "canonical"),
        ("proposal", PROPOSAL, "canonical"),
        ("obligation", OBLIGATION, "canonical"),
        ("certificate", CERTIFICATE, "raw"),
        ("checker", CHECKER, "raw"),
        ("model", MODEL, "canonical"),
        ("ground", GROUND, "canonical"),
        ("event", EVENT, "canonical"),
        ("fact", FACT, "canonical"),
        ("policy", POLICY, "canonical"),
        ("contract", CONTRACT, "canonical"),
        ("action", ACTION, "canonical"),
        ("paraphrase", PARAPHRASE, "canonical"),
        ("token", TOKEN, "canonical"),
        ("assumption", ASSUMPTION, "canonical"),
    ];
}

/// Hash de 32 bytes no domínio `T`.
pub struct Hash<T: Domain> {
    bytes: [u8; 32],
    domain: PhantomData<fn() -> T>,
}

impl<T: Domain> Hash<T> {
    /// Tamanho em bytes.
    pub const LEN: usize = 32;

    /// Reconstrói um hash lido de fora (ledger, rede). Não prova que foi calculado neste domínio.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            bytes,
            domain: PhantomData,
        }
    }

    /// Os 32 bytes do digest.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Hexadecimal minúsculo.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl<T: Domain> Clone for Hash<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Domain> Copy for Hash<T> {}
impl<T: Domain> PartialEq for Hash<T> {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}
impl<T: Domain> Eq for Hash<T> {}
impl<T: Domain> PartialOrd for Hash<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: Domain> Ord for Hash<T> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.bytes.cmp(&other.bytes)
    }
}
impl<T: Domain> core::hash::Hash for Hash<T> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
    }
}
impl<T: Domain> fmt::Debug for Hash<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", self.to_hex())
    }
}
impl<T: Domain> fmt::Display for Hash<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl<T: Domain> Canon for Hash<T> {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(Value::Bytes(self.bytes.to_vec()))
    }
}

impl<T: Domain> CanonDecode for Hash<T> {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        Bytes::from_value(value)
            .and_then(|b| <[u8; 32]>::try_from(b.0.as_slice()).map_err(|_| CanonError::WrongType))
            .map(Self::from_bytes)
    }
}

/// `SHA-256(tag_T ‖ canon(x))`.
///
/// # Errors
/// Os de [`Canon::encode`]: um objeto sem codificação canônica não tem hash.
pub fn hash<T: Domain + Canon>(x: &T) -> Result<Hash<T>, CanonError> {
    Ok(hash_raw::<T>(&to_canon(x)?))
}

/// `SHA-256(tag_T ‖ bytes)` para objetos que não são CBOR canônico (binários de checker,
/// certificados de prover…). Para objetos canônicos, `hash_raw::<T>(canon(x)) == hash(x)`.
#[must_use]
pub fn hash_raw<T: Domain>(bytes: &[u8]) -> Hash<T> {
    let mut hasher = RawHasher::<T>::new();
    hasher.update(bytes);
    hasher.finalize()
}

/// Versão incremental de [`hash_raw`], para arquivos grandes (certificados de gigabytes).
pub struct RawHasher<T: Domain> {
    inner: Sha256,
    domain: PhantomData<fn() -> T>,
}

impl<T: Domain> RawHasher<T> {
    /// Começa um hash já alimentado com a tag do domínio.
    #[must_use]
    pub fn new() -> Self {
        let mut inner = Sha256::new();
        inner.update(T::TAG);
        Self {
            inner,
            domain: PhantomData,
        }
    }

    /// Acrescenta bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
    }

    /// Termina e devolve o hash.
    #[must_use]
    pub fn finalize(self) -> Hash<T> {
        Hash::from_bytes(self.inner.finalize().into())
    }
}

impl<T: Domain> Default for RawHasher<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// `SHA-256(bytes)` **sem tag**: só para conferir binários de terceiros contra o hash que eles
/// publicam (ex.: `cake_lpr`) e no manifesto da TCB. Nunca é aceito onde se espera `Hash<T>`;
/// o ledger sempre registra `hash_raw::<Checker>`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExternalDigest([u8; 32]);

impl ExternalDigest {
    /// SHA-256 puro de `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// Os 32 bytes do digest.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Hexadecimal minúsculo.
    #[must_use]
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl fmt::Debug for ExternalDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExternalDigest({})", self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct A;
    impl Domain for A {
        const TAG: &'static [u8] = b"logos/a/v1\0";
    }
    struct B;
    impl Domain for B {
        const TAG: &'static [u8] = b"logos/b/v1\0";
    }

    #[test]
    fn domains_separate_hashes_of_the_same_bytes() {
        assert_ne!(
            hash_raw::<A>(b"x").as_bytes(),
            hash_raw::<B>(b"x").as_bytes()
        );
    }

    #[test]
    fn external_digest_is_untagged_sha256() {
        // SHA-256("abc")
        assert_eq!(
            ExternalDigest::of(b"abc").to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_ne!(
            ExternalDigest::of(b"x").as_bytes(),
            hash_raw::<A>(b"x").as_bytes()
        );
    }

    #[test]
    fn incremental_equals_one_shot() {
        let mut h = RawHasher::<A>::new();
        h.update(b"ab");
        h.update(b"c");
        assert_eq!(h.finalize(), hash_raw::<A>(b"abc"));
    }

    #[test]
    fn reserved_tags_are_nul_terminated_and_prefix_free() {
        for (name, tag, _) in tags::RESERVED {
            assert_eq!(tag.last(), Some(&0), "{name}");
            assert!(tag.starts_with(b"logos/"), "{name}");
            assert_eq!(
                tag.iter().filter(|&&b| b == 0).count(),
                1,
                "{name}: NUL só no fim"
            );
        }
        for (i, (n1, t1, _)) in tags::RESERVED.iter().enumerate() {
            for (n2, t2, _) in &tags::RESERVED[i + 1..] {
                assert!(!t1.starts_with(t2) && !t2.starts_with(t1), "{n1} × {n2}");
            }
        }
    }
}
