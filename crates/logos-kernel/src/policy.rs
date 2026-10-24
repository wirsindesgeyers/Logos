//! Publicação de política (7.2, ADR0070): objeto `policy/v1` no CAS, raiz Merkle `r_v`
//! recomputada pelo kernel, `PolicyPublished` assinado pelo admin e `EpochAdvanced` associado.

use logos_air::{Limits, Vocabulary};
use logos_crypto::{Hash, PublicKey, Signer, hash};
use logos_ledger::{Authority, Event, EventBody, Ledger, PolicyObject};
use logos_policy::{Policy, PolicyTree};

use crate::TransportError as Error;

/// Resultado de uma publicação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Published {
    /// Evento `PolicyPublished`.
    pub published: Hash<Event>,
    /// Evento `EpochAdvanced` que carrega `policy_root`.
    pub epoch: Hash<Event>,
    /// Versão publicada.
    pub version: u64,
    /// Raiz Merkle `r_v`.
    pub root: [u8; 32],
}

/// Publica a política `policy` (já no CAS, com o vocabulário `hV` dela), assinada pelo admin.
///
/// Confere: signer é o admin; o objeto decodifica e revalida contra o vocabulário (sort-check,
/// forma de Horn, estratificação); a versão é maior que a vigente. Recusa não grava evento.
///
/// # Errors
/// Signer que não é o admin; objeto ou vocabulário ausente/inválido; política recusada;
/// versão não crescente; assinatura ou I/O do ledger.
pub fn publish<S: Signer, A: Signer>(
    ledger: &mut Ledger<'_, S>,
    admin: &PublicKey,
    signer: &A,
    policy: Hash<PolicyObject>,
    reason: logos_canon::Text,
) -> Result<Published, Error> {
    if signer.public_key() != admin {
        return Err(Error::Unauthorized);
    }
    let bytes = ledger
        .cas()
        .get(&policy)?
        .ok_or(Error::Invalid("policy missing from CAS"))?;
    let header: Policy =
        logos_canon::from_canon(&bytes).map_err(|_| Error::Invalid("malformed policy"))?;
    let vocabulary_bytes = ledger
        .cas()
        .get(&header.vocabulary())?
        .ok_or(Error::Invalid("policy vocabulary missing from CAS"))?;
    let vocabulary = Vocabulary::decode_with_limits(&vocabulary_bytes, Limits::default())
        .map_err(|_| Error::Invalid("policy vocabulary"))?;
    let decoded =
        Policy::decode(&bytes, &vocabulary).map_err(|_| Error::Invalid("policy rejected"))?;
    if ledger
        .live_index()?
        .current_policy()?
        .is_some_and(|current| decoded.version() <= current.version)
    {
        return Err(Error::Invalid("policy version not increasing"));
    }
    let root = *PolicyTree::new(&decoded)
        .map_err(|_| Error::Invalid("policy tree"))?
        .root();
    let key = signer
        .key_id()
        .map_err(|_| Error::Invalid("authority key"))?;
    let mut body = EventBody::PolicyPublished {
        policy,
        version: decoded.version(),
        root,
        authority: Authority {
            key,
            sig: signer
                .sign::<Event>(b"")
                .map_err(|_| Error::Invalid("authority signature"))?,
        },
    };
    let message = body
        .authority_message()?
        .ok_or(Error::Invalid("policy authority"))?;
    let EventBody::PolicyPublished { authority, .. } = &mut body else {
        return Err(Error::Invalid("policy body"));
    };
    authority.sig = signer
        .sign::<Event>(&message)
        .map_err(|_| Error::Invalid("authority signature"))?;
    let published = hash(&ledger.append(body)?)?;
    let epoch = ledger
        .head()?
        .ok_or(Error::Invalid("policy publication requires Genesis"))?
        .epoch
        .checked_add(1)
        .ok_or(Error::Limit("epoch"))?;
    let epoch = hash(&ledger.append(EventBody::EpochAdvanced {
        new_epoch: epoch,
        reason,
        policy_root: Some(root),
        vocab: Some(decoded.vocabulary()),
    })?)?;
    Ok(Published {
        published,
        epoch,
        version: decoded.version(),
        root,
    })
}
