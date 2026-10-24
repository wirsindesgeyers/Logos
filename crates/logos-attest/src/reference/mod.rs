//! Atestadores de referência (6.3) e o vocabulário de exemplo que os ancora.

pub mod backup_svc;
pub mod clock;
pub mod db_monitor;
pub mod enum_source;
pub mod fs_stat;
pub mod sandbox_run;

use logos_air::{SortDecl, SymbolDecl, Vocabulary};
use logos_crypto::ExternalDigest;

use crate::sdk::AttestError;

/// Um atestador de referência: identificador, procedimento, tópico e documento.
#[derive(Debug, Clone, Copy)]
pub struct Reference {
    /// `AttestorId` dos símbolos ancorados.
    pub id: &'static str,
    /// Identificador do procedimento nos símbolos `Anchored`.
    pub procedure: &'static str,
    /// Tópico que o operador concede na configuração de confiança.
    pub topic: &'static str,
    /// Documento "procedimento" (`docs/attesters/<id>.md`).
    pub document: &'static str,
}

/// Os seis atestadores de referência, com o texto do documento de cada procedimento.
pub const REFERENCES: [Reference; 6] = [
    Reference {
        id: clock::ID,
        procedure: clock::PROCEDURE,
        topic: clock::TOPIC,
        document: include_str!("../../../../docs/attesters/clock.md"),
    },
    Reference {
        id: fs_stat::ID,
        procedure: fs_stat::PROCEDURE,
        topic: fs_stat::TOPIC,
        document: include_str!("../../../../docs/attesters/fs-stat.md"),
    },
    Reference {
        id: db_monitor::ID,
        procedure: db_monitor::PROCEDURE,
        topic: db_monitor::TOPIC,
        document: include_str!("../../../../docs/attesters/db-monitor.md"),
    },
    Reference {
        id: backup_svc::ID,
        procedure: backup_svc::PROCEDURE,
        topic: backup_svc::TOPIC,
        document: include_str!("../../../../docs/attesters/backup-svc.md"),
    },
    Reference {
        id: enum_source::ID,
        procedure: enum_source::PROCEDURE,
        topic: enum_source::TOPIC,
        document: include_str!("../../../../docs/attesters/enum-source.md"),
    },
    Reference {
        id: sandbox_run::ID,
        procedure: sandbox_run::PROCEDURE,
        topic: sandbox_run::TOPIC,
        document: include_str!("../../../../docs/attesters/sandbox-run.md"),
    },
];

/// Vocabulário de exemplo operacional: todos os símbolos dos seis atestadores, cada um
/// `Anchored` ao seu atestador e procedimento. Sorts aninhados repetidos entre atestadores
/// (`File`, `Table`, `Database`, `Digest`) aparecem uma vez.
///
/// # Errors
/// Nomes inválidos ou declaração repetida.
pub fn example_vocabulary() -> Result<Vocabulary, AttestError> {
    let digest = |byte: u8| ExternalDigest::of(&[byte]);
    let (mut sorts, mut symbols): (Vec<SortDecl>, Vec<SymbolDecl>) = (Vec::new(), Vec::new());
    let (enum_vocabulary, _) =
        enum_source::closure("User", &["alice".to_owned(), "bob".to_owned()])?;
    let parts = [
        clock::declarations()?,
        fs_stat::declarations("config_file")?,
        db_monitor::declarations(&["users"], &["main_db"])?,
        backup_svc::declarations("main_db_backup")?,
        sandbox_run::declarations(&[digest(1), digest(2), digest(3)])?,
        (
            enum_vocabulary.sorts().to_vec(),
            enum_vocabulary.symbols().to_vec(),
        ),
    ];
    for (new_sorts, new_symbols) in parts {
        for sort in new_sorts {
            if !sorts.iter().any(|s: &SortDecl| s.name == sort.name) {
                sorts.push(sort);
            }
        }
        for symbol in new_symbols {
            if !symbols
                .iter()
                .any(|s: &SymbolDecl| s.signature.name == symbol.signature.name)
            {
                symbols.push(symbol);
            }
        }
    }
    Ok(Vocabulary::new(1, sorts, symbols)?)
}
