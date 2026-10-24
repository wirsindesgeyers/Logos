//! `fs-stat`: existência, hash e mtime de um arquivo, versionados pelo hash do conteúdo.

use std::{fs, io::Read, path::PathBuf, time::UNIX_EPOCH};

use logos_air::{SortDecl, SortDomain, SymbolDecl, Vocabulary};
use logos_canon::Text;
use logos_crypto::ExternalDigest;

use crate::sdk::{
    AttestError, Attester, Observation, anchored, constant, literal, predicate, resource_version,
    sort, until,
};

/// Identificador do atestador.
pub const ID: &str = "fs-stat";
/// Procedimento (documento `docs/attesters/fs-stat.md`).
pub const PROCEDURE: &str = "fs-stat/lstat-sha256/v1";
/// Tópico atestado.
pub const TOPIC: &str = "fs";
/// Teto de leitura do conteúdo para o hash.
pub const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Atesta `file_present(alias)` (ou a negação) e a versão `sha256=…;size=…;mtime=…`.
#[derive(Debug)]
pub struct FsStat {
    alias: String,
    path: PathBuf,
    ttl_secs: u64,
}

impl FsStat {
    /// `alias` é a constante do vocabulário que nomeia o arquivo; `path`, o arquivo real.
    pub fn new(alias: impl Into<String>, path: impl Into<PathBuf>, ttl_secs: u64) -> Self {
        Self {
            alias: alias.into(),
            path: path.into(),
            ttl_secs,
        }
    }
}

/// Símbolos para um arquivo de alias `alias`.
///
/// # Errors
/// Nomes inválidos.
pub fn declarations(alias: &str) -> Result<(Vec<SortDecl>, Vec<SymbolDecl>), AttestError> {
    let file = sort("File")?;
    Ok((
        vec![SortDecl {
            name: Text::new("File")?,
            domain: SortDomain::Open,
        }],
        vec![
            anchored(
                constant(alias, &file)?,
                ID,
                PROCEDURE,
                "arquivo vigiado pelo atestador fs-stat",
            )?,
            anchored(
                predicate("file_present", vec![file])?,
                ID,
                PROCEDURE,
                "o arquivo existe e é um arquivo regular",
            )?,
        ],
    ))
}

impl Attester for FsStat {
    fn id(&self) -> &str {
        ID
    }
    fn topic(&self) -> &str {
        TOPIC
    }
    fn procedure(&self) -> &'static str {
        PROCEDURE
    }
    fn observe(&mut self, now: u64) -> Result<Observation, AttestError> {
        let (sorts, decls) = declarations(&self.alias)?;
        let alias = decls[0].signature.clone();
        let present = decls[1].signature.clone();
        let (holds, version) = match fs::symlink_metadata(&self.path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(AttestError::Observation(
                    "refusing to attest through a symbolic link".into(),
                ));
            }
            Ok(meta) if meta.is_file() => {
                if meta.len() > MAX_BYTES {
                    return Err(AttestError::Observation("file above the size cap".into()));
                }
                let mut bytes = Vec::new();
                fs::File::open(&self.path)?
                    .take(MAX_BYTES)
                    .read_to_end(&mut bytes)?;
                let mtime = meta
                    .modified()?
                    .duration_since(UNIX_EPOCH)
                    .map_err(|_| AttestError::Observation("mtime before the Unix epoch".into()))?
                    .as_secs();
                let digest = ExternalDigest::of(&bytes).to_hex();
                (
                    true,
                    Some(resource_version(
                        &self.alias,
                        &format!("sha256={digest};size={};mtime={mtime}", bytes.len()),
                    )?),
                )
            }
            Ok(_) => (false, None),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (false, None),
            Err(e) => return Err(e.into()),
        };
        Ok(Observation {
            vocabulary: Vocabulary::new(1, sorts, decls)?,
            formula: literal(&present, &[&alias], holds)?,
            version,
            valid: until(now, self.ttl_secs)?,
        })
    }
}
