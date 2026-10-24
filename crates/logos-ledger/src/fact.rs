//! Objeto `Fact` (fact/v1, Etapa 6.1, ADR0061): afirmação assinada por um emissor.
//!
//! Identidade `h(Fact)` = hash com domínio `logos/fact/v1\0` dos bytes canônicos
//! completos (inclusive a assinatura). A assinatura do emissor cobre o corpo sem a
//! assinatura, com a tag `logos/sig/fact/v1\0`. Assinatura válida não basta para
//! citar: o kernel só registra fatos com `Trusted(emissor, tópico)` vigente (6.2), e a
//! validade (épocas ou instante) decide quando o fato é citável.

use logos_air::{Formula, Vocabulary};
use logos_canon::{Array, Canon, CanonDecode, CanonError, Decoder, Text, Value, to_canon};
use logos_core::EpochRange;
use logos_crypto::{
    CryptoError, Domain, Hash, PublicKey, SigDomain, Signature, Signer, sig_tags, tags,
};

/// Teto de bytes de um fato no CAS (mesma ordem de um evento).
pub const MAX_FACT_BYTES: usize = 1024 * 1024;
const MAX_FIELD_BYTES: usize = 4096;

/// Janela em que o fato é aplicável.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Validity {
    /// Épocas inclusivas `[first, last]`.
    Epochs(EpochRange),
    /// Até o instante Unix (segundos) inclusive, em qualquer época a partir do registro.
    Until(u64),
}

/// Versão observada de um recurso versionado (gancho de TOCTOU da Fase 8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceVersion {
    /// Identificador do recurso.
    pub resource: Text,
    /// Versão observada.
    pub version: Text,
}

/// Fato assinado. Construir com [`Fact::sign`] ou decodificar do CAS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    /// Fórmula afirmada.
    pub formula: Formula,
    /// Vocabulário da fórmula.
    pub vocabulary: Hash<Vocabulary>,
    /// Chave pública do emissor (o `KeyId` é derivado dela).
    pub issuer: PublicKey,
    /// Tópico declarado; a confiança é por tópico (trust/v1, 6.2).
    pub topic: Text,
    /// Instante Unix (segundos) da observação.
    pub observed_at: u64,
    /// Janela de aplicabilidade.
    pub valid: Validity,
    /// Versão observada do recurso, se houver.
    pub version: Option<ResourceVersion>,
    /// Assinatura do emissor sobre o corpo.
    pub sig: Signature,
}

impl Domain for Fact {
    const TAG: &'static [u8] = tags::FACT;
}

/// Domínio da assinatura do emissor.
pub struct FactSignature;

impl SigDomain for FactSignature {
    const TAG: &'static [u8] = sig_tags::FACT;
}

fn array(items: Vec<Value>) -> Result<Value, CanonError> {
    Array::new(items).map(Value::Array)
}

fn items(value: &Value) -> Result<&[Value], CanonError> {
    match value {
        Value::Array(array) => Ok(array.items()),
        _ => Err(CanonError::WrongType),
    }
}

fn bounded_text(value: &Value) -> Result<Text, CanonError> {
    let text = Text::from_value(value)?;
    if text.as_str().is_empty() || text.as_str().len() > MAX_FIELD_BYTES {
        return Err(CanonError::WrongType);
    }
    Ok(text)
}

impl Validity {
    fn to_value(self) -> Result<Value, CanonError> {
        match self {
            Self::Epochs(range) => array(vec![
                Value::Unsigned(0),
                Value::Unsigned(range.first()),
                Value::Unsigned(range.last()),
            ]),
            Self::Until(instant) => array(vec![Value::Unsigned(1), Value::Unsigned(instant)]),
        }
    }

    fn from_value(value: &Value) -> Result<Self, CanonError> {
        match items(value)? {
            [Value::Unsigned(0), first, last] => {
                let range = EpochRange::new(u64::from_value(first)?, u64::from_value(last)?)
                    .map_err(|_| CanonError::WrongType)?;
                Ok(Self::Epochs(range))
            }
            [Value::Unsigned(1), instant] => Ok(Self::Until(u64::from_value(instant)?)),
            _ => Err(CanonError::WrongType),
        }
    }

    /// Aplicável na época `epoch` no instante `now`, para um fato registrado na
    /// época `registered`. Fora da janela o fato existe no histórico mas não é premissa.
    #[must_use]
    pub fn applicable(self, epoch: u64, now: u64, registered: u64) -> bool {
        match self {
            Self::Epochs(range) => range.contains(epoch),
            Self::Until(instant) => epoch >= registered && now <= instant,
        }
    }
}

impl Fact {
    fn body(&self) -> Result<Vec<Value>, CanonError> {
        let version = match &self.version {
            None => array(vec![])?,
            Some(v) => array(vec![array(vec![
                v.resource.to_value()?,
                v.version.to_value()?,
            ])?])?,
        };
        Ok(vec![
            Value::Unsigned(1),
            self.formula.to_value()?,
            self.vocabulary.to_value()?,
            self.issuer.to_value()?,
            self.topic.to_value()?,
            Value::Unsigned(self.observed_at),
            self.valid.to_value()?,
            version,
        ])
    }

    /// Bytes assinados pelo emissor: o corpo canônico sem a assinatura.
    ///
    /// # Errors
    /// Fórmula/campos sem codificação canônica.
    pub fn signing_bytes(&self) -> Result<Vec<u8>, CanonError> {
        to_canon(&Value::Array(Array::new(self.body()?)?))
    }

    /// Assina um fato novo com a chave do emissor.
    ///
    /// # Errors
    /// Codificação, chave pública ou assinatura.
    #[allow(clippy::too_many_arguments)]
    pub fn sign<S: Signer>(
        signer: &S,
        formula: Formula,
        vocabulary: Hash<Vocabulary>,
        topic: Text,
        observed_at: u64,
        valid: Validity,
        version: Option<ResourceVersion>,
    ) -> Result<Self, FactError> {
        let mut fact = Self {
            formula,
            vocabulary,
            issuer: signer.public_key().clone(),
            topic,
            observed_at,
            valid,
            version,
            sig: signer.sign::<FactSignature>(b"")?,
        };
        fact.sig = signer.sign::<FactSignature>(&fact.signing_bytes()?)?;
        Ok(fact)
    }

    /// Confere a assinatura contra a chave pública declarada. Não diz se o emissor
    /// é confiável: isso é da configuração de confiança no ledger (6.2).
    ///
    /// # Errors
    /// Assinatura inválida ou codificação.
    pub fn verify_signature(&self) -> Result<(), FactError> {
        logos_crypto::verify::<FactSignature>(&self.signing_bytes()?, &self.sig, &self.issuer)?;
        Ok(())
    }
}

impl Canon for Fact {
    fn to_value(&self) -> Result<Value, CanonError> {
        let mut body = self.body()?;
        body.push(self.sig.to_value()?);
        array(body)
    }
}

impl CanonDecode for Fact {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let [
            Value::Unsigned(1),
            formula,
            vocabulary,
            issuer,
            topic,
            observed_at,
            valid,
            version,
            sig,
        ] = items(value)?
        else {
            return Err(CanonError::WrongType);
        };
        let version = match items(version)? {
            [] => None,
            [pair] => match items(pair)? {
                [resource, version] => Some(ResourceVersion {
                    resource: bounded_text(resource)?,
                    version: bounded_text(version)?,
                }),
                _ => return Err(CanonError::WrongType),
            },
            _ => return Err(CanonError::WrongType),
        };
        Ok(Self {
            formula: Formula::from_value(formula)?,
            vocabulary: match vocabulary {
                Value::Bytes(bytes) => Hash::from_bytes(
                    <[u8; 32]>::try_from(bytes.as_slice()).map_err(|_| CanonError::WrongType)?,
                ),
                _ => return Err(CanonError::WrongType),
            },
            issuer: PublicKey::from_value(issuer)?,
            topic: bounded_text(topic)?,
            observed_at: u64::from_value(observed_at)?,
            valid: Validity::from_value(valid)?,
            version,
            sig: Signature::from_value(sig)?,
        })
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, CanonError> {
        if decoder.remaining() > MAX_FACT_BYTES {
            return Err(CanonError::WrongType);
        }
        Self::from_value(&decoder.value()?)
    }
}

/// Erro explícito de fato; nunca um fato "parecido".
#[derive(Debug)]
pub enum FactError {
    /// Bytes fora de fact/v1.
    Canon(CanonError),
    /// Assinatura ou chave.
    Crypto(CryptoError),
}

impl std::fmt::Display for FactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Canon(e) => write!(f, "fact encoding: {e}"),
            Self::Crypto(e) => write!(f, "fact signature: {e}"),
        }
    }
}

impl std::error::Error for FactError {}

impl From<CanonError> for FactError {
    fn from(e: CanonError) -> Self {
        Self::Canon(e)
    }
}

impl From<CryptoError> for FactError {
    fn from(e: CryptoError) -> Self {
        Self::Crypto(e)
    }
}
