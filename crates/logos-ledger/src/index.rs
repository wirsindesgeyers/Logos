//! Snapshot de índices derivados exclusivamente de uma cadeia conferida.

use std::collections::BTreeMap;
use std::fs::File;
use std::marker::PhantomData;
use std::path::Path;

use logos_air::Formula;
use logos_crypto::{Hash, PublicKey};
use logos_obligation::{EntryKind, LedgerEntry, LedgerView, PremiseRef};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

use crate::index_resolve::{ResolvedEntry, Resolver};
use crate::{
    Cas, Event, LedgerError, LedgerHead, RebuildReport, index_project, index_store, ledger,
    ledger_store,
};

/// Assumptions abertas de um claim (5.7): `(h(AssumptionDeclared), scope)`.
pub type OpenAssumptions = Vec<(Hash<Event>, String)>;

/// Estado efetivo da referência de evento, sem inferir validade da prova.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClaimStatus {
    /// Categoria projetada; invalidação inclui dependentes usados.
    pub kind: EntryKind,
    /// Época imutável da emissão (sem promoção silenciosa).
    pub epoch: u64,
    /// O próprio evento foi alvo de Retracted.
    pub retracted: bool,
}

/// Índices privados e imutáveis de um histórico autenticado.
///
/// Mantém o lock de escritor até Drop. index.sqlite3 é somente cópia derivada;
/// consultas usam SQLite em memória reconstruído, nunca o cache anterior.
pub struct LedgerIndex<'a> {
    pub(crate) connection: Connection,
    pub(crate) head: Option<LedgerHead>,
    report: RebuildReport,
    _owned_lock: Option<File>,
    _borrowed_lock: Option<&'a File>,
}

impl LedgerIndex<'static> {
    /// Abre um ledger existente com chave pública, confere e reconstrói índices.
    ///
    /// # Errors
    /// Lock ocupado, cadeia/schema/chave inválidos ou falha de armazenamento.
    pub fn open(directory: &Path, public_key: &PublicKey) -> Result<Self, LedgerError> {
        let path = std::fs::canonicalize(directory)?;
        let lock = ledger::writer_lock(&path)?;
        Self::build(&path, public_key, Some(lock), None, None)
    }
}

impl<'a> LedgerIndex<'a> {
    pub(crate) fn from_writer(
        path: &Path,
        public_key: &PublicKey,
        lock: &'a File,
    ) -> Result<Self, LedgerError> {
        Self::build(path, public_key, None, Some(lock), None)
    }

    pub(crate) fn build(
        path: &Path,
        public_key: &PublicKey,
        owned: Option<File>,
        borrowed: Option<&'a File>,
        expected: Option<Option<LedgerHead>>,
    ) -> Result<Self, LedgerError> {
        let mut source = Connection::open_with_flags(
            path.join("ledger.sqlite3"),
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        source.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA query_only=ON")?;
        source.set_limit(
            rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH,
            (crate::MAX_EVENT_BYTES + 1024) as i32,
        )?;
        source.busy_timeout(std::time::Duration::ZERO)?;
        let source_transaction = source.transaction()?;
        ledger_store::schema(&source_transaction)?;
        let mut connection = Connection::open_in_memory()?;
        index_store::create(&connection)?;
        let transaction = connection.transaction()?;
        let cas = crate::Cas::open(path.join("cas"), crate::Limits::default())?;
        let head = ledger::scan(&source_transaction, public_key, |event, head| {
            index_project::apply(&transaction, &cas, event, head)
        })?;
        if expected.is_some_and(|expected| expected != head) {
            return Err(LedgerError::Invalid("live index head"));
        }
        write_head(&transaction, head)?;
        transaction.commit()?;
        source_transaction.commit()?;
        let report = index_store::publish(path, &connection, head)?;
        connection.execute_batch("PRAGMA query_only=ON")?;
        Ok(Self {
            connection,
            head,
            report,
            _owned_lock: owned,
            _borrowed_lock: borrowed,
        })
    }

    /// Cabeça que identifica exatamente o histórico deste snapshot.
    #[must_use]
    pub fn head(&self) -> Option<LedgerHead> {
        self.head
    }

    /// Resultado da reconstrução/comparação com o cache anterior.
    #[must_use]
    pub fn report(&self) -> RebuildReport {
        self.report
    }

    /// Categoria/época/retratação de uma referência; None somente sem claim.
    ///
    /// # Errors
    /// Falha SQL ou estado privado incoerente.
    pub fn status(&self, reference: Hash<Event>) -> Result<Option<ClaimStatus>, LedgerError> {
        let fields = self
            .connection
            .prepare("SELECT status,epoch,retracted FROM claims WHERE id=?1")?
            .query_row([reference.as_bytes().as_slice()], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, bool>(2)?,
                ))
            })
            .optional()?;
        let Some((status, epoch, retracted)) = fields else {
            return Ok(None);
        };
        let kind = match status {
            index_project::PROPOSAL => EntryKind::Proposal,
            index_project::PENDING => EntryKind::Pending,
            index_project::VERIFIED => EntryKind::Verified,
            index_project::REFUTED => EntryKind::Refuted,
            index_project::UNKNOWN => EntryKind::Unknown,
            index_project::ASSUMPTION => EntryKind::Assumption,
            index_project::INVALIDATED => EntryKind::Invalidated,
            index_project::SUSPENDED => EntryKind::Suspended,
            index_project::FACT => EntryKind::Fact,
            _ => return Err(LedgerError::Invalid("index status")),
        };
        let epoch = u64::from_be_bytes(
            epoch
                .as_slice()
                .try_into()
                .map_err(|_| LedgerError::Invalid("index epoch"))?,
        );
        Ok(Some(ClaimStatus {
            kind,
            epoch,
            retracted,
        }))
    }

    /// Primeiro evento Refuted da obrigação, se houver (S4, Etapa 5.6).
    ///
    /// Refutação é testemunha permanente: nenhuma outra linha a desfaz.
    ///
    /// # Errors
    /// Falha SQL ou digest privado inválido.
    pub fn refuted(
        &self,
        obligation: Hash<logos_obligation::Obligation>,
    ) -> Result<Option<Hash<Event>>, LedgerError> {
        let row = self
            .connection
            .prepare("SELECT id FROM claims WHERE h_o=?1 AND status=?2 ORDER BY id LIMIT 1")?
            .query_row(
                params![obligation.as_bytes().as_slice(), index_project::REFUTED],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .optional()?;
        row.map(|bytes| {
            bytes
                .as_slice()
                .try_into()
                .map(Hash::from_bytes)
                .map_err(|_| LedgerError::Invalid("refuted digest"))
        })
        .transpose()
    }

    /// Menor rota Verified ativa (não suspensa) do nó h(O), citável na época da
    /// cabeça; base de especulação resolvida no CERTIFY (Etapa 5.8, ADR0058).
    ///
    /// # Errors
    /// Falha SQL ou digest privado inválido.
    pub fn active_route(
        &self,
        obligation: Hash<logos_obligation::Obligation>,
    ) -> Result<Option<Hash<Event>>, LedgerError> {
        let Some(head) = self.head else {
            return Ok(None);
        };
        let row = self
            .connection
            .prepare("SELECT claims.id FROM claims JOIN citable ON citable.ref=claims.id WHERE h_o=?1 AND checker IS NOT NULL AND status=?2 AND citable.epoch=?3 ORDER BY claims.id LIMIT 1")?
            .query_row(
                params![
                    obligation.as_bytes().as_slice(),
                    index_project::VERIFIED,
                    head.epoch.to_be_bytes().as_slice()
                ],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .optional()?;
        row.map(|bytes| {
            bytes
                .as_slice()
                .try_into()
                .map(Hash::from_bytes)
                .map_err(|_| LedgerError::Invalid("route digest"))
        })
        .transpose()
    }

    /// Rotas Verified mortas do nó h(O), em ordem de h(Event), se o nó teve
    /// rotas e nenhuma está viva (Verified ou Suspended); vazio caso contrário.
    ///
    /// # Errors
    /// Falha SQL ou digest privado inválido.
    pub fn dead_routes(
        &self,
        obligation: Hash<logos_obligation::Obligation>,
    ) -> Result<Vec<Hash<Event>>, LedgerError> {
        let mut statement = self.connection.prepare(
            "SELECT id, status FROM claims WHERE h_o=?1 AND checker IS NOT NULL AND status IN (?2,?3,?4) ORDER BY id",
        )?;
        let rows = statement
            .query_map(
                params![
                    obligation.as_bytes().as_slice(),
                    index_project::VERIFIED,
                    index_project::SUSPENDED,
                    index_project::INVALIDATED
                ],
                |r| Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, i64>(1)?)),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        if rows
            .iter()
            .any(|(_, status)| *status != index_project::INVALIDATED)
        {
            return Ok(Vec::new());
        }
        rows.into_iter()
            .map(|(bytes, _)| {
                bytes
                    .as_slice()
                    .try_into()
                    .map(Hash::from_bytes)
                    .map_err(|_| LedgerError::Invalid("route digest"))
            })
            .collect()
    }

    /// Assumptions no fecho de dependências used da referência, inclusive ela
    /// mesma, com seus scopes; ordenadas por h(Event). `None` se a referência
    /// não é linha do índice; lista vazia = claim incondicional (Etapa 5.7).
    ///
    /// # Errors
    /// Falha SQL ou digest/scope privado inválido.
    pub fn open_assumptions(
        &self,
        reference: Hash<Event>,
    ) -> Result<Option<OpenAssumptions>, LedgerError> {
        if self.status(reference)?.is_none() {
            return Ok(None);
        }
        let mut statement = self.connection.prepare(
            "WITH RECURSIVE ancestors(id) AS (VALUES(?1) UNION SELECT deps.premise FROM deps JOIN ancestors ON deps.claim=ancestors.id) \
             SELECT claims.id, claims.scope FROM claims JOIN ancestors ON claims.id=ancestors.id \
             WHERE claims.scope IS NOT NULL ORDER BY claims.id",
        )?;
        let mut rows = statement.query([reference.as_bytes().as_slice()])?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            let bytes: Vec<u8> = row.get(0)?;
            let digest = bytes
                .as_slice()
                .try_into()
                .map_err(|_| LedgerError::Invalid("assumption digest"))?;
            result.push((Hash::from_bytes(digest), row.get::<_, String>(1)?));
        }
        Ok(Some(result))
    }

    /// Citabilidade de estado na época corrente; scope é conferido no snapshot.
    ///
    /// # Errors
    /// Falha SQL, nunca convertida em false.
    pub fn is_citable(&self, reference: Hash<Event>, epoch: u64) -> Result<bool, LedgerError> {
        if self.head.is_none_or(|head| head.epoch != epoch) {
            return Ok(false);
        }
        let listed = self
            .connection
            .prepare("SELECT 1 FROM citable WHERE ref=?1 AND epoch=?2")?
            .query_row(
                params![
                    reference.as_bytes().as_slice(),
                    epoch.to_be_bytes().as_slice()
                ],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if listed {
            return Ok(true);
        }
        // 6.1: fato vivo é citável enquanto a janela incluir a época e o instante;
        // 6.2: e enquanto o emissor for confiável para o tópico.
        Ok(match self.fact_window(reference)? {
            Some((registered, validity, trusted)) => {
                trusted && validity.applicable(epoch, unix_now()?, registered)
            }
            None => false,
        })
    }

    /// Época de registro, janela e confiança vigente de um fato vivo (status Fact).
    fn fact_window(
        &self,
        reference: Hash<Event>,
    ) -> Result<Option<(u64, crate::Validity, bool)>, LedgerError> {
        let row = self
            .connection
            .prepare("SELECT claims.epoch, facts.first, facts.last, facts.until, trust.config IS NOT NULL FROM facts JOIN claims ON claims.id=facts.id LEFT JOIN trust ON trust.issuer=facts.issuer AND trust.topic=facts.topic WHERE facts.id=?1 AND claims.status=?2")?
            .query_row(
                params![reference.as_bytes().as_slice(), index_project::FACT],
                |r| {
                    Ok((
                        r.get::<_, Vec<u8>>(0)?,
                        r.get::<_, Option<Vec<u8>>>(1)?,
                        r.get::<_, Option<Vec<u8>>>(2)?,
                        r.get::<_, Option<Vec<u8>>>(3)?,
                        r.get::<_, bool>(4)?,
                    ))
                },
            )
            .optional()?;
        let Some((registered, first, last, until, trusted)) = row else {
            return Ok(None);
        };
        let number = |bytes: &[u8]| -> Result<u64, LedgerError> {
            Ok(u64::from_be_bytes(
                bytes
                    .try_into()
                    .map_err(|_| LedgerError::Invalid("fact window bytes"))?,
            ))
        };
        let validity = match (first, last, until) {
            (Some(first), Some(last), None) => crate::Validity::Epochs(
                logos_core::EpochRange::new(number(&first)?, number(&last)?)
                    .map_err(|_| LedgerError::Invalid("fact window range"))?,
            ),
            (None, None, Some(until)) => crate::Validity::Until(number(&until)?),
            _ => return Err(LedgerError::Invalid("fact window shape")),
        };
        Ok(Some((number(&registered)?, validity, trusted)))
    }

    /// Época de aplicabilidade do fato no snapshot: a corrente se a janela a inclui;
    /// senão o limite mais próximo (o gerador recusa com InvalidEpoch). `Until`
    /// vencido marca `expired` (o gerador recusa com Expired).
    fn resolve_fact_window(
        &self,
        reference: Hash<Event>,
        entry: &mut ResolvedEntry,
    ) -> Result<(), LedgerError> {
        let current = self.head.map_or(0, |h| h.epoch);
        let Some((registered, validity, trusted)) = self.fact_window(reference)? else {
            return Err(LedgerError::Invalid("fact without window"));
        };
        entry.untrusted = !trusted;
        match validity {
            crate::Validity::Epochs(range) => {
                entry.status.epoch = if range.contains(current) {
                    current
                } else if current < range.first() {
                    range.first()
                } else {
                    range.last()
                };
            }
            crate::Validity::Until(instant) => {
                entry.expired = unix_now()? > instant;
                if !entry.expired && current >= registered {
                    entry.status.epoch = current;
                }
            }
        }
        Ok(())
    }

    /// `Trusted(emissor, tópico)` na configuração vigente: o evento
    /// TrustConfigPublished que concede, se houver (6.2).
    ///
    /// # Errors
    /// Falha SQL ou digest privado inválido.
    pub fn trust_grant(
        &self,
        issuer: &logos_crypto::KeyId,
        topic: &str,
    ) -> Result<Option<Hash<Event>>, LedgerError> {
        let row = self
            .connection
            .prepare("SELECT config FROM trust WHERE issuer=?1 AND topic=?2")?
            .query_row(params![issuer.as_bytes().as_slice(), topic], |r| {
                r.get::<_, Vec<u8>>(0)
            })
            .optional()?;
        row.map(|bytes| {
            bytes
                .as_slice()
                .try_into()
                .map(Hash::from_bytes)
                .map_err(|_| LedgerError::Invalid("trust config digest"))
        })
        .transpose()
    }

    /// O emissor foi revogado (6.4): não registra mais fatos.
    ///
    /// # Errors
    /// Falha SQL.
    pub fn issuer_revoked(&self, issuer: &logos_crypto::KeyId) -> Result<bool, LedgerError> {
        Ok(self
            .connection
            .prepare("SELECT 1 FROM revoked_issuers WHERE issuer=?1")?
            .query_row([issuer.as_bytes().as_slice()], |_| Ok(()))
            .optional()?
            .is_some())
    }

    /// Versão da configuração de confiança vigente, se alguma foi publicada (6.2).
    ///
    /// # Errors
    /// Falha SQL ou bytes de versão inválidos.
    pub fn trust_version(&self) -> Result<Option<u64>, LedgerError> {
        let row = self
            .connection
            .prepare("SELECT max(version) FROM trust_configs")?
            .query_row([], |r| r.get::<_, Option<Vec<u8>>>(0))?;
        row.map(|bytes| {
            bytes
                .as_slice()
                .try_into()
                .map(u64::from_be_bytes)
                .map_err(|_| LedgerError::Invalid("trust config version bytes"))
        })
        .transpose()
    }

    /// Concessão vigente que torna citável o fato registrado em `reference` (6.2).
    ///
    /// # Errors
    /// Falha SQL ou digest privado inválido.
    pub fn fact_trust(&self, reference: Hash<Event>) -> Result<Option<Hash<Event>>, LedgerError> {
        let row = self
            .connection
            .prepare("SELECT trust.config FROM facts JOIN trust ON trust.issuer=facts.issuer AND trust.topic=facts.topic WHERE facts.id=?1")?
            .query_row([reference.as_bytes().as_slice()], |r| r.get::<_, Vec<u8>>(0))
            .optional()?;
        row.map(|bytes| {
            bytes
                .as_slice()
                .try_into()
                .map(Hash::from_bytes)
                .map_err(|_| LedgerError::Invalid("trust config digest"))
        })
        .transpose()
    }

    /// Evento FactRegistered de um fato, se registrado (6.1).
    ///
    /// # Errors
    /// Falha SQL ou digest privado inválido.
    pub fn fact_event(&self, fact: Hash<crate::Fact>) -> Result<Option<Hash<Event>>, LedgerError> {
        let row = self
            .connection
            .prepare("SELECT id FROM facts WHERE fact=?1")?
            .query_row([fact.as_bytes().as_slice()], |r| r.get::<_, Vec<u8>>(0))
            .optional()?;
        row.map(|bytes| {
            bytes
                .as_slice()
                .try_into()
                .map(Hash::from_bytes)
                .map_err(|_| LedgerError::Invalid("fact event digest"))
        })
        .transpose()
    }

    /// Dependentes diretos ordenados por h(Event), exclusivamente dos used.
    ///
    /// # Errors
    /// Falha SQL ou digest privado inválido.
    pub fn dependents(&self, reference: Hash<Event>) -> Result<Vec<Hash<Event>>, LedgerError> {
        let mut statement = self
            .connection
            .prepare("SELECT claim FROM deps WHERE premise=?1 ORDER BY claim")?;
        let mut rows = statement.query([reference.as_bytes().as_slice()])?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            let bytes: Vec<u8> = row.get(0)?;
            let digest = bytes
                .as_slice()
                .try_into()
                .map_err(|_| LedgerError::Invalid("dependency digest"))?;
            result.push(Hash::from_bytes(digest));
        }
        Ok(result)
    }

    /// Resolve a fórmula real/contextual da referência, sem conferir certificado.
    ///
    /// # Errors
    /// CAS ausente/corrompido, envelope/hash/tipagem/limites ou I/O inválidos.
    pub fn formula_of(
        &self,
        reference: Hash<Event>,
        cas: &Cas,
    ) -> Result<Option<Formula>, LedgerError> {
        let Some(status) = self.status(reference)? else {
            return Ok(None);
        };
        let entry =
            Resolver::default().resolve(&self.connection, cas, reference.as_bytes(), status)?;
        Ok(Some((*entry.formula).clone()))
    }

    /// Prepara referências exatas para o gerador, sob um scope opaco explícito.
    ///
    /// # Errors
    /// Resolução CAS/tipagem inválida ou scope de assumption incompatível,
    /// inclusive em dependências transitivas usadas. Unknown nunca é promovido.
    pub fn snapshot<'v>(
        &'v self,
        cas: &Cas,
        references: &[PremiseRef],
        scope: &str,
    ) -> Result<LedgerSnapshot<'v>, LedgerError> {
        let mut entries = BTreeMap::new();
        let mut resolver = Resolver::default();
        for reference in references {
            if entries.contains_key(reference) {
                continue;
            }
            let hash = Hash::from_bytes(*reference.event_hash());
            let entry = match self.status(hash)? {
                None => None,
                Some(status) => {
                    let mismatch = self.connection.prepare("WITH RECURSIVE ancestors(id) AS (VALUES(?1) UNION SELECT deps.premise FROM deps JOIN ancestors ON deps.claim=ancestors.id) SELECT 1 FROM claims JOIN ancestors ON claims.id=ancestors.id WHERE scope IS NOT NULL AND scope<>?2 LIMIT 1")?
                        .query_row(params![reference.event_hash().as_slice(), scope], |_| Ok(())).optional()?.is_some();
                    if mismatch {
                        return Err(LedgerError::Invalid("assumption scope mismatch"));
                    }
                    let mut resolved =
                        resolver.resolve(&self.connection, cas, reference.event_hash(), status)?;
                    if resolved.status.kind == EntryKind::Fact {
                        self.resolve_fact_window(hash, &mut resolved)?;
                    }
                    Some(resolved)
                }
            };
            entries.insert(*reference, entry);
        }
        Ok(LedgerSnapshot {
            epoch: self.head.map_or(0, |h| h.epoch),
            entries,
            _index: PhantomData,
        })
    }
}

/// Instante Unix corrente, para a janela `Until` de fatos (6.1).
fn unix_now() -> Result<u64, LedgerError> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| LedgerError::Invalid("system clock before the Unix epoch"))
}

pub(crate) fn write_head(
    connection: &Connection,
    head: Option<LedgerHead>,
) -> Result<(), LedgerError> {
    let seq = head.map(|h| h.seq as i64);
    let epoch = head.map(|h| h.epoch.to_be_bytes().to_vec());
    let hash = head.map(|h| h.hash.as_bytes().to_vec());
    connection.execute(
        "INSERT INTO metadata(singleton,seq,epoch,hash) VALUES(1,?1,?2,?3) ON CONFLICT(singleton) DO UPDATE SET seq=excluded.seq,epoch=excluded.epoch,hash=excluded.hash",
        params![seq, epoch, hash],
    )?;
    Ok(())
}

/// LedgerView de referências preparadas e objetos contextualizados/conferidos.
pub struct LedgerSnapshot<'a> {
    epoch: u64,
    entries: BTreeMap<PremiseRef, Option<ResolvedEntry>>,
    _index: PhantomData<&'a ()>,
}

impl LedgerView for LedgerSnapshot<'_> {
    fn current_epoch(&self) -> u64 {
        self.epoch
    }

    fn lookup(
        &self,
        reference: PremiseRef,
    ) -> Result<Option<LedgerEntry<'_>>, logos_obligation::LedgerError> {
        let entry = self
            .entries
            .get(&reference)
            .ok_or(logos_obligation::LedgerError::Unavailable)?;
        Ok(entry.as_ref().map(|entry| LedgerEntry {
            kind: entry.status.kind,
            epoch: entry.status.epoch,
            retracted: entry.status.retracted,
            expired: entry.expired,
            untrusted: entry.untrusted,
            vocabulary: &entry.vocabulary,
            formula: &entry.formula,
        }))
    }
}
