//! `logos-agent-sandbox`: executa o agente isolado (Etapa 5.11, ADR0059).
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsString;
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::process::ExitCode;

use logos_sandbox::{LlmProxy, Plan, SandboxError};

const USAGE: &str = "usage: logos-agent-sandbox --kernel-socket PATH --staging DIR --code DIR \
                     [--llm HOST:PORT --proxy-dir DIR] -- PROGRAM [ARGS...]";

fn run() -> Result<ExitCode, SandboxError> {
    let mut args = std::env::args_os().skip(1);
    let (mut kernel, mut staging, mut code, mut llm, mut proxy_dir) =
        (None, None, None, None, None);
    let mut program: Vec<OsString> = Vec::new();
    while let Some(flag) = args.next() {
        let mut value = || args.next().ok_or(SandboxError::Invalid(USAGE));
        match flag.to_str() {
            Some("--kernel-socket") => kernel = Some(PathBuf::from(value()?)),
            Some("--staging") => staging = Some(PathBuf::from(value()?)),
            Some("--code") => code = Some(PathBuf::from(value()?)),
            Some("--llm") => llm = Some(value()?),
            Some("--proxy-dir") => proxy_dir = Some(PathBuf::from(value()?)),
            Some("--") => {
                program.extend(args.by_ref());
                break;
            }
            _ => return Err(SandboxError::Invalid(USAGE)),
        }
    }
    // O proxy vive neste processo: termina junto com o agente.
    let proxy = match (llm, proxy_dir) {
        (None, None) => None,
        (Some(upstream), Some(dir)) => {
            let upstream = upstream
                .to_str()
                .ok_or(SandboxError::Invalid("LLM address is not UTF-8"))?
                .to_socket_addrs()?
                .next()
                .ok_or(SandboxError::Invalid("LLM address did not resolve"))?;
            Some(LlmProxy::start(&dir.join("llm.sock"), upstream)?)
        }
        _ => return Err(SandboxError::Invalid("--llm and --proxy-dir go together")),
    };
    let plan = Plan {
        kernel_socket: kernel.ok_or(SandboxError::Invalid(USAGE))?,
        staging: staging.ok_or(SandboxError::Invalid(USAGE))?,
        code: code.ok_or(SandboxError::Invalid(USAGE))?,
        llm_socket: proxy.as_ref().map(|p| p.path().to_path_buf()),
        program,
    };
    let status = plan.command()?.status()?;
    Ok(match status.code() {
        Some(0) => ExitCode::SUCCESS,
        Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        None => ExitCode::FAILURE,
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("logos-agent-sandbox: {error}");
            ExitCode::from(2)
        }
    }
}
