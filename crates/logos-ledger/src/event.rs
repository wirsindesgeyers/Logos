//! Envelope event/v1 e assinatura externa; não valida histórico ou prova lógica.

use core::fmt;
use logos_canon::{Array, Canon, CanonDecode, CanonError, Decoder, Value};
use logos_crypto::{
    CryptoError, Domain, Hash, KeyId, PublicKey, SigDomain, Signature, Signer, sig_tags, tags,
};

use crate::EventBody;

/// Máximo inclusivo de bytes de um evento canônico.
pub const MAX_EVENT_BYTES: usize = 16 * 1024 * 1024;

/// Erro explícito de estrutura, limite ou assinatura de evento.
#[derive(Debug)]
pub enum EventError {
    /// Falha de formato canônico.
    Canon(CanonError),
    /// Falha criptográfica ou do backend assinante.
    Crypto(CryptoError),
    /// Violação de invariante do catálogo.
    Invalid(&'static str),
    /// Limite inclusivo excedido.
    Limit(&'static str),
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Canon(error) => write!(f, "event canon: {error}"),
            Self::Crypto(error) => write!(f, "event crypto: {error}"),
            Self::Invalid(message) => write!(f, "invalid event: {message}"),
            Self::Limit(message) => write!(f, "event limit: {message}"),
        }
    }
}
impl std::error::Error for EventError {}
impl From<CanonError> for EventError {
    fn from(error: CanonError) -> Self {
        Self::Canon(error)
    }
}
impl From<CryptoError> for EventError {
    fn from(error: CryptoError) -> Self {
        Self::Crypto(error)
    }
}

/// Evento serializável; leitura/construção não conferem autoridade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Posição no ledger, iniciando em zero.
    pub seq: u64,
    /// Época da entrada.
    pub epoch: u64,
    /// Hash integral do evento anterior; ausente somente em Genesis.
    pub prev: Option<Hash<Event>>,
    /// Payload do catálogo v1.
    pub body: EventBody,
    /// Identidade da chave que deve verificar a assinatura externa.
    pub kernel_key: KeyId,
    /// Assinatura de todos os demais campos no domínio de evento.
    pub sig: Signature,
}

impl Domain for Event {
    const TAG: &'static [u8] = tags::EVENT;
}
impl SigDomain for Event {
    const TAG: &'static [u8] = sig_tags::EVENT;
}

fn header(
    seq: u64,
    epoch: u64,
    prev: Option<Hash<Event>>,
    body: &EventBody,
    key: KeyId,
) -> Result<(), EventError> {
    match body {
        EventBody::Genesis { kernel_key, .. } => {
            if seq != 0 || epoch != 0 || prev.is_some() || *kernel_key != key {
                return Err(EventError::Invalid("Genesis position/epoch/prev/key"));
            }
        }
        _ if seq == 0 || prev.is_none() => {
            return Err(EventError::Invalid("non-Genesis position/prev"));
        }
        EventBody::EpochAdvanced { new_epoch, .. } if *new_epoch != epoch => {
            return Err(EventError::Invalid("EpochAdvanced epoch mismatch"));
        }
        _ => {}
    }
    Ok(())
}

fn fields(
    seq: u64,
    epoch: u64,
    prev: Option<Hash<Event>>,
    body: &EventBody,
    key: KeyId,
) -> Result<Vec<Value>, CanonError> {
    Ok(vec![
        Value::Unsigned(1),
        Value::Unsigned(0),
        seq.to_value()?,
        epoch.to_value()?,
        prev.to_value()?,
        body.to_value()?,
        key.to_value()?,
    ])
}

fn head_size(length: u64) -> usize {
    match length {
        0..=23 => 1,
        24..=255 => 2,
        256..=65535 => 3,
        65536..=0xffff_ffff => 5,
        _ => 9,
    }
}

/// Conta bytes sem alocar/codificar uma segunda cópia do Value validado.
pub(crate) fn encoded_size(value: &Value) -> Result<usize, CanonError> {
    let add = |a: usize, b: usize| a.checked_add(b).ok_or(CanonError::IntOutOfRange);
    match value {
        Value::Null | Value::Bool(_) => Ok(1),
        Value::Unsigned(n) | Value::Negative(n) => Ok(head_size(*n)),
        Value::Bytes(bytes) => add(head_size(bytes.len() as u64), bytes.len()),
        Value::Text(text) => add(head_size(text.as_str().len() as u64), text.as_str().len()),
        Value::Array(array) => array
            .items()
            .iter()
            .try_fold(head_size(array.items().len() as u64), |total, item| {
                add(total, encoded_size(item)?)
            }),
        Value::Map(map) => map.entries().iter().try_fold(
            head_size(map.entries().len() as u64),
            |total, (key, value)| add(add(total, encoded_size(key)?)?, encoded_size(value)?),
        ),
    }
}

impl Event {
    /// Assina estrutura validada com chave do Signer; não escreve ledger.
    ///
    /// # Errors
    /// Estrutura/limites inválidos, codificação ou backend de assinatura.
    pub fn sign<S: Signer>(
        seq: u64,
        epoch: u64,
        prev: Option<Hash<Event>>,
        body: EventBody,
        signer: &S,
    ) -> Result<Self, EventError> {
        let kernel_key = signer.public_key().key_id()?;
        header(seq, epoch, prev, &body, kernel_key)?;
        let unsigned = Value::Array(Array::new(fields(seq, epoch, prev, &body, kernel_key)?)?);
        let size = encoded_size(&unsigned)?;
        // Uma assinatura Ed25519 canônica ocupa 83 bytes, com alg/sig explícitos.
        if size
            .checked_add(83)
            .is_none_or(|size| size > MAX_EVENT_BYTES)
        {
            return Err(EventError::Limit("wire bytes"));
        }
        let mut bytes = Vec::with_capacity(size);
        unsigned.encode_into(&mut bytes);
        let sig = signer.sign::<Event>(&bytes)?;
        Ok(Self {
            seq,
            epoch,
            prev,
            body,
            kernel_key,
            sig,
        })
    }

    /// Confere estrutura, KeyId e assinatura externa contra chave confiada pelo caller.
    ///
    /// # Errors
    /// Não corresponde a chave/assinatura esperada, estrutura ou limite inválido.
    pub fn verify(&self, public_key: &PublicKey) -> Result<(), EventError> {
        header(self.seq, self.epoch, self.prev, &self.body, self.kernel_key)?;
        if public_key.key_id()? != self.kernel_key {
            return Err(EventError::Invalid("kernel KeyId mismatch"));
        }
        let unsigned = Value::Array(Array::new(fields(
            self.seq,
            self.epoch,
            self.prev,
            &self.body,
            self.kernel_key,
        )?)?);
        let size = encoded_size(&unsigned)?;
        if size
            .checked_add(encoded_size(&self.sig.to_value()?)?)
            .is_none_or(|size| size > MAX_EVENT_BYTES)
        {
            return Err(EventError::Limit("wire bytes"));
        }
        let mut bytes = Vec::with_capacity(size);
        unsigned.encode_into(&mut bytes);
        logos_crypto::verify::<Event>(&bytes, &self.sig, public_key)?;
        Ok(())
    }

    /// Decodifica bytes completos, conferindo limite antes de materializar Value.
    /// Não verifica assinatura ou proveniência.
    ///
    /// # Errors
    /// Formato/limites/invariantes inválidos; bytes sobrando não são ignorados.
    pub fn decode(bytes: &[u8]) -> Result<Self, EventError> {
        if bytes.len() > MAX_EVENT_BYTES {
            return Err(EventError::Limit("wire bytes"));
        }
        Ok(logos_canon::from_canon(bytes)?)
    }
}

impl Canon for Event {
    fn to_value(&self) -> Result<Value, CanonError> {
        header(self.seq, self.epoch, self.prev, &self.body, self.kernel_key)
            .map_err(|_| CanonError::WrongType)?;
        let mut items = fields(self.seq, self.epoch, self.prev, &self.body, self.kernel_key)?;
        items.push(self.sig.to_value()?);
        let value = Value::Array(Array::new(items)?);
        if encoded_size(&value)? > MAX_EVENT_BYTES {
            return Err(CanonError::WrongType);
        }
        Ok(value)
    }
}

impl CanonDecode for Event {
    fn from_value(value: &Value) -> Result<Self, CanonError> {
        if encoded_size(value)? > MAX_EVENT_BYTES {
            return Err(CanonError::WrongType);
        }
        let Value::Array(array) = value else {
            return Err(CanonError::WrongType);
        };
        let [
            Value::Unsigned(1),
            Value::Unsigned(0),
            seq,
            epoch,
            prev,
            body,
            key,
            sig,
        ] = array.items()
        else {
            return Err(CanonError::WrongType);
        };
        let event = Self {
            seq: u64::from_value(seq)?,
            epoch: u64::from_value(epoch)?,
            prev: Option::<[u8; 32]>::from_value(prev)?.map(Hash::from_bytes),
            body: EventBody::from_value(body)?,
            kernel_key: KeyId::from_bytes(<[u8; 32]>::from_value(key)?),
            sig: Signature::from_value(sig)?,
        };
        header(
            event.seq,
            event.epoch,
            event.prev,
            &event.body,
            event.kernel_key,
        )
        .map_err(|_| CanonError::WrongType)?;
        Ok(event)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, CanonError> {
        if decoder.remaining() > MAX_EVENT_BYTES {
            return Err(CanonError::WrongType);
        }
        Self::from_value(&decoder.value()?)
    }
}
