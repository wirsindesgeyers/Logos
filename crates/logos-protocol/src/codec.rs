//! JSON fora da TCB; AIR canônico é conferido pelo decoder existente.

use std::{borrow::Cow, fmt};

use logos_air::{Formula, Limits};
use logos_canon::to_canon;
use logos_crypto::Hash;
use logos_obligation::PremiseRef;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    ClaimId, MAX_GOAL_BYTES, MAX_MESSAGE_BYTES, MAX_REFERENCES, MAX_TEXT_BYTES, PROPOSAL_VERSION,
    Proposal, ProtocolError, UntrustedText,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<'a> {
    version: u32,
    #[serde(borrow, deserialize_with = "object")]
    proposal: Payload<'a>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Payload<'a> {
    #[serde(borrow)]
    text_pt: Cow<'a, str>,
    #[serde(borrow)]
    goal: Cow<'a, str>,
    #[serde(deserialize_with = "cites")]
    cites: Vec<PremiseRef>,
    #[serde(deserialize_with = "speculative_on")]
    speculative_on: Vec<ClaimId>,
    vocab: Digest,
}

// Serde derive permite structs posicionais. Este protocolo exige mapas JSON.
fn object<'de, D: Deserializer<'de>, T: Deserialize<'de>>(deserializer: D) -> Result<T, D::Error> {
    struct Visitor<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Visitor<T> {
        type Value = T;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a JSON object")
        }
        fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_map(Visitor(std::marker::PhantomData))
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn byte(pair: &[u8; 2]) -> Option<u8> {
    Some(nibble(pair[0])? * 16 + nibble(pair[1])?)
}

struct Digest([u8; 32]);

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl de::Visitor<'_> for Visitor {
            type Value = Digest;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("64 lowercase ASCII hex digits")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Digest, E> {
                let mut bytes = [0; 32];
                if value.len() != 64 {
                    return Err(E::invalid_value(de::Unexpected::Str(value), &self));
                }
                for (out, pair) in bytes.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
                    *out = byte(pair)
                        .ok_or_else(|| E::invalid_value(de::Unexpected::Str(value), &self))?;
                }
                Ok(Digest(bytes))
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}

fn references<'de, D: Deserializer<'de>, T>(
    deserializer: D,
    make: fn([u8; 32]) -> T,
) -> Result<Vec<T>, D::Error> {
    struct Visitor<T>(fn([u8; 32]) -> T);
    impl<'de, T> de::Visitor<'de> for Visitor<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "an array of at most {MAX_REFERENCES} digests")
        }
        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<T>, A::Error> {
            let mut out = Vec::new();
            while let Some(Digest(bytes)) = seq.next_element()? {
                if out.len() == MAX_REFERENCES {
                    return Err(de::Error::custom("too many references"));
                }
                out.push((self.0)(bytes));
            }
            Ok(out)
        }
    }
    deserializer.deserialize_seq(Visitor(make))
}

fn cites<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<PremiseRef>, D::Error> {
    references(deserializer, PremiseRef::from_event_hash)
}

fn speculative_on<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<ClaimId>, D::Error> {
    references(deserializer, ClaimId::from_bytes)
}

fn bounded(length: usize, max: usize, field: &'static str) -> Result<(), ProtocolError> {
    if length > max {
        Err(ProtocolError::Limit { field, max })
    } else {
        Ok(())
    }
}

/// Lê um documento completo `proposal/v1`, sem resolver/autenticar referências.
///
/// # Errors
/// JSON não estrito, versão, hex, AIR ou limites inválidos.
pub fn decode(bytes: &[u8]) -> Result<Proposal, ProtocolError> {
    bounded(bytes.len(), MAX_MESSAGE_BYTES, "message")?;
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let envelope: Envelope<'_> = object(&mut deserializer).map_err(ProtocolError::Json)?;
    deserializer.end().map_err(ProtocolError::Json)?;
    if envelope.version != PROPOSAL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(envelope.version));
    }
    let payload = envelope.proposal;
    bounded(payload.text_pt.len(), MAX_TEXT_BYTES, "text_pt")?;
    let hex = payload.goal.as_bytes();
    bounded(hex.len().div_ceil(2), MAX_GOAL_BYTES, "goal")?;
    if hex.is_empty() || !hex.len().is_multiple_of(2) {
        return Err(ProtocolError::GoalHex);
    }
    let mut goal_bytes = Vec::with_capacity(hex.len() / 2);
    for pair in hex.as_chunks::<2>().0 {
        goal_bytes.push(byte(pair).ok_or(ProtocolError::GoalHex)?);
    }
    let goal =
        Formula::decode_with_limits(&goal_bytes, Limits::default()).map_err(ProtocolError::Air)?;
    Ok(Proposal {
        text_pt: UntrustedText::new(payload.text_pt.into_owned()),
        goal,
        cites: payload.cites,
        speculative_on: payload.speculative_on,
        vocab: Hash::from_bytes(payload.vocab.0),
    })
}

struct Hex<'a>(&'a [u8]);

impl fmt::Display for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Serialize for Hex<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

struct Cites<'a>(&'a [PremiseRef]);
impl Serialize for Cites<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|reference| Hex(reference.event_hash())))
    }
}

struct Speculative<'a>(&'a [ClaimId]);
impl Serialize for Speculative<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(|claim| Hex(claim.as_bytes())))
    }
}

#[derive(Serialize)]
struct Output<'a> {
    version: u32,
    proposal: OutputPayload<'a>,
}

#[derive(Serialize)]
struct OutputPayload<'a> {
    text_pt: &'a str,
    goal: Hex<'a>,
    cites: Cites<'a>,
    speculative_on: Speculative<'a>,
    vocab: Hex<'a>,
}

/// Escreve JSON `proposal/v1` sem promover texto ou referências a autoridade.
///
/// # Errors
/// Limites excedidos ou objetivo sem codificação canônica válida.
pub fn encode(proposal: &Proposal) -> Result<Vec<u8>, ProtocolError> {
    let text_pt = proposal.text_pt.as_untrusted_str();
    bounded(text_pt.len(), MAX_TEXT_BYTES, "text_pt")?;
    bounded(proposal.cites.len(), MAX_REFERENCES, "cites")?;
    bounded(
        proposal.speculative_on.len(),
        MAX_REFERENCES,
        "speculative_on",
    )?;
    // Construção em memória pode ter limites mais permissivos que proposal/v1.
    // Reusar stats já validados da Formula, sem decodificar/clonar de novo.
    let limits = Limits::default();
    bounded(proposal.goal.depth(), limits.max_depth, "goal_depth")?;
    bounded(proposal.goal.node_count(), limits.max_nodes, "goal_nodes")?;
    let goal = to_canon(&proposal.goal).map_err(ProtocolError::Canon)?;
    bounded(goal.len(), MAX_GOAL_BYTES, "goal")?;
    let out = serde_json::to_vec(&Output {
        version: PROPOSAL_VERSION,
        proposal: OutputPayload {
            text_pt,
            goal: Hex(&goal),
            cites: Cites(&proposal.cites),
            speculative_on: Speculative(&proposal.speculative_on),
            vocab: Hex(proposal.vocab.as_bytes()),
        },
    })
    .map_err(ProtocolError::Json)?;
    bounded(out.len(), MAX_MESSAGE_BYTES, "message")?;
    Ok(out)
}
