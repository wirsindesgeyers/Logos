//! Autenticação histórica e composição com checkers, sem executar artefatos CAS.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use logos_air::Vocabulary;
use logos_canon::from_canon;
use logos_core::CheckOutcome;
use logos_crypto::{ExternalDigest, Hash, PublicKey, hash_raw};
use logos_ground::{Cnf, GroundLimits, clausify, ground_for_check};
use logos_lrat::{CakeRegistry, LratCertificate, LratChecker};
use logos_model::{FiniteModel, ModelChecker};
use logos_obligation::{DecodeLimits, Obligation, vocabulary_for_check};

use crate::event::{Certificate, Checker, Parsed, Recheck};
use crate::{AuditError, Report, Result, event, fail, source};

pub(crate) fn walk(
    path: &Path,
    key: &PublicKey,
    mut visit: impl FnMut(&Parsed, &[u8]) -> Result<()>,
) -> Result<Report> {
    let mut report = Report::default();
    let mut epoch = 0;
    source::visit(path, |sql_seq, wire| {
        let p =
            event::parse(wire, key).map_err(|e| fail(format!("event {}: {e}", report.events)))?;
        if p.seq != report.events
            || p.seq > i64::MAX as u64
            || sql_seq.is_some_and(|s| s != p.seq)
            || p.prev != report.head
        {
            return Err(fail(format!("event {}: seq/prev mismatch", report.events)));
        }
        if p.advance {
            if p.epoch <= epoch {
                return Err(fail("epoch did not advance strictly"));
            }
        } else if p.epoch != epoch {
            return Err(fail("epoch changed without EpochAdvanced"));
        }
        visit(&p, wire).map_err(|e| fail(format!("event {}: {e}", p.seq)))?;
        epoch = p.epoch;
        report.events += 1;
        report.head = Some(p.digest);
        report.unchecked_proofs += u64::from(p.proof);
        Ok(())
    })?;
    Ok(report)
}

/// Confere cadeia, estrutura, assinaturas, objetos e todas as refutações/modelos.
///
/// # Errors
/// Falha explícita em entrada ausente/adulterada, recheck Rejected/Unknown ou I/O.
pub fn verify(path: &Path, cas: &Path, key: &PublicKey) -> std::result::Result<Report, AuditError> {
    verify_with_cake(path, cas, key, None)
}

/// Recheca camada1 com os dois checkers; executável permitido vem do operador.
///
/// # Errors
/// Recusa checker ausente/desconhecido, CAS inválido e resultado diferente de Accepted.
pub fn verify_with_cake(
    path: &Path,
    cas: &Path,
    key: &PublicKey,
    cake: Option<&CakeRegistry>,
) -> std::result::Result<Report, AuditError> {
    verify_with_checkers(path, cas, key, cake, &[])
}

/// Recheca também identidades Rust explicitamente permitidas pelo operador.
/// O hash do ledger nunca amplia a lista; nenhum objeto CAS é executado.
/// Provas LRAT continuam exigindo os dois checkers independentes.
/// # Errors
/// Mesmas recusas de `verify_with_cake`, incluindo identidade fora da lista.
pub fn verify_with_checkers(
    path: &Path,
    cas: &Path,
    key: &PublicKey,
    cake: Option<&CakeRegistry>,
    rust_checkers: &[[u8; 32]],
) -> std::result::Result<Report, AuditError> {
    let mut refutations = 0;
    let mut consistencies = 0;
    let mut cnf_derivations = 0;
    let mut cnfs = BTreeMap::new();
    let mut lrat_proofs = 0;
    let mut report = walk(path, key, |p, _| {
        for (domain, digest) in &p.refs {
            match *domain {
                "vocab" => {
                    source::cas::<Vocabulary>(cas, Hash::from_bytes(*digest))?;
                }
                "obligation" => {
                    source::cas::<Obligation>(cas, Hash::from_bytes(*digest))?;
                }
                "model" => {
                    source::cas::<FiniteModel>(cas, Hash::from_bytes(*digest))?;
                }
                "certificate" => {
                    source::cas::<Certificate>(cas, Hash::from_bytes(*digest))?;
                }
                "cnf" => {
                    source::cas::<Cnf>(cas, Hash::from_bytes(*digest))?;
                }
                "checker" => {
                    source::cas::<Checker>(cas, Hash::from_bytes(*digest))?;
                }
                _ => return Err(fail("unhandled CAS reference domain")),
            }
        }
        if let Some(d) = &p.derivation {
            let expected = d
                .cnf
                .ok_or_else(|| fail("LRAT derivation metadata missing"))?;
            let obligation = source::cas(cas, d.obligation)?;
            let vh = vocabulary_for_check(&obligation, d.obligation, DecodeLimits::default())
                .map_err(|e| fail(format!("CNF obligation: {e}")))?;
            let v = from_canon::<Vocabulary>(&source::cas(cas, vh)?)?;
            let ground = ground_for_check(
                &obligation,
                d.obligation,
                &v,
                DecodeLimits::default(),
                GroundLimits::default(),
            )
            .map_err(|e| fail(format!("CNF grounding: {e}")))?;
            let (cnf, _) = clausify(&ground).map_err(|e| fail(format!("CNF encoding: {e}")))?;
            let hash = Hash::<Cnf>::from_bytes(expected);
            let bytes = source::cas(cas, hash)?;
            if hash_raw::<Cnf>(cnf.dimacs()) != hash
                || cnf.dimacs() != bytes
                || ExternalDigest::of(cnf.dimacs()).as_bytes() != &d.artifact
            {
                return Err(fail("CNF rederivation mismatch"));
            }
            cnf_derivations += 1;
            cnfs.insert(d.obligation, hash);
        }
        if let Some(c) = &p.proof_check {
            if let Some(&hash) = cnfs.get(&c.obligation) {
                let cake = cake.ok_or_else(|| fail("LRAT recheck requires configured cake_lpr"))?;
                if c.checker != *cake.identity().as_bytes() && !rust_checkers.contains(&c.checker) {
                    return Err(fail("LRAT checker identity not allowed"));
                }
                let cnf = source::cas(cas, hash)?;
                let proof = source::cas(cas, c.certificate)?;
                let rust = LratChecker::new()?.check_lrat(&cnf, hash, &proof, &c.budget);
                // Os dois checkers são executados, inclusive quando Rust rejeita.
                let external = cake
                    .check_bytes(
                        &cnf,
                        hash,
                        &proof,
                        Hash::<LratCertificate>::from_bytes(*c.certificate.as_bytes()),
                        &c.budget,
                    )
                    .map_err(|e| fail(format!("cake_lpr recheck: {e}")))?;
                if rust != CheckOutcome::Accepted || external != CheckOutcome::Accepted {
                    return Err(fail(format!(
                        "LRAT recheck Rust={rust:?} cake_lpr={external:?}"
                    )));
                }
                lrat_proofs += 1;
            } else if cake.is_some_and(|cake| c.checker == *cake.identity().as_bytes()) {
                return Err(fail("LRAT obligation metadata missing"));
            }
        }
        for r in &p.rechecks {
            recheck(cas, r)?;
            if r.consistency {
                consistencies += 1;
            } else {
                refutations += 1;
            }
        }
        Ok(())
    })?;
    report.refutations = refutations;
    report.consistencies = consistencies;
    report.cnf_derivations = cnf_derivations;
    report.lrat_proofs = lrat_proofs;
    report.unchecked_proofs -= lrat_proofs;
    Ok(report)
}

fn recheck(root: &Path, r: &Recheck) -> Result<()> {
    let c = &r.check;
    let obligation = source::cas(root, c.obligation)?;
    let vocabulary_hash = vocabulary_for_check(&obligation, c.obligation, DecodeLimits::default())
        .map_err(|e| fail(format!("obligation header: {e}")))?;
    let vocab = from_canon::<Vocabulary>(&source::cas(root, vocabulary_hash)?)?;
    let model_hash = Hash::<FiniteModel>::from_bytes(r.model);
    let model_bytes = source::cas(root, model_hash)?;
    let certificate = source::cas(root, c.certificate)?;
    if certificate != model_bytes {
        return Err(fail("certificate/model bytes differ"));
    }
    let checker = ModelChecker::new(&vocab)?;
    let load = |_: &Hash<FiniteModel>| Ok::<_, AuditError>(Some(model_bytes));
    let outcome = if r.consistency {
        checker.check_consistency_from_source(
            &obligation,
            c.obligation,
            load,
            model_hash,
            &c.budget,
        )?
    } else {
        checker.check_refutation_from_source(
            &obligation,
            c.obligation,
            load,
            model_hash,
            &c.budget,
        )?
    };
    if outcome != CheckOutcome::Accepted {
        return Err(fail(format!("model recheck: {outcome:?}")));
    }
    Ok(())
}

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
struct Spool {
    path: PathBuf,
    file: File,
}
impl Spool {
    fn new() -> Result<Self> {
        loop {
            let n = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("logos-audit-{}-{n}.tmp", std::process::id()));
            match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&path)
            {
                Ok(file) => return Ok(Self { path, file }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }
}
impl Drop for Spool {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Exporta eventos autenticados byte a byte em audit-stream/v1, sem recheck CAS.
/// Só escreve no consumidor após conferir todo o histórico; erro de saída pode
/// naturalmente deixar stream parcial, que o consumidor deve descartar.
///
/// # Errors
/// Propaga autenticação, leitura e escrita sem publicar prefixo como sucesso.
pub fn export(
    path: &Path,
    key: &PublicKey,
    output: &mut impl Write,
) -> std::result::Result<Report, AuditError> {
    let mut spool = Spool::new()?;
    spool.file.write_all(source::MAGIC)?;
    // A mesma passagem valida e copia: nenhuma segunda leitura mutável da origem.
    let report = walk(path, key, |_, wire| {
        spool.file.write_all(&(wire.len() as u32).to_be_bytes())?;
        spool.file.write_all(wire)?;
        Ok(())
    })?;
    spool.file.rewind()?;
    std::io::copy(&mut spool.file, output)?;
    output.flush()?;
    Ok(report)
}
