//! Appender exclusivo com replay completo e commits WAL/FULL.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::Path;

use logos_canon::to_canon;
use logos_crypto::{Hash, PublicKey, Signer, hash};
use rusqlite::{Connection, OpenFlags, TransactionBehavior, types::ValueRef};

use crate::{Event, EventBody, LedgerError, ledger_store};

/// Cabeça conferida da cadeia, sem autoridade lógica ou garantia anti-rollback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LedgerHead {
    /// Última posição confirmada, iniciando em zero.
    pub seq: u64,
    /// Época do último evento confirmado.
    pub epoch: u64,
    /// Hash do evento completo, incluindo assinatura.
    pub hash: Hash<Event>,
}

fn next_seq(head: Option<LedgerHead>) -> Result<u64, LedgerError> {
    match head {
        None => Ok(0),
        Some(head) if head.seq < i64::MAX as u64 => Ok(head.seq + 1),
        Some(_) => Err(LedgerError::SequenceExhausted),
    }
}

fn event_epoch(head: Option<LedgerHead>, body: &EventBody) -> Result<u64, LedgerError> {
    let epoch = head.map_or(0, |head| head.epoch);
    if let EventBody::EpochAdvanced { new_epoch, .. } = body {
        if *new_epoch <= epoch {
            return Err(LedgerError::Invalid("epoch must increase"));
        }
        Ok(*new_epoch)
    } else {
        Ok(epoch)
    }
}

fn advance(
    head: Option<LedgerHead>,
    event: &Event,
    public_key: &PublicKey,
) -> Result<LedgerHead, LedgerError> {
    event.verify(public_key)?;
    if event.seq != next_seq(head)? || event.prev != head.map(|head| head.hash) {
        return Err(LedgerError::Invalid("sequence/previous hash"));
    }
    if event.epoch != event_epoch(head, &event.body)? {
        return Err(LedgerError::Invalid("epoch continuity"));
    }
    Ok(LedgerHead {
        seq: event.seq,
        epoch: event.epoch,
        hash: hash(event)?,
    })
}

fn replay(
    connection: &Connection,
    public_key: &PublicKey,
) -> Result<Option<LedgerHead>, LedgerError> {
    let mut head = None;
    let mut statement = connection.prepare("SELECT seq, wire FROM events ORDER BY seq")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let seq: i64 = row.get(0)?;
        let ValueRef::Blob(bytes) = row.get_ref(1)? else {
            return Err(LedgerError::Invalid("wire is not a BLOB"));
        };
        let event = Event::decode(bytes)?;
        if seq < 0 || event.seq != seq as u64 {
            return Err(LedgerError::Invalid(
                "row sequence differs from signed sequence",
            ));
        }
        head = Some(advance(head, &event, public_key)?);
    }
    Ok(head)
}

/// Escritor único de ledger-sqlite/v1. A conexão e o lock nunca são expostos.
///
/// O diretório deve estar em filesystem local, sob controle do dono, e seu
/// pai deve existir. Mesmo Signer/chave durante toda a vida do ledger.
/// Campos são descartados na ordem declarada: conexão fecha antes do lock.
pub struct Ledger<'a, S: Signer> {
    connection: Connection,
    directory: File,
    _writer_lock: File,
    signer: &'a S,
    public_key: PublicKey,
    head: Option<LedgerHead>,
    poisoned: bool,
}

impl<'a, S: Signer> Ledger<'a, S> {
    /// Abre/cria um diretório e confere schema, assinaturas e cadeia inteira.
    /// Ledger vazio aceita somente Genesis como primeiro append.
    ///
    /// # Errors
    /// Lock ocupado, I/O/SQLite, schema/cadeia/chave/assinatura inválidos.
    /// Não remove, trunca nem repara histórico inválido.
    pub fn open(directory: &Path, signer: &'a S) -> Result<Self, LedgerError> {
        match fs::create_dir(directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let path = fs::canonicalize(directory)?;
        let parent = path
            .parent()
            .ok_or(LedgerError::Invalid("directory parent"))?;
        File::open(parent)?.sync_all()?;
        let directory = File::open(&path)?;
        if !directory.metadata()?.is_dir() {
            return Err(LedgerError::Invalid("ledger directory"));
        }
        let writer_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.join("writer.lock"))?;
        if !writer_lock.metadata()?.is_file() {
            return Err(LedgerError::Invalid("writer lock is not a regular file"));
        }
        match writer_lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(LedgerError::WriterBusy),
            Err(TryLockError::Error(error)) => return Err(error.into()),
        }
        let mut connection = Connection::open_with_flags(
            path.join("ledger.sqlite3"),
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        ledger_store::configure(&connection)?;
        let public_key = signer.public_key().clone();
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        ledger_store::schema(&transaction)?;
        let head = replay(&transaction, &public_key)?;
        transaction.commit()?;
        directory.sync_all()?;
        Ok(Self {
            connection,
            directory,
            _writer_lock: writer_lock,
            signer,
            public_key,
            head,
            poisoned: false,
        })
    }

    /// Cabeça confirmada; após falha de persistência exige reabertura.
    ///
    /// # Errors
    /// Poisoned se não é possível afirmar o resultado da última escrita.
    pub fn head(&self) -> Result<Option<LedgerHead>, LedgerError> {
        if self.poisoned {
            return Err(LedgerError::Poisoned);
        }
        Ok(self.head)
    }

    /// Única operação de escrita de eventos: calcula cabeçalho, assina e commita.
    /// Só devolve o evento depois do commit e sync do diretório.
    ///
    /// # Errors
    /// Body/assinante inválido não consome seq. Falha de persistência torna o
    /// handle Poisoned; reabertura decide se o evento foi confirmado ou não.
    pub fn append(&mut self, body: EventBody) -> Result<Event, LedgerError> {
        if self.poisoned {
            return Err(LedgerError::Poisoned);
        }
        let event = Event::sign(
            next_seq(self.head)?,
            event_epoch(self.head, &body)?,
            self.head.map(|head| head.hash),
            body,
            self.signer,
        )?;
        // Também confere assinantes defeituosos antes de persistir bytes.
        let head = advance(self.head, &event, &self.public_key)?;
        let wire = to_canon(&event)?;
        self.poisoned = true;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = transaction.execute(
            "INSERT INTO events(seq, wire) VALUES (?1, ?2)",
            (event.seq as i64, wire),
        )?;
        if inserted != 1 {
            return Err(LedgerError::Invalid("append did not insert one event"));
        }
        transaction.commit()?;
        self.directory.sync_all()?;
        self.head = Some(head);
        self.poisoned = false;
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos_crypto::FileSigner;

    #[test]
    fn sqlite_sequence_limit_is_explicit() {
        let head = LedgerHead {
            seq: i64::MAX as u64 - 1,
            epoch: 0,
            hash: Hash::from_bytes([0; 32]),
        };
        assert_eq!(next_seq(Some(head)).unwrap(), i64::MAX as u64);
        assert!(matches!(
            next_seq(Some(LedgerHead {
                seq: i64::MAX as u64,
                ..head
            })),
            Err(LedgerError::SequenceExhausted)
        ));
        assert!(matches!(
            next_seq(Some(LedgerHead {
                seq: u64::MAX,
                ..head
            })),
            Err(LedgerError::SequenceExhausted)
        ));
    }

    #[test]
    fn actual_writer_uses_wal_full_and_checked_pragmas() {
        let path =
            std::env::temp_dir().join(format!("logos-ledger-pragmas-{}", std::process::id()));
        let signer = FileSigner::from_seed(&[0x11; 32]).unwrap();
        let ledger = Ledger::open(&path, &signer).unwrap();
        for (pragma, expected) in [
            ("synchronous", 2),
            ("fullfsync", 1),
            ("checkpoint_fullfsync", 1),
            ("trusted_schema", 0),
        ] {
            let actual: i64 = ledger
                .connection
                .pragma_query_value(None, pragma, |r| r.get(0))
                .unwrap();
            assert_eq!(actual, expected, "{pragma}");
        }
        let mode: String = ledger
            .connection
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        drop(ledger);
        fs::remove_dir_all(path).unwrap();
    }
}
