//! `cargo xtask tcb [check|report|diff|strict-crates] ...` delega para `tcb/count.sh`.
//!
//! Sem dependências de propósito: a lógica fica em `tcb/tcb.py` (só biblioteca padrão do
//! Python + `tokei`), para não aumentar a árvore de dependências do workspace.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("tcb") => {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
            let script = root.join("tcb").join("count.sh");
            match Command::new(&script).args(args).current_dir(&root).status() {
                Ok(status) if status.success() => ExitCode::SUCCESS,
                Ok(_) => ExitCode::FAILURE,
                Err(e) => {
                    eprintln!("erro: não foi possível executar {}: {e}", script.display());
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("uso: cargo xtask tcb [check|report|diff|strict-crates] ...");
            ExitCode::FAILURE
        }
    }
}
