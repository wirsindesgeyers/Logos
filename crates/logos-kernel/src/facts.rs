//! Registro de fatos assinados (6.1, ADR0061) e confiança por tópico (6.2, ADR0063).
//!
//! Um fato fact/v1 no CAS só vira `FactRegistered` se a configuração de confiança
//! vigente no ledger (`TrustConfigPublished`, assinada pelo admin) conceder
//! `Trusted(emissor, tópico)`, a assinatura verificar, o vocabulário existir e a
//! fórmula for bem tipada nele, a janela ainda não tiver passado e o fato não estiver
//! registrado. A citação confere a mesma concessão de novo (índice/snapshot).

use logos_air::{Limits, Vocabulary};
use logos_crypto::{Hash, KeyId, PublicKey, Signer, hash};
use logos_ledger::{Authority, Event, EventBody, Fact, Ledger, TrustConfig, Validity};

use crate::TransportError as Error;

/// Publica uma configuração de confiança (já no CAS), assinada pelo admin.
///
/// `admin` é a chave administrativa configurada pelo operador; `signer` precisa ser
/// ela. A versão precisa ser maior que a vigente. Devolve o evento publicado.
///
/// # Errors
/// Signer que não é o admin; configuração ausente/malformada; versão não crescente;
/// assinatura ou I/O do ledger.
pub fn publish_trust<S: Signer, A: Signer>(
    ledger: &mut Ledger<'_, S>,
    admin: &PublicKey,
    signer: &A,
    config: Hash<TrustConfig>,
) -> Result<Hash<Event>, Error> {
    if signer.public_key() != admin {
        return Err(Error::Unauthorized);
    }
    let bytes = ledger
        .cas()
        .get(&config)?
        .ok_or(Error::Invalid("trust config missing from CAS"))?;
    let decoded: TrustConfig =
        logos_canon::from_canon(&bytes).map_err(|_| Error::Invalid("malformed trust config"))?;
    if ledger
        .live_index()?
        .trust_version()?
        .is_some_and(|current| decoded.version <= current)
    {
        return Err(Error::Invalid("trust config version not increasing"));
    }
    let key = signer
        .key_id()
        .map_err(|_| Error::Invalid("authority key"))?;
    let mut body = EventBody::TrustConfigPublished {
        config,
        authority: Authority {
            key,
            sig: signer
                .sign::<Event>(b"")
                .map_err(|_| Error::Invalid("authority signature"))?,
        },
    };
    let message = body
        .authority_message()?
        .ok_or(Error::Invalid("trust config authority"))?;
    let EventBody::TrustConfigPublished { authority, .. } = &mut body else {
        return Err(Error::Invalid("trust config body"));
    };
    authority.sig = signer
        .sign::<Event>(&message)
        .map_err(|_| Error::Invalid("authority signature"))?;
    Ok(hash(&ledger.append(body)?)?)
}

fn unix_now() -> Result<u64, Error> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| Error::Invalid("system clock before the Unix epoch"))
}

/// Confere e registra o fato `fact` (já no CAS). Devolve o evento FactRegistered.
///
/// # Errors
/// Fato ausente/malformado, emissor sem `Trusted(emissor, tópico)` vigente,
/// assinatura inválida, vocabulário ausente ou fórmula mal tipada, janela já
/// vencida, fato repetido; I/O do ledger.
pub fn register<S: Signer>(
    ledger: &mut Ledger<'_, S>,
    fact: Hash<Fact>,
) -> Result<Hash<Event>, Error> {
    let bytes = ledger
        .cas()
        .get(&fact)?
        .ok_or(Error::Invalid("fact missing from CAS"))?;
    let decoded: Fact =
        logos_canon::from_canon(&bytes).map_err(|_| Error::Invalid("malformed fact"))?;
    let issuer = decoded
        .issuer
        .key_id()
        .map_err(|_| Error::Invalid("fact issuer key"))?;
    // 6.4: emissor revogado não registra mais fatos.
    if ledger.live_index()?.issuer_revoked(&issuer)? {
        return Err(Error::Unauthorized);
    }
    // 6.2: `db-monitor` não atesta `backup`; sem configuração publicada, ninguém atesta.
    if ledger
        .live_index()?
        .trust_grant(&issuer, decoded.topic.as_str())?
        .is_none()
    {
        return Err(Error::Unauthorized);
    }
    decoded
        .verify_signature()
        .map_err(|_| Error::Invalid("fact signature"))?;
    let vocabulary_bytes = ledger
        .cas()
        .get(&decoded.vocabulary)?
        .ok_or(Error::Invalid("fact vocabulary missing"))?;
    let vocabulary = Vocabulary::decode_with_limits(&vocabulary_bytes, Limits::default())
        .map_err(|_| Error::Invalid("fact vocabulary"))?;
    logos_air::check(&vocabulary, &decoded.formula)
        .map_err(|_| Error::Invalid("fact formula sort"))?;
    let epoch = ledger
        .head()?
        .ok_or(Error::Invalid("fact registration requires Genesis"))?
        .epoch;
    let expired = match decoded.valid {
        Validity::Epochs(range) => range.last() < epoch,
        Validity::Until(instant) => unix_now()? > instant,
    };
    if expired {
        return Err(Error::Invalid("fact validity already over"));
    }
    if ledger.live_index()?.fact_event(fact)?.is_some() {
        return Err(Error::Invalid("fact already registered"));
    }
    Ok(hash(&ledger.append(EventBody::FactRegistered { fact })?)?)
}

/// Resultado de uma revogação de emissor (6.4, ADR0064).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuerRevocation {
    /// Evento IssuerRevoked gravado.
    pub revoked: Hash<Event>,
    /// Invalidated gravados pela cascata (claims que dependiam dos fatos mortos).
    pub invalidated: Vec<Hash<Event>>,
}

/// Revoga o emissor `issuer` (assinado pelo admin): todo fato dele registrado na
/// época `since` ou depois morre, a cascata ATMS derruba quem dependia só deles e
/// fatos novos dele passam a ser recusados. `since` é a época de **registro**
/// (gravada pelo kernel), não o `observed_at` que o emissor comprometido assina.
///
/// # Errors
/// Signer que não é o admin; emissor já revogado; replay, índice ou append.
pub fn revoke_issuer<S: Signer, A: Signer>(
    ledger: &mut Ledger<'_, S>,
    admin: &PublicKey,
    signer: &A,
    issuer: KeyId,
    since: u64,
) -> Result<IssuerRevocation, Error> {
    if signer.public_key() != admin {
        return Err(Error::Unauthorized);
    }
    if ledger.live_index()?.issuer_revoked(&issuer)? {
        return Err(Error::Invalid("issuer already revoked"));
    }
    let key = signer
        .key_id()
        .map_err(|_| Error::Invalid("authority key"))?;
    let mut body = EventBody::IssuerRevoked {
        issuer,
        since,
        authority: Authority {
            key,
            sig: signer
                .sign::<Event>(b"")
                .map_err(|_| Error::Invalid("authority signature"))?,
        },
    };
    let message = body
        .authority_message()?
        .ok_or(Error::Invalid("issuer revocation authority"))?;
    let EventBody::IssuerRevoked { authority, .. } = &mut body else {
        return Err(Error::Invalid("issuer revocation body"));
    };
    authority.sig = signer
        .sign::<Event>(&message)
        .map_err(|_| Error::Invalid("authority signature"))?;
    let revoked = hash(&ledger.append(body)?)?;
    let invalidated = crate::retract::Retractions::new(admin.clone()).reconcile(ledger)?;
    Ok(IssuerRevocation {
        revoked,
        invalidated,
    })
}
