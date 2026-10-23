//! Sandbox do processo do agente (Etapa 5.11, ADR0059).
//!
//! O agente roda em `bwrap` com namespaces próprios de usuário, PID, rede, IPC e
//! UTS, sem capabilities e sem o home do operador. Só entram: o runtime do sistema
//! (somente leitura), o código do agente (somente leitura), o socket do kernel, o
//! diretório de staging e, opcionalmente, um socket Unix do [`LlmProxy`], que só
//! encaminha para o endereço da API do LLM fixado pelo operador. Chave do kernel,
//! ledger, CAS privado, socket/porta do gateway e credenciais de ferramentas não são
//! montados: dentro do sandbox esses caminhos não existem e não há rede.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsString;
use std::fs;
use std::io;
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// Raiz, dentro do sandbox, dos únicos canais com o mundo de fora.
pub const RUN: &str = "/run/logos";
/// Socket do kernel dentro do sandbox.
pub const KERNEL_SOCKET: &str = "/run/logos/kernel.sock";
/// Diretório de staging (fonte de certificados/modelos) dentro do sandbox.
pub const STAGING: &str = "/run/logos/staging";
/// Socket do proxy do LLM dentro do sandbox.
pub const LLM_SOCKET: &str = "/run/logos/llm.sock";
/// Código do agente, somente leitura.
pub const CODE: &str = "/opt/logos";
const BWRAP: &str = "/usr/bin/bwrap";

/// Erro explícito; nenhum caminho cai para execução sem isolamento.
#[derive(Debug)]
pub enum SandboxError {
    /// Configuração recusada antes de executar qualquer coisa.
    Invalid(&'static str),
    /// Falha de sistema ao preparar o sandbox ou o proxy.
    Io(io::Error),
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(reason) => write!(f, "invalid sandbox plan: {reason}"),
            Self::Io(error) => write!(f, "sandbox I/O: {error}"),
        }
    }
}

impl std::error::Error for SandboxError {}

impl From<io::Error> for SandboxError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// O que o operador concede ao agente; tudo o mais fica fora.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Socket do kernel (o kernel ainda autentica o UID do peer).
    pub kernel_socket: PathBuf,
    /// Diretório de staging do agente.
    pub staging: PathBuf,
    /// Código do agente (montado somente leitura em [`CODE`]).
    pub code: PathBuf,
    /// Socket do [`LlmProxy`], se o agente pode falar com o LLM.
    pub llm_socket: Option<PathBuf>,
    /// Programa e argumentos, resolvidos dentro do sandbox.
    pub program: Vec<OsString>,
}

fn absolute_existing(path: &Path, what: &'static str) -> Result<fs::Metadata, SandboxError> {
    if !path.is_absolute() {
        return Err(SandboxError::Invalid(what));
    }
    Ok(fs::symlink_metadata(path)?)
}

impl Plan {
    /// Confere o plano: caminhos absolutos, sem links simbólicos, do tipo certo.
    ///
    /// # Errors
    /// Caminho relativo, inexistente, link simbólico ou de tipo errado; programa vazio.
    pub fn validate(&self) -> Result<(), SandboxError> {
        if self.program.is_empty() {
            return Err(SandboxError::Invalid("empty program"));
        }
        let socket = absolute_existing(&self.kernel_socket, "kernel socket must be absolute")?;
        if !socket.file_type().is_socket() {
            return Err(SandboxError::Invalid("kernel socket is not a socket"));
        }
        if !absolute_existing(&self.staging, "staging must be absolute")?.is_dir() {
            return Err(SandboxError::Invalid("staging is not a directory"));
        }
        if !absolute_existing(&self.code, "code must be absolute")?.is_dir() {
            return Err(SandboxError::Invalid("code is not a directory"));
        }
        if let Some(llm) = &self.llm_socket
            && !absolute_existing(llm, "LLM socket must be absolute")?
                .file_type()
                .is_socket()
        {
            return Err(SandboxError::Invalid("LLM socket is not a socket"));
        }
        Ok(())
    }

    /// Argumentos completos do `bwrap`, na ordem em que são passados.
    ///
    /// # Errors
    /// Plano inválido (ver [`Plan::validate`]).
    pub fn arguments(&self) -> Result<Vec<OsString>, SandboxError> {
        self.validate()?;
        let mut args: Vec<OsString> = [
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
            "usr/sbin",
            "/sbin",
            "--symlink",
            "usr/lib",
            "/lib",
            "--symlink",
            "usr/lib64",
            "/lib64",
            // Só o necessário de /etc: certificados de CA (TLS fim a fim pelo proxy)
            // e cache do loader. Nada de passwd/hosts do operador além disso.
            "--ro-bind-try",
            "/etc/pki",
            "/etc/pki",
            "--ro-bind-try",
            "/etc/ssl",
            "/etc/ssl",
            "--ro-bind-try",
            "/etc/ld.so.cache",
            "/etc/ld.so.cache",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--dir",
            RUN,
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        let mut bind = |flag: &str, from: &Path, to: &str| {
            args.extend([flag.into(), from.as_os_str().to_owned(), to.into()]);
        };
        bind("--bind", &self.kernel_socket, KERNEL_SOCKET);
        bind("--bind", &self.staging, STAGING);
        bind("--ro-bind", &self.code, CODE);
        if let Some(llm) = &self.llm_socket {
            bind("--bind", llm, LLM_SOCKET);
        }
        let mut env = vec![
            ("PATH", "/usr/bin"),
            ("HOME", "/tmp"),
            ("LANG", "C.UTF-8"),
            ("LOGOS_KERNEL_SOCKET", KERNEL_SOCKET),
            ("LOGOS_STAGING", STAGING),
        ];
        if self.llm_socket.is_some() {
            env.push(("LOGOS_LLM_SOCKET", LLM_SOCKET));
        }
        for (key, value) in env {
            args.extend(["--setenv".into(), key.into(), value.into()]);
        }
        args.extend(["--chdir".into(), "/tmp".into(), "--".into()]);
        args.extend(self.program.iter().cloned());
        Ok(args)
    }

    /// Comando pronto para executar o agente isolado.
    ///
    /// # Errors
    /// Plano inválido ou `bwrap` ausente: nunca há fallback sem isolamento.
    pub fn command(&self) -> Result<Command, SandboxError> {
        let args = self.arguments()?;
        if !Path::new(BWRAP).is_file() {
            return Err(SandboxError::Invalid("bwrap is required"));
        }
        let mut command = Command::new(BWRAP);
        command.args(args);
        Ok(command)
    }
}

/// Encaminhador socket Unix → TCP para um único endereço fixado pelo operador.
///
/// O agente não escolhe destino: cada conexão no socket vira uma conexão TCP para
/// `upstream` e os bytes passam sem interpretação (TLS fim a fim). Encerra quando
/// o processo termina.
#[derive(Debug)]
pub struct LlmProxy {
    path: PathBuf,
}

impl LlmProxy {
    /// Cria o socket `path` (modo 0600; o diretório deve ser privado) e atende em
    /// segundo plano.
    ///
    /// # Errors
    /// Diretório inexistente/permissivo, caminho ocupado ou falha de bind.
    pub fn start(path: &Path, upstream: SocketAddr) -> Result<Self, SandboxError> {
        let parent = path
            .parent()
            .ok_or(SandboxError::Invalid("proxy socket needs a directory"))?;
        let mode = fs::symlink_metadata(parent)?.permissions().mode();
        if mode & 0o077 != 0 {
            return Err(SandboxError::Invalid("proxy directory must be private"));
        }
        let listener = UnixListener::bind(path)?;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        std::thread::spawn(move || {
            for connection in listener.incoming() {
                let Ok(client) = connection else {
                    continue;
                };
                std::thread::spawn(move || {
                    let _ = forward(client, upstream);
                });
            }
        });
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Caminho do socket no host.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn forward(client: UnixStream, upstream: SocketAddr) -> io::Result<()> {
    let server = TcpStream::connect_timeout(&upstream, Duration::from_secs(10))?;
    let (mut client_read, mut server_write) = (client.try_clone()?, server.try_clone()?);
    let upload = std::thread::spawn(move || {
        let _ = io::copy(&mut client_read, &mut server_write);
        let _ = server_write.shutdown(Shutdown::Write);
    });
    let (mut server_read, mut client_write) = (server, client);
    let _ = io::copy(&mut server_read, &mut client_write);
    let _ = client_write.shutdown(Shutdown::Write);
    let _ = upload.join();
    Ok(())
}
