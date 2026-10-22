//! kernel-protocol/v1: candidatos canônicos estritos, sem autoridade.

use crate::TransportError as Error;
use logos_air::{Formula, Limits, Vocabulary};
use logos_canon::{Array, Canon, CanonDecode, Text, Value, from_canon, to_canon};
use logos_crypto::{Domain, Hash};
use logos_ledger::{Certificate, Event, EventProposal, ModelObject, TaintedText};
use logos_obligation::Obligation;

/// Teto inclusivo do payload de requisição.
pub const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;
/// Teto inclusivo do payload de resposta (evento inteiro, sem truncamento).
pub const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024 + 1024;
/// Limite inclusivo da fórmula AIR embutida.
pub const MAX_AIR_BYTES: usize = 1024 * 1024;

/// Referência candidata a objeto; nome/hash não provam existência ou validade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectRef<T: Domain> {
    /// Objeto já publicado no CAS.
    Cas(Hash<T>),
    /// Upload do agente, a capturar e conferir antes de publicar.
    Staged {
        /// Nome opaco, sem caminho.
        name: Text,
        /// Hash com domínio esperado.
        hash: Hash<T>,
    },
}

/// Consulta somente de leitura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Query {
    /// Cabeça confirmada.
    Head,
    /// Categoria/época/retratação de um evento.
    Status(Hash<Event>),
}

/// Mensagens candidatas v1. Nenhuma constrói Claim ou obrigação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// Proposta com os cinco campos da1.6.
    Propose(EventProposal),
    /// Candidato a certificado de prova.
    Certify {
        /// Obrigação identificada pelo kernel.
        obligation: Hash<Obligation>,
        /// Formato candidato.
        format: Text,
        /// Certificado candidato.
        certificate: ObjectRef<Certificate>,
    },
    /// Candidato a contramodelo.
    Refute {
        /// Obrigação alvo.
        obligation: Hash<Obligation>,
        /// Modelo candidato.
        model: ObjectRef<ModelObject>,
    },
    /// Candidato a testemunha de consistência.
    Witness {
        /// Obrigação alvo.
        obligation: Hash<Obligation>,
        /// Modelo candidato.
        model: ObjectRef<ModelObject>,
    },
    /// Pressuposto candidato, sem autor fornecido pelo agente.
    Assume {
        /// Vocabulário pretendido.
        vocabulary: Hash<Vocabulary>,
        /// Fórmula candidata.
        formula: Formula,
        /// Escopo explícito.
        scope: Text,
    },
    /// Consulta.
    Query(Query),
    /// Fluxo limitado de eventos comprometidos, com cursor de continuação.
    Subscribe {
        /// Primeira sequência desejada.
        from_seq: u64,
        /// Quantidade máxima,1..256.
        max_events: u64,
    },
}

/// Envelope sem assinatura nem autoridade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Correlação fornecida pelo cliente, sem semântica de idempotência.
    pub id: u64,
    /// Comando candidato.
    pub message: Message,
}

fn array(items: Vec<Value>) -> Result<Value, Error> {
    Ok(Value::Array(Array::new(items)?))
}
fn items(value: &Value) -> Result<&[Value], Error> {
    if let Value::Array(v) = value {
        Ok(v.items())
    } else {
        Err(Error::Invalid("expected array"))
    }
}
fn uint(value: &Value) -> Result<u64, Error> {
    Ok(u64::from_value(value)?)
}
fn digest<T: Domain>(value: &Value) -> Result<Hash<T>, Error> {
    Ok(Hash::from_value(value)?)
}
fn bounded_text(value: &Value, max: usize) -> Result<Text, Error> {
    let text = Text::from_value(value)?;
    if text.as_str().len() > max {
        return Err(Error::Limit("text"));
    }
    Ok(text)
}
fn formula(value: &Value) -> Result<Formula, Error> {
    let Value::Bytes(bytes) = value else {
        return Err(Error::Invalid("AIR must be bytes"));
    };
    if bytes.len() > MAX_AIR_BYTES {
        return Err(Error::Limit("AIR bytes"));
    }
    Formula::decode_with_limits(bytes, Limits::default()).map_err(|_| Error::Invalid("invalid AIR"))
}
fn refs<T: Domain>(value: &Value) -> Result<Vec<Hash<T>>, Error> {
    let values = items(value)?;
    if values.len() > 1024 {
        return Err(Error::Limit("references"));
    }
    values.iter().map(digest).collect()
}
/// Confere nome opaco de upload, sem aceitar componentes de caminho.
#[must_use]
pub fn valid_staging_name(name: &str) -> bool {
    name.len() == 64
        && name
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn object<T: Domain>(value: &Value) -> Result<ObjectRef<T>, Error> {
    match items(value)? {
        [Value::Unsigned(0), hash] => Ok(ObjectRef::Cas(digest(hash)?)),
        [Value::Unsigned(1), name, hash] => {
            let name = bounded_text(name, 64)?;
            if !valid_staging_name(name.as_str()) {
                return Err(Error::Invalid("staging name"));
            }
            Ok(ObjectRef::Staged {
                name,
                hash: digest(hash)?,
            })
        }
        _ => Err(Error::Invalid("object reference")),
    }
}
fn object_value<T: Domain>(reference: &ObjectRef<T>) -> Result<Value, Error> {
    match reference {
        ObjectRef::Cas(h) => array(vec![Value::Unsigned(0), h.to_value()?]),
        ObjectRef::Staged { name, hash } => {
            array(vec![Value::Unsigned(1), name.to_value()?, hash.to_value()?])
        }
    }
}

impl Request {
    /// Lê payload completo, rejeitando campos/tipos/versões extras e excesso.
    /// # Errors
    /// Canon, estrutura, versão ou limites inválidos.
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(Error::Limit("request bytes"));
        }
        let value = from_canon::<Value>(bytes)?;
        let [Value::Unsigned(1), id, Value::Unsigned(tag), payload] = items(&value)? else {
            return Err(Error::Invalid("request envelope/version"));
        };
        let id = uint(id)?;
        let fields = items(payload)?;
        let message = match (*tag, fields) {
            (0, [Value::Bytes(text), goal, cites, speculative, vocabulary]) => {
                if text.len() > 64 * 1024 {
                    return Err(Error::Limit("tainted text"));
                }
                let text =
                    std::str::from_utf8(text).map_err(|_| Error::Invalid("tainted UTF-8"))?;
                let cited = refs::<Event>(cites)?;
                let speculative = items(speculative)?;
                if speculative.len() > 1024 {
                    return Err(Error::Limit("speculative references"));
                }
                let speculative_on = speculative
                    .iter()
                    .map(|v| <[u8; 32]>::from_value(v).map_err(Error::from))
                    .collect::<Result<_, _>>()?;
                Message::Propose(EventProposal {
                    text_pt: TaintedText::new(text.to_owned())
                        .map_err(|_| Error::Limit("tainted text"))?,
                    goal: formula(goal)?,
                    cites: cited,
                    speculative_on,
                    vocabulary: digest(vocabulary)?,
                })
            }
            (1, [obligation, format, certificate]) => {
                let format = bounded_text(format, 256)?;
                if format.as_str().is_empty() {
                    return Err(Error::Invalid("empty format"));
                }
                Message::Certify {
                    obligation: digest(obligation)?,
                    format,
                    certificate: object(certificate)?,
                }
            }
            (2, [obligation, model]) => Message::Refute {
                obligation: digest(obligation)?,
                model: object(model)?,
            },
            (3, [obligation, model]) => Message::Witness {
                obligation: digest(obligation)?,
                model: object(model)?,
            },
            (4, [vocabulary, f, scope]) => Message::Assume {
                vocabulary: digest(vocabulary)?,
                formula: formula(f)?,
                scope: bounded_text(scope, 64 * 1024)?,
            },
            (5, [Value::Unsigned(0)]) => Message::Query(Query::Head),
            (5, [Value::Unsigned(1), reference]) => {
                Message::Query(Query::Status(digest(reference)?))
            }
            (6, [from, count]) => {
                let max_events = uint(count)?;
                if !(1..=256).contains(&max_events) {
                    return Err(Error::Limit("subscription count"));
                }
                Message::Subscribe {
                    from_seq: uint(from)?,
                    max_events,
                }
            }
            _ => return Err(Error::Invalid("unknown message/type/arity")),
        };
        Ok(Self { id, message })
    }

    /// Codifica somente objetos que o decoder v1 aceita, sem truncar campos.
    /// # Errors
    /// Estrutura, codificação ou limites inválidos.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let (tag, fields) = match &self.message {
            Message::Propose(p) => (
                0,
                vec![
                    Value::Bytes(p.text_pt.as_bytes().to_vec()),
                    Value::Bytes(to_canon(&p.goal)?),
                    p.cites.to_value()?,
                    p.speculative_on.to_value()?,
                    p.vocabulary.to_value()?,
                ],
            ),
            Message::Certify {
                obligation,
                format,
                certificate,
            } => (
                1,
                vec![
                    obligation.to_value()?,
                    format.to_value()?,
                    object_value(certificate)?,
                ],
            ),
            Message::Refute { obligation, model } => {
                (2, vec![obligation.to_value()?, object_value(model)?])
            }
            Message::Witness { obligation, model } => {
                (3, vec![obligation.to_value()?, object_value(model)?])
            }
            Message::Assume {
                vocabulary,
                formula,
                scope,
            } => (
                4,
                vec![
                    vocabulary.to_value()?,
                    Value::Bytes(to_canon(formula)?),
                    scope.to_value()?,
                ],
            ),
            Message::Query(Query::Head) => (5, vec![Value::Unsigned(0)]),
            Message::Query(Query::Status(h)) => (5, vec![Value::Unsigned(1), h.to_value()?]),
            Message::Subscribe {
                from_seq,
                max_events,
            } => (6, vec![from_seq.to_value()?, max_events.to_value()?]),
        };
        let bytes = to_canon(&array(vec![
            Value::Unsigned(1),
            self.id.to_value()?,
            Value::Unsigned(tag),
            array(fields)?,
        ])?)?;
        // Encoder não é caminho para introduzir formas que o decoder rejeitaria.
        Self::decode(&bytes)?;
        Ok(bytes)
    }
}

/// Decodifica um único frame completo em memória, inclusive seu comprimento.
/// # Errors
/// Prefixo/tamanho/excesso/bytes sobrando ou payload inválidos.
pub fn decode_frame(bytes: &[u8]) -> Result<Request, Error> {
    let (prefix, payload) = bytes
        .split_at_checked(4)
        .ok_or(Error::Invalid("short frame"))?;
    let prefix: [u8; 4] = prefix
        .try_into()
        .map_err(|_| Error::Invalid("frame prefix"))?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 || length > MAX_REQUEST_BYTES {
        return Err(Error::Limit("frame length"));
    }
    if payload.len() != length {
        return Err(Error::Invalid("frame length mismatch"));
    }
    Request::decode(payload)
}

/// Produz prefixo big-endian e payload limitado.
/// # Errors
/// Objeto inválido ou excesso.
pub fn encode_frame(request: &Request) -> Result<Vec<u8>, Error> {
    let payload = request.encode()?;
    let length = u32::try_from(payload.len()).map_err(|_| Error::Limit("frame length"))?;
    let mut out = length.to_be_bytes().to_vec();
    out.extend_from_slice(&payload);
    Ok(out)
}

pub(crate) fn response(id: u64, result: Result<Value, &Error>) -> Result<Vec<u8>, Error> {
    let (tag, value) = match result {
        Ok(v) => (1, v),
        // Único erro com detalhe: [5, h_O, h(Refuted), h(M)] (kernel-protocol, 5.6).
        Err(Error::AlreadyRefuted {
            obligation,
            refuted,
            model,
        }) => (
            0,
            array(vec![
                Value::Unsigned(5),
                obligation.to_value()?,
                refuted.to_value()?,
                model.to_value()?,
            ])?,
        ),
        Err(e) => (0, Value::Unsigned(e.code())),
    };
    let bytes = to_canon(&array(vec![
        Value::Unsigned(1),
        Value::Unsigned(id),
        Value::Unsigned(tag),
        value,
    ])?)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(Error::Limit("response bytes"));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use logos_crypto::Hash;

    #[test]
    fn already_refuted_detail_matches_independent_vector() {
        let root: serde_json::Value = serde_json::from_str(include_str!(
            "../../../spec/vectors/kernel-protocol/already-refuted-v1.json"
        ))
        .unwrap();
        let unhex = |s: &str| -> Vec<u8> {
            (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
                .collect()
        };
        let digest = |field: &str, v: &serde_json::Value| -> [u8; 32] {
            unhex(v[field].as_str().unwrap()).try_into().unwrap()
        };
        for v in root["vectors"].as_array().unwrap() {
            let error = Error::AlreadyRefuted {
                obligation: Hash::from_bytes(digest("obligation_hex", v)),
                refuted: Hash::from_bytes(digest("refuted_hex", v)),
                model: Hash::from_bytes(digest("model_hex", v)),
            };
            let bytes = response(v["request_id"].as_u64().unwrap(), Err(&error)).unwrap();
            assert_eq!(bytes, unhex(v["response_hex"].as_str().unwrap()));
            assert_eq!(error.code(), 5);
        }
    }
}
