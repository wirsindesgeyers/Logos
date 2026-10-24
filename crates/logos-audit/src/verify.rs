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

use crate::event::{Certificate, Checker, FactObject, Parsed, PolicyObject, Recheck, TrustObject};
use crate::{AuditError, Report, Result, event, fail, source};

pub(crate) fn walk(
    path: &Path,
    key: &PublicKey,
    mut visit: impl FnMut(&Parsed, &[u8]) -> Result<()>,
) -> Result<Report> {
    let mut report = Report::default();
    let mut epoch = 0;
    let mut claims = BTreeMap::new();
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
        if let Some((target, obligation)) = p.consistency_target
            && claims.get(&target) != Some(&(obligation, p.epoch))
        {
            return Err(fail("consistency target obligation/epoch mismatch"));
        }
        if let Some(obligation) = p.claim_obligation {
            claims.insert(p.digest, (obligation, p.epoch));
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
    // 5.10: rotas LRAT por evento; as que falham só passam se um Rechecked
    // Rejected posterior as invalidar (incidente reportado, nunca escondido).
    let mut routes: BTreeMap<[u8; 32], (Hash<Obligation>, Hash<Certificate>)> = BTreeMap::new();
    let mut failed: BTreeMap<[u8; 32], String> = BTreeMap::new();
    let mut invalidated_by_recheck = 0;
    let mut facts = 0;
    let mut trust = Grants::default();
    let mut policy_version = 0u64;
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
                "fact" => {
                    let (issuer, topic) = fact_signature(&source::cas::<FactObject>(
                        cas,
                        Hash::from_bytes(*digest),
                    )?)?;
                    if !trust.grants.contains(&(issuer, topic)) {
                        return Err(fail("fact issuer not trusted for its topic"));
                    }
                    if trust.revoked.contains(&issuer) {
                        return Err(fail("fact from a revoked issuer"));
                    }
                    facts += 1;
                }
                "issuer-revoked" => {
                    if !trust.revoked.insert(*digest) {
                        return Err(fail("issuer revoked twice"));
                    }
                }
                "policy" => {
                    source::cas::<PolicyObject>(cas, Hash::from_bytes(*digest))?;
                }
                "trust" => {
                    trust.publish(&source::cas::<TrustObject>(cas, Hash::from_bytes(*digest))?)?;
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
                routes.insert(p.digest, (c.obligation, c.certificate));
                if rust != CheckOutcome::Accepted || external != CheckOutcome::Accepted {
                    failed.insert(
                        p.digest,
                        format!("LRAT recheck Rust={rust:?} cake_lpr={external:?}"),
                    );
                } else {
                    lrat_proofs += 1;
                }
            } else if cake.is_some_and(|cake| c.checker == *cake.identity().as_bytes()) {
                return Err(fail("LRAT obligation metadata missing"));
            }
        }
        if let Some((policy, version, root)) = p.policy {
            if version <= policy_version {
                return Err(fail("policy version not increasing"));
            }
            policy_version = version;
            // Rederiva a raiz Merkle do objeto no CAS: um evento com raiz forjada não passa.
            let bytes = source::cas(cas, Hash::<PolicyObject>::from_bytes(policy))?;
            let header: logos_policy::Policy = from_canon(&bytes)?;
            let vocabulary = from_canon::<Vocabulary>(&source::cas(cas, header.vocabulary())?)?;
            let decoded = logos_policy::Policy::decode(&bytes, &vocabulary)
                .map_err(|e| fail(format!("published policy: {e}")))?;
            if decoded.version() != version {
                return Err(fail("policy version differs from the object"));
            }
            let tree = logos_policy::PolicyTree::new(&decoded)
                .map_err(|e| fail(format!("policy tree: {e}")))?;
            if tree.root() != &root {
                return Err(fail("policy root does not match the published object"));
            }
        }
        if let Some((route, c)) = &p.rechecked {
            let cake = cake.ok_or_else(|| fail("Rechecked requires configured cake_lpr"))?;
            if c.checker != *cake.identity().as_bytes() && !rust_checkers.contains(&c.checker) {
                return Err(fail("Rechecked checker identity not allowed"));
            }
            if routes.get(route) != Some(&(c.obligation, c.certificate)) {
                return Err(fail("Rechecked does not match an earlier LRAT route"));
            }
            match (&c.outcome, failed.contains_key(route)) {
                (CheckOutcome::Rejected(_), true) => {
                    failed.remove(route);
                    invalidated_by_recheck += 1;
                }
                (CheckOutcome::Rejected(_), false) => {
                    return Err(fail("Rechecked rejection not reproduced by the auditor"));
                }
                (CheckOutcome::Accepted, true) => {
                    return Err(fail("Rechecked accepted a certificate the auditor rejects"));
                }
                _ => {}
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
    if let Some((route, reason)) = failed.iter().next() {
        return Err(fail(format!(
            "{reason} for route {} without Rechecked rejection ({} routes)",
            crate::hex(route),
            failed.len()
        )));
    }
    report.invalidated_by_recheck = invalidated_by_recheck;
    report.facts = facts;
    report.trust_configs = trust.configs;
    report.refutations = refutations;
    report.consistencies = consistencies;
    report.cnf_derivations = cnf_derivations;
    report.lrat_proofs = lrat_proofs;
    report.unchecked_proofs -= lrat_proofs + invalidated_by_recheck;
    Ok(report)
}

/// Concessões `Trusted(KeyId, tópico)` da configuração vigente no ponto da caminhada.
#[derive(Default)]
struct Grants {
    grants: std::collections::BTreeSet<([u8; 32], String)>,
    version: Option<u64>,
    configs: u64,
    /// Emissores revogados até o ponto da caminhada (6.4).
    revoked: std::collections::BTreeSet<[u8; 32]>,
}

impl Grants {
    /// Reaplica trust/v1 (6.2) com decoder próprio e estrito (trust.md §1):
    /// `[1, version≥1, roots]`, 1..1024 raízes `[PublicKey, name, topics, level]` em
    /// ordem estrita de KeyId, 1..256 tópicos em ordem estrita de bytes, textos não
    /// vazios de até 4096 bytes; versão estritamente crescente.
    fn publish(&mut self, bytes: &[u8]) -> Result<()> {
        use logos_canon::{CanonDecode, Text, Value};
        let items = |value: &Value| match value {
            Value::Array(array) => Ok(array.items().to_vec()),
            _ => Err(fail("trust config shape")),
        };
        let text = |value: &Value| -> Result<String> {
            let text = Text::from_value(value)?.as_str().to_owned();
            if text.is_empty() || text.len() > 4096 {
                return Err(fail("trust config text"));
            }
            Ok(text)
        };
        let value: Value = from_canon(bytes)?;
        let [Value::Unsigned(1), Value::Unsigned(version), roots] = &items(&value)?[..] else {
            return Err(fail("trust config version/arity"));
        };
        if *version == 0 || self.version.is_some_and(|current| *version <= current) {
            return Err(fail("trust config version not increasing"));
        }
        let roots = items(roots)?;
        if roots.is_empty() || roots.len() > 1024 {
            return Err(fail("trust config roots"));
        }
        let mut grants = std::collections::BTreeSet::new();
        let mut previous: Option<[u8; 32]> = None;
        for root in roots {
            let [issuer, name, topics, Value::Unsigned(_level)] = &items(&root)?[..] else {
                return Err(fail("trust root arity"));
            };
            text(name)?;
            let issuer = *logos_crypto::hash(&PublicKey::from_value(issuer)?)?.as_bytes();
            if previous.is_some_and(|p| p >= issuer) {
                return Err(fail("trust roots not in strict KeyId order"));
            }
            previous = Some(issuer);
            let topics = items(topics)?;
            if topics.is_empty() || topics.len() > 256 {
                return Err(fail("trust root topics"));
            }
            let mut last: Option<String> = None;
            for topic in &topics {
                let topic = text(topic)?;
                if last
                    .as_ref()
                    .is_some_and(|l| l.as_bytes() >= topic.as_bytes())
                {
                    return Err(fail("trust topics not in strict order"));
                }
                grants.insert((issuer, topic.clone()));
                last = Some(topic);
            }
        }
        self.grants = grants;
        self.version = Some(*version);
        self.configs += 1;
        Ok(())
    }
}

/// Reverifica a assinatura do emissor de um fact/v1 (6.1) sem o decoder do ledger:
/// corpo = os oito primeiros campos, assinatura = o nono. Devolve o `KeyId` do
/// emissor e o tópico, para a regra de confiança da 6.2.
fn fact_signature(bytes: &[u8]) -> Result<([u8; 32], String)> {
    let value: logos_canon::Value = from_canon(bytes)?;
    let logos_canon::Value::Array(array) = value else {
        return Err(fail("fact is not an array"));
    };
    let [first, .., sig] = array.items() else {
        return Err(fail("fact arity"));
    };
    if array.items().len() != 9 || *first != logos_canon::Value::Unsigned(1) {
        return Err(fail("fact version/arity"));
    }
    let body = logos_canon::to_canon(&logos_canon::Value::Array(logos_canon::Array::new(
        array.items()[..8].to_vec(),
    )?))?;
    let issuer = <PublicKey as logos_canon::CanonDecode>::from_value(&array.items()[3])?;
    let sig = <logos_crypto::Signature as logos_canon::CanonDecode>::from_value(sig)?;
    logos_crypto::verify::<FactSig>(&body, &sig, &issuer)
        .map_err(|_| fail("fact signature does not verify"))?;
    let topic = <logos_canon::Text as logos_canon::CanonDecode>::from_value(&array.items()[4])?;
    Ok((
        *logos_crypto::hash(&issuer)?.as_bytes(),
        topic.as_str().to_owned(),
    ))
}

struct FactSig;
impl logos_crypto::SigDomain for FactSig {
    const TAG: &'static [u8] = logos_crypto::sig_tags::FACT;
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
