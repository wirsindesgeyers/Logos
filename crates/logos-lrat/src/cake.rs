//! Registry local: só executa identidades fixadas, nunca um checker escolhido pelo ledger.

use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use logos_core::{Budget, CheckOutcome, Exhausted};
use logos_crypto::{Domain, ExternalDigest, Hash, hash_raw, tags};

use crate::sandbox;

/// Domínio de identidade de binário, compatível com CheckerRegistered.
pub struct CakeBinary;
impl Domain for CakeBinary {
    const TAG: &'static [u8] = tags::CHECKER;
}

/// Domínio dos bytes crus do certificado no CAS.
pub struct LratCertificate;
impl Domain for LratCertificate {
    const TAG: &'static [u8] = tags::CERTIFICATE;
}

/// Erro operacional/configuração: não é refutação nem prova rejeitada.
#[derive(Debug)]
pub struct CakeError {
    pub(crate) message: String,
    pub(crate) exhausted: Option<Exhausted>,
}
impl std::fmt::Display for CakeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for CakeError {}
impl From<std::io::Error> for CakeError {
    fn from(e: std::io::Error) -> Self {
        Self {
            message: format!("cake_lpr I/O: {e}"),
            exhausted: None,
        }
    }
}
impl From<rustix::io::Errno> for CakeError {
    fn from(e: rustix::io::Errno) -> Self {
        Self {
            message: format!("cake_lpr syscall: {e}"),
            exhausted: None,
        }
    }
}
pub(crate) type Result<T> = std::result::Result<T, CakeError>;
pub(crate) fn fail(message: &str) -> CakeError {
    CakeError {
        message: message.into(),
        exhausted: None,
    }
}

/// Cadastro local de um único cake_lpr conhecido; não concede autoridade administrativa.
#[derive(Debug)]
pub struct CakeRegistry {
    path: PathBuf,
    identity: Hash<CakeBinary>,
}

impl CakeRegistry {
    /// Confere o binário contra a identidade de build fixada para esta arquitetura.
    ///
    /// # Errors
    /// Recusa arquivo ausente/irregular, hash desconhecido ou plataforma sem build fixado.
    pub fn register(path: &Path) -> Result<Self> {
        let bytes = executable(path)?;
        Ok(Self {
            path: path.to_owned(),
            identity: hash_raw(&bytes),
        })
    }

    /// Identidade no domínio logos/checker/v1, diferente do SHA-256 cru do manifesto.
    #[must_use]
    pub fn identity(&self) -> Hash<CakeBinary> {
        self.identity
    }

    /// Lê CNF/certificado pelo hash no CAS e executa os bytes conferidos em sandbox.
    ///
    /// # Errors
    /// Recusa integridade/configuração/isolamento inválidos; nunca executa binário do CAS.
    pub fn check_from_cas(
        &self,
        cas: &Path,
        cnf: Hash<logos_ground::Cnf>,
        certificate: Hash<LratCertificate>,
        budget: &Budget,
    ) -> Result<CheckOutcome> {
        let load = || {
            let bytes = cas_bytes(cas, "cnf", cnf, budget.size_bytes)?;
            let remaining = budget.size_bytes.saturating_sub(bytes.len() as u64);
            let proof = cas_bytes(cas, "certificate", certificate, remaining)?;
            self.check_bytes(&bytes, cnf, &proof, certificate, budget)
        };
        match load() {
            Err(error) if error.exhausted.is_some() => match error.exhausted {
                Some(reason) => Ok(CheckOutcome::Unknown(reason)),
                None => Err(error),
            },
            other => other,
        }
    }

    /// Executa snapshots já obtidos do CAS, conferindo novamente os dois hashes.
    /// Não chama o checker Rust nem filtra sua gramática antes do cake_lpr.
    ///
    /// # Errors
    /// Propaga falha operacional; limites reconhecidos retornam Unknown.
    pub fn check_bytes(
        &self,
        cnf: &[u8],
        expected: Hash<logos_ground::Cnf>,
        proof: &[u8],
        certificate: Hash<LratCertificate>,
        budget: &Budget,
    ) -> Result<CheckOutcome> {
        let size = cnf
            .len()
            .checked_add(proof.len())
            .ok_or_else(|| fail("input overflow"))?;
        if size as u64 > budget.size_bytes {
            return Ok(CheckOutcome::Unknown(Exhausted::Size));
        }
        if budget.mem_bytes == 0 {
            return Ok(CheckOutcome::Unknown(Exhausted::Memory));
        }
        if budget.wall_ms == 0 {
            return Ok(CheckOutcome::Unknown(Exhausted::Time));
        }
        if hash_raw(cnf) != expected || hash_raw(proof) != certificate {
            return Err(fail("CNF/certificate hash mismatch"));
        }
        // Revalidar em toda execução detecta substituição depois do cadastro.
        let executable = executable(&self.path)?;
        if hash_raw::<CakeBinary>(&executable) != self.identity {
            return Err(fail("registered checker changed"));
        }
        sandbox::run(&executable, cnf, proof, budget)
    }
}

fn executable(path: &Path) -> Result<Vec<u8>> {
    let bytes = read_regular(path, 8 * 1024 * 1024)?;
    if ExternalDigest::of(&bytes).as_bytes() != &crate::cake_known::SHA256 {
        return Err(fail("unknown cake_lpr binary hash"));
    }
    Ok(bytes)
}

fn cas_bytes<D: Domain>(root: &Path, name: &str, hash: Hash<D>, limit: u64) -> Result<Vec<u8>> {
    let h = hash.to_hex();
    let path = root.join(format!("{name}-v1")).join(&h[..2]).join(&h[2..]);
    let bytes = read_regular(&path, limit.min(268_435_456))?;
    if hash_raw::<D>(&bytes) != hash {
        return Err(fail("CAS integrity mismatch"));
    }
    Ok(bytes)
}

fn read_regular(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut file: File = OpenOptions::new()
        .read(true)
        .custom_flags(
            rustix::fs::OFlags::NOFOLLOW.bits() as i32 | rustix::fs::OFlags::NONBLOCK.bits() as i32,
        )
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(fail("not a regular file"));
    }
    if file.metadata()?.len() > limit {
        return Err(size_limit());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(usize::try_from(file.metadata()?.len()).map_err(|_| fail("file size"))?)
        .map_err(|_| fail("allocation failed"))?;
    let mut chunk = [0_u8; 65_536];
    loop {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        if (bytes.len() as u64).saturating_add(n as u64) > limit {
            return Err(size_limit());
        }
        bytes
            .try_reserve(n)
            .map_err(|_| fail("allocation failed"))?;
        bytes.extend_from_slice(&chunk[..n]);
    }
    Ok(bytes)
}

fn size_limit() -> CakeError {
    CakeError {
        message: "file size limit".into(),
        exhausted: Some(Exhausted::Size),
    }
}
