//! Registro de fatos assinados (Etapa 6.1, ADR0061).
//!
//! Um fato fact/v1 no CAS só vira `FactRegistered` se o emissor estiver na raiz de
//! confiança do operador, a assinatura verificar, o vocabulário existir e a fórmula
//! for bem tipada nele, a janela ainda não tiver passado e o fato não estiver
//! registrado. Confiança por tópico é da 6.2: aqui a raiz vale para qualquer tópico.

use logos_air::{Limits, Vocabulary};
use logos_crypto::{Hash, KeyId, PublicKey, Signer, hash};
use logos_ledger::{Event, EventBody, Fact, Ledger, Validity};

use crate::TransportError as Error;

/// Emissores reconhecidos, configurados pelo operador (não vêm do protocolo).
#[derive(Debug, Clone, Default)]
pub struct TrustRoots {
    issuers: Vec<KeyId>,
}

impl TrustRoots {
    /// Raiz a partir das chaves públicas dos emissores.
    ///
    /// # Errors
    /// Chave sem identidade derivável.
    pub fn new(issuers: &[PublicKey]) -> Result<Self, Error> {
        let issuers = issuers
            .iter()
            .map(PublicKey::key_id)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Error::Invalid("trust root key"))?;
        Ok(Self { issuers })
    }

    fn trusts(&self, issuer: &PublicKey) -> Result<bool, Error> {
        let id = issuer
            .key_id()
            .map_err(|_| Error::Invalid("fact issuer key"))?;
        Ok(self.issuers.contains(&id))
    }
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
/// Fato ausente/malformado, emissor fora da raiz, assinatura inválida, vocabulário
/// ausente ou fórmula mal tipada, janela já vencida, fato repetido; I/O do ledger.
pub fn register<S: Signer>(
    ledger: &mut Ledger<'_, S>,
    roots: &TrustRoots,
    fact: Hash<Fact>,
) -> Result<Hash<Event>, Error> {
    let bytes = ledger
        .cas()
        .get(&fact)?
        .ok_or(Error::Invalid("fact missing from CAS"))?;
    let decoded: Fact =
        logos_canon::from_canon(&bytes).map_err(|_| Error::Invalid("malformed fact"))?;
    if !roots.trusts(&decoded.issuer)? {
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
