//! Configuração de confiança por tópico (trust/v1, Etapa 6.2, ADR0063).
//!
//! `TrustConfig = [1, version, roots]`, `roots` em ordem estrita de `KeyId`, cada
//! raiz `[PublicKey, name, topics, level]` com tópicos em ordem estrita de bytes.
//! O objeto não carrega assinatura: a do admin está no evento
//! `TrustConfigPublished` (Authority, events.md §4), que fixa `h(TrustConfig)`.

use logos_canon::{Array, Canon, CanonDecode, CanonError, Decoder, Text, Value};
use logos_crypto::{Domain, KeyId, PublicKey, tags};

/// Teto de bytes de uma configuração no CAS.
pub const MAX_TRUST_BYTES: usize = 1024 * 1024;
/// Teto de emissores por configuração.
pub const MAX_TRUST_ROOTS: usize = 1024;
/// Teto de tópicos por emissor.
pub const MAX_TRUST_TOPICS: usize = 256;
const MAX_FIELD_BYTES: usize = 4096;

/// Um emissor confiável e os tópicos que ele pode atestar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustRoot {
    /// Chave pública do emissor (o `KeyId` é derivado dela).
    pub issuer: PublicKey,
    /// Nome legível (ex.: `db-monitor`); não participa da regra.
    pub name: Text,
    /// Tópicos atestáveis, em ordem estrita, comparados por igualdade exata.
    pub topics: Vec<Text>,
    /// Nível declarado pelo admin; informativo nesta etapa (sem efeito na regra).
    pub level: u64,
}

/// Configuração versionada; a vigente é a de maior versão publicada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustConfig {
    /// Versão, estritamente crescente entre publicações (a primeira ≥ 1).
    pub version: u64,
    /// Emissores em ordem estrita de `KeyId`.
    pub roots: Vec<TrustRoot>,
}

impl Domain for TrustConfig {
    const TAG: &'static [u8] = tags::TRUST;
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

impl TrustConfig {
    /// Constrói e confere as invariantes do formato (ordem, unicidade, tetos).
    ///
    /// # Errors
    /// Versão 0, listas vazias/fora de ordem/repetidas, campos fora dos tetos.
    pub fn new(version: u64, roots: Vec<TrustRoot>) -> Result<Self, CanonError> {
        let config = Self { version, roots };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), CanonError> {
        if self.version == 0 || self.roots.is_empty() || self.roots.len() > MAX_TRUST_ROOTS {
            return Err(CanonError::WrongType);
        }
        let mut previous: Option<KeyId> = None;
        for root in &self.roots {
            let id = root.issuer.key_id().map_err(|_| CanonError::WrongType)?;
            if previous.is_some_and(|p| p.as_bytes() >= id.as_bytes()) {
                return Err(CanonError::WrongType);
            }
            previous = Some(id);
            let name = root.name.as_str();
            if name.is_empty() || name.len() > MAX_FIELD_BYTES {
                return Err(CanonError::WrongType);
            }
            if root.topics.is_empty() || root.topics.len() > MAX_TRUST_TOPICS {
                return Err(CanonError::WrongType);
            }
            for topic in &root.topics {
                if topic.as_str().is_empty() || topic.as_str().len() > MAX_FIELD_BYTES {
                    return Err(CanonError::WrongType);
                }
            }
            if root
                .topics
                .windows(2)
                .any(|w| w[0].as_str().as_bytes() >= w[1].as_str().as_bytes())
            {
                return Err(CanonError::WrongType);
            }
        }
        Ok(())
    }

    /// O emissor `issuer` é confiável para `topic` nesta configuração.
    #[must_use]
    pub fn trusts(&self, issuer: &KeyId, topic: &str) -> bool {
        self.roots.iter().any(|root| {
            root.issuer.key_id().is_ok_and(|id| &id == issuer)
                && root.topics.iter().any(|t| t.as_str() == topic)
        })
    }
}

impl Canon for TrustConfig {
    fn to_value(&self) -> Result<Value, CanonError> {
        self.validate()?;
        let roots = self
            .roots
            .iter()
            .map(|root| {
                array(vec![
                    root.issuer.to_value()?,
                    root.name.to_value()?,
                    array(
                        root.topics
                            .iter()
                            .map(Canon::to_value)
                            .collect::<Result<_, _>>()?,
                    )?,
                    Value::Unsigned(root.level),
                ])
            })
            .collect::<Result<_, CanonError>>()?;
        array(vec![
            Value::Unsigned(1),
            Value::Unsigned(self.version),
            array(roots)?,
        ])
    }
}

impl CanonDecode for TrustConfig {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        let [Value::Unsigned(1), version, roots] = items(value)? else {
            return Err(CanonError::WrongType);
        };
        let roots = items(roots)?;
        if roots.len() > MAX_TRUST_ROOTS {
            return Err(CanonError::WrongType);
        }
        let mut decoded = Vec::new();
        for root in roots {
            let [issuer, name, topics, level] = items(root)? else {
                return Err(CanonError::WrongType);
            };
            let topics = items(topics)?;
            if topics.len() > MAX_TRUST_TOPICS {
                return Err(CanonError::WrongType);
            }
            decoded.push(TrustRoot {
                issuer: PublicKey::from_value(issuer)?,
                name: bounded_text(name)?,
                topics: topics.iter().map(bounded_text).collect::<Result<_, _>>()?,
                level: u64::from_value(level)?,
            });
        }
        Self::new(u64::from_value(version)?, decoded)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, CanonError> {
        if decoder.remaining() > MAX_TRUST_BYTES {
            return Err(CanonError::WrongType);
        }
        Self::from_value(&decoder.value()?)
    }
}
