//! Registro administrativo não carrega código: somente implementações locais.

use std::{fs::File, io::Read, path::Path};

use logos_canon::Text;
use logos_crypto::{Hash, PublicKey, Signer, hash_raw};
use logos_ledger::{Authority, Checker, Event, EventBody, Ledger};
use logos_lrat::CakeRegistry;

use crate::{TransportError as Error, replay::replay};

/// Formato do checker Rust com extração do fecho LRAT.
pub const LRAT: &str = "lrat-rup/v1";
/// Mesmo perfil com conferência adicional do cake_lpr fixado.
pub const CAKE: &str = "cake-lpr/v1";
/// Formato de contramodelos finitos.
pub const MODEL: &str = "model/v1";

/// Implementações conhecidas pelo operador; nenhum caminho vem de CERTIFY.
pub struct CheckerRegistry {
    authority: PublicKey,
    builtin: Vec<u8>,
    identity: Hash<Checker>,
    cake: Option<(CakeRegistry, Vec<u8>)>,
}

impl CheckerRegistry {
    /// Identifica o executável em execução e opcionalmente o cake fixado.
    /// # Errors
    /// I/O, tamanho ou binário cake desconhecido; sem fallback de configuração.
    pub fn new(authority: PublicKey, cake: Option<&Path>) -> Result<Self, Error> {
        let builtin = bytes(Path::new("/proc/self/exe"), 128 * 1024 * 1024)?;
        let identity = hash_raw(&builtin);
        let cake = cake
            .map(|path| {
                let registry = CakeRegistry::register(path)
                    .map_err(|_| Error::Invalid("unknown cake checker"))?;
                let snapshot = bytes(path, 8 * 1024 * 1024)?;
                if hash_raw::<Checker>(&snapshot).as_bytes() != registry.identity().as_bytes() {
                    return Err(Error::Invalid("cake changed during registration"));
                }
                Ok((registry, snapshot))
            })
            .transpose()?;
        Ok(Self {
            authority,
            builtin,
            identity,
            cake,
        })
    }

    /// Identidade dos checkers compilados neste executável, por formato.
    #[must_use]
    pub fn builtin_identity(&self) -> Hash<Checker> {
        self.identity
    }

    /// Identidade cake conhecida, se provisionada pelo operador.
    #[must_use]
    pub fn cake_identity(&self) -> Option<Hash<Checker>> {
        self.cake
            .as_ref()
            .map(|(c, _)| Hash::from_bytes(*c.identity().as_bytes()))
    }

    /// Provisionamento local explícito antes de servir; nunca reativa revogados.
    /// # Errors
    /// Signer não é a autoridade configurada, replay, CAS, assinatura ou append.
    pub fn install<S: Signer>(&self, ledger: &mut Ledger<'_, S>, signer: &S) -> Result<(), Error> {
        if signer.public_key() != &self.authority {
            return Err(Error::Unauthorized);
        }
        self.register(ledger, signer, LRAT, self.identity, &self.builtin)?;
        self.register(ledger, signer, MODEL, self.identity, &self.builtin)?;
        if let Some((cake, snapshot)) = &self.cake {
            self.register(
                ledger,
                signer,
                CAKE,
                Hash::from_bytes(*cake.identity().as_bytes()),
                snapshot,
            )?;
        }
        Ok(())
    }

    fn register<S: Signer>(
        &self,
        ledger: &mut Ledger<'_, S>,
        signer: &S,
        format: &str,
        identity: Hash<Checker>,
        snapshot: &[u8],
    ) -> Result<(), Error> {
        let (registered, revoked) = self.state(ledger, format, identity)?;
        if registered || revoked {
            return Ok(());
        }
        if ledger.cas().put::<Checker>(snapshot)? != identity {
            return Err(Error::Invalid("checker snapshot identity"));
        }
        let mut body = EventBody::CheckerRegistered {
            checker: identity,
            format: Text::new(format)?,
            authority: Authority {
                key: signer
                    .key_id()
                    .map_err(|_| Error::Invalid("authority key"))?,
                sig: signer
                    .sign::<Event>(b"")
                    .map_err(|_| Error::Invalid("authority signature"))?,
            },
        };
        let message = body
            .authority_message()?
            .ok_or(Error::Invalid("registration authority"))?;
        let EventBody::CheckerRegistered { authority, .. } = &mut body else {
            return Err(Error::Invalid("registration body"));
        };
        authority.sig = signer
            .sign::<Event>(&message)
            .map_err(|_| Error::Invalid("authority signature"))?;
        ledger.append(body)?;
        Ok(())
    }

    fn state<S: Signer>(
        &self,
        ledger: &Ledger<'_, S>,
        format: &str,
        identity: Hash<Checker>,
    ) -> Result<(bool, bool), Error> {
        let mut registered = false;
        let mut revoked = false;
        replay(ledger, |event| {
            match &event.body {
                EventBody::Genesis { kernel_key, .. } => {
                    if *kernel_key
                        != self
                            .authority
                            .key_id()
                            .map_err(|_| Error::Invalid("registry root"))?
                    {
                        return Err(Error::Unauthorized);
                    }
                }
                EventBody::CheckerRegistered {
                    checker, format: f, ..
                } if *checker == identity && f.as_str() == format => {
                    event.body.verify_authority(&self.authority)?;
                    registered = true;
                }
                EventBody::CheckerRevoked { checker, .. } if *checker == identity => {
                    event.body.verify_authority(&self.authority)?;
                    revoked = true;
                }
                _ => {}
            }
            Ok(())
        })?;
        Ok((registered, revoked))
    }

    pub(crate) fn active<S: Signer>(
        &self,
        ledger: &Ledger<'_, S>,
        format: &str,
    ) -> Result<Hash<Checker>, Error> {
        let identity = match format {
            LRAT | MODEL => self.identity,
            CAKE => self
                .cake_identity()
                .ok_or(Error::Invalid("cake not locally configured"))?,
            _ => return Err(Error::Invalid("unknown checker format")),
        };
        if self.state(ledger, format, identity)? != (true, false) {
            return Err(Error::Invalid("checker not registered or revoked"));
        }
        Ok(identity)
    }

    pub(crate) fn cake(&self) -> Result<&CakeRegistry, Error> {
        self.cake
            .as_ref()
            .map(|(c, _)| c)
            .ok_or(Error::Invalid("cake not configured"))
    }
}

fn bytes(path: &Path, limit: u64) -> Result<Vec<u8>, Error> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit {
        return Err(Error::Limit("checker binary size/type"));
    }
    let mut data = Vec::new();
    file.take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 > limit {
        return Err(Error::Limit("checker binary bytes"));
    }
    Ok(data)
}
