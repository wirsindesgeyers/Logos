//! Algoritmos de assinatura, chaves públicas, assinaturas e `KeyId`.
//!
//! Toda chave pública e toda assinatura carregam o **identificador do algoritmo**: o tipo não
//! fica preso a Ed25519, e uma assinatura nunca é interpretada sob outro algoritmo. A v1 só
//! implementa Ed25519; ES256 (P-256, passkeys) está reservado para a Etapa 9.2.

use crate::domain::{Domain, Hash, hash, tags};
use crate::error::CryptoError;
use ed25519_dalek::{Signature as DalekSignature, VerifyingKey};
use logos_canon::{Bytes, Canon, CanonDecode, CanonError, Map, Text, Value};

/// Algoritmo de assinatura.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SigAlg {
    /// Ed25519 (RFC 8032), implementado.
    Ed25519,
    /// ECDSA P-256 com SHA-256, reservado (passkeys/WebAuthn, Etapa 9.2). Ainda não implementado.
    Es256,
}

impl SigAlg {
    /// Nome estável usado na codificação canônica.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Ed25519 => "ed25519",
            Self::Es256 => "es256",
        }
    }

    /// Procura um algoritmo pelo nome.
    ///
    /// # Errors
    /// [`CryptoError::UnknownAlgorithm`] se o nome não existe.
    pub fn from_name(name: &str) -> Result<Self, CryptoError> {
        match name {
            "ed25519" => Ok(Self::Ed25519),
            "es256" => Ok(Self::Es256),
            _ => Err(CryptoError::UnknownAlgorithm),
        }
    }

    /// Falha com [`CryptoError::UnsupportedAlgorithm`] se o algoritmo ainda não é implementado.
    ///
    /// # Errors
    /// Para `Es256`.
    pub fn ensure_implemented(self) -> Result<(), CryptoError> {
        match self {
            Self::Ed25519 => Ok(()),
            Self::Es256 => Err(CryptoError::UnsupportedAlgorithm(self)),
        }
    }
}

/// Chave pública com seu algoritmo.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PublicKey {
    alg: SigAlg,
    bytes: Vec<u8>,
}

impl PublicKey {
    /// Valida e embrulha uma chave pública. Ed25519: 32 bytes, ponto válido e **não** de ordem
    /// pequena (chaves fracas são recusadas).
    ///
    /// # Errors
    /// Tamanho errado, ponto inválido/fraco ou algoritmo não implementado.
    pub fn new(alg: SigAlg, bytes: &[u8]) -> Result<Self, CryptoError> {
        alg.ensure_implemented()?;
        let raw: [u8; 32] = bytes.try_into().map_err(|_| CryptoError::BadKeyLength)?;
        let key = VerifyingKey::from_bytes(&raw).map_err(|_| CryptoError::InvalidPublicKey)?;
        if key.is_weak() {
            return Err(CryptoError::InvalidPublicKey);
        }
        Ok(Self {
            alg,
            bytes: raw.to_vec(),
        })
    }

    /// O algoritmo.
    #[must_use]
    pub fn alg(&self) -> SigAlg {
        self.alg
    }

    /// Os bytes da chave.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// `KeyId = H_key(canon(PublicKey))`: inclui o algoritmo, então a mesma sequência de bytes
    /// sob outro algoritmo é outra identidade.
    ///
    /// # Errors
    /// Só se a codificação canônica falhar (não ocorre para chaves válidas).
    pub fn key_id(&self) -> Result<KeyId, CryptoError> {
        Ok(hash(self)?)
    }
}

/// Identidade de uma chave: hash da chave pública no domínio `key`.
pub type KeyId = Hash<PublicKey>;

impl Domain for PublicKey {
    const TAG: &'static [u8] = tags::KEY;
}

impl Canon for PublicKey {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(Value::Map(Map::new(vec![
            (
                Value::Text(Text::new("alg")?),
                Value::Text(Text::new(self.alg.name())?),
            ),
            (
                Value::Text(Text::new("key")?),
                Value::Bytes(self.bytes.clone()),
            ),
        ])?))
    }
}

impl CanonDecode for PublicKey {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let (alg, bytes) = alg_and_bytes(value, "key")?;
        Self::new(alg, &bytes).map_err(|_| CanonError::WrongType)
    }
}

/// Assinatura com seu algoritmo.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Signature {
    alg: SigAlg,
    bytes: Vec<u8>,
}

impl Signature {
    /// Valida o tamanho (Ed25519: 64 bytes) e embrulha.
    ///
    /// # Errors
    /// Tamanho errado ou algoritmo não implementado.
    pub fn new(alg: SigAlg, bytes: &[u8]) -> Result<Self, CryptoError> {
        alg.ensure_implemented()?;
        if bytes.len() != 64 {
            return Err(CryptoError::BadSignatureLength);
        }
        Ok(Self {
            alg,
            bytes: bytes.to_vec(),
        })
    }

    /// O algoritmo.
    #[must_use]
    pub fn alg(&self) -> SigAlg {
        self.alg
    }

    /// Os bytes da assinatura.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl Canon for Signature {
    fn to_value(&self) -> Result<Value, CanonError> {
        Ok(Value::Map(Map::new(vec![
            (
                Value::Text(Text::new("alg")?),
                Value::Text(Text::new(self.alg.name())?),
            ),
            (
                Value::Text(Text::new("sig")?),
                Value::Bytes(self.bytes.clone()),
            ),
        ])?))
    }
}

impl CanonDecode for Signature {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let (alg, bytes) = alg_and_bytes(value, "sig")?;
        Self::new(alg, &bytes).map_err(|_| CanonError::WrongType)
    }
}

/// Lê um mapa `{"alg": texto, <campo>: bytes}` com exatamente essas duas chaves.
fn alg_and_bytes(value: &Value, field: &str) -> Result<(SigAlg, Vec<u8>), CanonError> {
    let Value::Map(map) = value else {
        return Err(CanonError::WrongType);
    };
    if map.entries().len() != 2 {
        return Err(CanonError::WrongType);
    }
    let alg = match map.get_text("alg") {
        Some(Value::Text(t)) => SigAlg::from_name(t.as_str()).map_err(|_| CanonError::WrongType)?,
        _ => return Err(CanonError::WrongType),
    };
    let bytes = map.get_text(field).ok_or(CanonError::WrongType)?;
    Ok((alg, Bytes::from_value(bytes)?.0))
}

/// Domínio de assinatura: uma assinatura de um domínio nunca vale em outro (um evento nunca vale
/// como token). A mensagem assinada é `tag ‖ bytes`; as tags seguem a regra de [`Domain`].
pub trait SigDomain {
    /// Bytes da tag, incluindo o `0x00` final.
    const TAG: &'static [u8];
}

/// Tags de assinatura reservadas (espelham `spec/domains.md`).
pub mod sig_tags {
    /// Evento do ledger, assinado pelo kernel (3.2).
    pub const EVENT: &[u8] = b"logos/sig/event/v1\0";
    /// Token de ação, assinado pelo kernel (8.3).
    pub const TOKEN: &[u8] = b"logos/sig/token/v1\0";
    /// Fato, assinado por um atestador (6.1).
    pub const FACT: &[u8] = b"logos/sig/fact/v1\0";
    /// Raiz de política, assinada pela autoridade (7.2).
    pub const POLICY: &[u8] = b"logos/sig/policy/v1\0";
    /// Contrato de ferramenta, assinado pelo admin (8.1).
    pub const CONTRACT: &[u8] = b"logos/sig/contract/v1\0";
    /// Autorização do usuário sobre `h(a ‖ paráfrase)` (9.2).
    pub const AUTHORIZATION: &[u8] = b"logos/sig/authorization/v1\0";
    /// Override humano (8.7).
    pub const OVERRIDE: &[u8] = b"logos/sig/override/v1\0";
    /// Atestado de fidelidade (9.3, 15.2).
    pub const FIDELITY: &[u8] = b"logos/sig/fidelity/v1\0";

    /// `(nome, tag)` de todas as tags de assinatura reservadas.
    pub const RESERVED: [(&str, &[u8]); 8] = [
        ("event", EVENT),
        ("token", TOKEN),
        ("fact", FACT),
        ("policy", POLICY),
        ("contract", CONTRACT),
        ("authorization", AUTHORIZATION),
        ("override", OVERRIDE),
        ("fidelity", FIDELITY),
    ];
}

/// Verifica que `sig` é uma assinatura de `D::TAG ‖ bytes` por `key`.
///
/// Usa a verificação estrita do Ed25519 (rejeita assinaturas maleáveis e chaves fracas).
///
/// # Errors
/// [`CryptoError::AlgorithmMismatch`] se `sig` e `key` declaram algoritmos diferentes;
/// [`CryptoError::BadSignature`] se não verifica; erro tipado para algoritmo não implementado.
pub fn verify<D: SigDomain>(
    bytes: &[u8],
    sig: &Signature,
    key: &PublicKey,
) -> Result<(), CryptoError> {
    if sig.alg != key.alg {
        return Err(CryptoError::AlgorithmMismatch);
    }
    key.alg.ensure_implemented()?;
    let raw_key: [u8; 32] = key
        .bytes
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::BadKeyLength)?;
    let verifying =
        VerifyingKey::from_bytes(&raw_key).map_err(|_| CryptoError::InvalidPublicKey)?;
    let raw_sig: [u8; 64] = sig
        .bytes
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::BadSignatureLength)?;
    let message = tagged_message(D::TAG, bytes);
    verifying
        .verify_strict(&message, &DalekSignature::from_bytes(&raw_sig))
        .map_err(|_| CryptoError::BadSignature)
}

/// `tag ‖ bytes`.
pub(crate) fn tagged_message(tag: &[u8], bytes: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(tag.len() + bytes.len());
    message.extend_from_slice(tag);
    message.extend_from_slice(bytes);
    message
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Raw;
    impl SigDomain for Raw {
        const TAG: &'static [u8] = b"";
    }

    /// Só Ed25519 é construível pela API pública, então a checagem de algoritmo de `verify`
    /// (que protege a futura chegada do ES256) é exercitada construindo os valores aqui dentro.
    #[test]
    fn verify_rejects_signature_and_key_with_different_algorithms() {
        let key = PublicKey {
            alg: SigAlg::Es256,
            bytes: vec![0; 33],
        };
        let sig = Signature {
            alg: SigAlg::Ed25519,
            bytes: vec![0; 64],
        };
        assert_eq!(
            verify::<Raw>(b"", &sig, &key),
            Err(CryptoError::AlgorithmMismatch)
        );
    }

    #[test]
    fn verify_with_unimplemented_algorithm_is_a_typed_error_never_success() {
        let key = PublicKey {
            alg: SigAlg::Es256,
            bytes: vec![0; 33],
        };
        let sig = Signature {
            alg: SigAlg::Es256,
            bytes: vec![0; 64],
        };
        assert_eq!(
            verify::<Raw>(b"", &sig, &key),
            Err(CryptoError::UnsupportedAlgorithm(SigAlg::Es256))
        );
    }
}
