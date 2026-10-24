//! `sandbox-run`: execução determinística isolada (`bwrap` + `prlimit`, sem rede) que atesta
//! `computed(h_script, h_input, h_output)`.
//!
//! Fecha a cadeia criptográfica entre dado bruto, receita e saída. Não promete verdade
//! empírica: o fato diz que *esta* receita sobre *este* dado produziu *esta* saída (L1).
//! Duas execuções com saídas diferentes não atestam nada.

use std::{
    fs,
    io::Read,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use logos_air::{SortDecl, SortDomain, SymbolDecl, Vocabulary};
use logos_canon::Text;
use logos_crypto::ExternalDigest;

use crate::sdk::{
    AttestError, Attester, Observation, anchored, atom, constant, predicate, resource_version,
    sort, until,
};

/// Identificador do atestador.
pub const ID: &str = "sandbox-run";
/// Procedimento (documento `docs/attesters/sandbox-run.md`).
pub const PROCEDURE: &str = "sandbox-run/bwrap-prlimit/v1";
/// Tópico atestado.
pub const TOPIC: &str = "computation";

const BWRAP: &str = "/usr/bin/bwrap";
const PRLIMIT: &str = "/usr/bin/prlimit";

/// Limites da execução.
#[derive(Debug, Clone, Copy)]
pub struct RunLimits {
    /// Tempo de parede.
    pub wall: Duration,
    /// CPU em segundos (`prlimit --cpu`).
    pub cpu_secs: u64,
    /// Espaço de endereçamento em bytes (`prlimit --as`).
    pub address_space: u64,
    /// Teto da saída capturada.
    pub max_output: usize,
}

impl Default for RunLimits {
    fn default() -> Self {
        Self {
            wall: Duration::from_secs(20),
            cpu_secs: 10,
            address_space: 1 << 30,
            max_output: 1 << 20,
        }
    }
}

/// Executa `script` (shell POSIX) sobre `input` e atesta a tripla de hashes.
#[derive(Debug)]
pub struct SandboxRun {
    script: PathBuf,
    input: PathBuf,
    limits: RunLimits,
    ttl_secs: u64,
}

impl SandboxRun {
    /// Caminhos absolutos de arquivos regulares existentes.
    pub fn new(
        script: impl Into<PathBuf>,
        input: impl Into<PathBuf>,
        limits: RunLimits,
        ttl_secs: u64,
    ) -> Self {
        Self {
            script: script.into(),
            input: input.into(),
            limits,
            ttl_secs,
        }
    }
}

/// Nome da constante de um digest.
#[must_use]
pub fn digest_name(digest: &ExternalDigest) -> String {
    format!("sha256:{}", digest.to_hex())
}

/// Símbolos para os digests dados (sem repetição).
///
/// # Errors
/// Nomes inválidos.
pub fn declarations(
    digests: &[ExternalDigest],
) -> Result<(Vec<SortDecl>, Vec<SymbolDecl>), AttestError> {
    let of = sort("Digest")?;
    let mut decls = Vec::new();
    let mut seen = Vec::new();
    for digest in digests {
        if seen.contains(digest.as_bytes()) {
            continue;
        }
        seen.push(*digest.as_bytes());
        decls.push(anchored(
            constant(&digest_name(digest), &of)?,
            ID,
            PROCEDURE,
            "digest SHA-256 de um artefato local",
        )?);
    }
    decls.push(anchored(
        predicate("computed", vec![of.clone(), of.clone(), of])?,
        ID,
        PROCEDURE,
        "o script, aplicado à entrada, produziu a saída (por hashes)",
    )?);
    Ok((
        vec![SortDecl {
            name: Text::new("Digest")?,
            domain: SortDomain::Open,
        }],
        decls,
    ))
}

fn regular_file(path: &Path, what: &'static str) -> Result<Vec<u8>, AttestError> {
    if !path.is_absolute() {
        return Err(AttestError::Invalid("sandbox-run paths must be absolute"));
    }
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() {
        return Err(AttestError::Observation(format!(
            "{what} is not a regular file"
        )));
    }
    if meta.len() > 64 * 1024 * 1024 {
        return Err(AttestError::Observation(format!("{what} above 64 MiB")));
    }
    Ok(fs::read(path)?)
}

fn run_once(script: &Path, input: &Path, limits: &RunLimits) -> Result<Vec<u8>, AttestError> {
    let mut command = Command::new(PRLIMIT);
    command
        .arg(format!("--cpu={}", limits.cpu_secs))
        .arg(format!("--as={}", limits.address_space))
        .arg("--fsize=1048576")
        .arg("--core=0")
        .arg("--")
        .arg(BWRAP)
        .args([
            "--unshare-user",
            "--unshare-pid",
            "--unshare-net",
            "--unshare-ipc",
            "--unshare-uts",
            "--unshare-cgroup-try",
            "--die-with-parent",
            "--new-session",
            "--clearenv",
            "--cap-drop",
            "ALL",
            "--ro-bind",
            "/usr",
            "/usr",
            "--symlink",
            "usr/bin",
            "/bin",
            "--symlink",
            "usr/lib",
            "/lib",
            "--symlink",
            "usr/lib64",
            "/lib64",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--dir",
            "/work",
            "--ro-bind",
        ])
        .arg(script)
        .arg("/work/script.sh")
        .arg("--ro-bind")
        .arg(input)
        .arg("/work/input")
        .args([
            "--setenv",
            "PATH",
            "/usr/bin",
            "--setenv",
            "LC_ALL",
            "C",
            "--setenv",
            "TZ",
            "UTC",
            "--chdir",
            "/work",
            "--",
            "/usr/bin/sh",
            "/work/script.sh",
            "/work/input",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    let mut child = command
        .spawn()
        .map_err(|e| AttestError::Observation(format!("cannot start the sandbox: {e}")))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| AttestError::Observation("no stdout pipe".into()))?;
    let cap = limits.max_output;
    let exceeded = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = std::sync::Arc::clone(&exceeded);
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let n = stdout.read(&mut chunk)?;
            if n == 0 {
                return Ok::<_, std::io::Error>((output, false));
            }
            if output.len() + n > cap {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                return Ok((output, true));
            }
            output.extend_from_slice(&chunk[..n]);
        }
    });
    let deadline = Instant::now() + limits.wall;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline || exceeded.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            let (_, over) = reader
                .join()
                .map_err(|_| AttestError::Observation("reader thread panicked".into()))??;
            return Err(AttestError::Observation(if over {
                "output above the cap".into()
            } else {
                "wall-clock limit exceeded".into()
            }));
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let (output, over) = reader
        .join()
        .map_err(|_| AttestError::Observation("reader thread panicked".into()))??;
    if over {
        return Err(AttestError::Observation("output above the cap".into()));
    }
    if !status.success() {
        return Err(AttestError::Observation(format!(
            "script failed inside the sandbox: {status}"
        )));
    }
    Ok(output)
}

impl Attester for SandboxRun {
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
        let script_bytes = regular_file(&self.script, "script")?;
        let input_bytes = regular_file(&self.input, "input")?;
        let first = run_once(&self.script, &self.input, &self.limits)?;
        let second = run_once(&self.script, &self.input, &self.limits)?;
        if first != second {
            return Err(AttestError::Observation(
                "two runs produced different output: not deterministic".into(),
            ));
        }
        // O script e a entrada não podem ter mudado durante a execução.
        if regular_file(&self.script, "script")? != script_bytes
            || regular_file(&self.input, "input")? != input_bytes
        {
            return Err(AttestError::Observation(
                "script or input changed while running".into(),
            ));
        }
        let script = ExternalDigest::of(&script_bytes);
        let input = ExternalDigest::of(&input_bytes);
        let output = ExternalDigest::of(&first);
        let (sorts, decls) = declarations(&[script, input, output])?;
        let find = |name: &str| {
            decls
                .iter()
                .find(|d| d.signature.name.as_str() == name)
                .map(|d| d.signature.clone())
                .ok_or(AttestError::Invalid("symbol missing from declarations"))
        };
        let formula = atom(
            &find("computed")?,
            &[
                &find(&digest_name(&script))?,
                &find(&digest_name(&input))?,
                &find(&digest_name(&output))?,
            ],
        )?;
        Ok(Observation {
            vocabulary: Vocabulary::new(1, sorts, decls)?,
            formula,
            version: Some(resource_version(
                "sandbox-run",
                &format!(
                    "script={};input={};output={}",
                    script.to_hex(),
                    input.to_hex(),
                    output.to_hex()
                ),
            )?),
            valid: until(now, self.ttl_secs)?,
        })
    }
}
