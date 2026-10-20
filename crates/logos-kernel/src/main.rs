//! Processo Linux: PROPOSE, CERTIFY, REFUTE e leitura autenticada.
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(target_os = "linux")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use logos_crypto::{ExternalDigest, FileSigner, Signer};
    use logos_kernel::{
        certify::CertificationHandlers,
        registry::CheckerRegistry,
        server::{ConnectionLimits, Server},
        staging::Staging,
    };
    use logos_ledger::{EventBody, Ledger};
    use std::{io::Read, path::Path, sync::atomic::AtomicBool};
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(6..=7).contains(&args.len()) {
        return Err(
            "usage: logos-kernel SOCKET LEDGER KEYFILE STAGING PRIVATE AGENT_UID [CAKE_BINARY]"
                .into(),
        );
    }
    let uid: u32 = args[5].to_str().ok_or("UID is not UTF-8")?.parse()?;
    let signer = FileSigner::load(Path::new(&args[2]))?;
    let mut ledger = Ledger::open(Path::new(&args[1]), &signer)?;
    if ledger.head()?.is_none() {
        let mut bytes = Vec::new();
        std::fs::File::open("/proc/self/exe")?
            .take(128 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 128 * 1024 * 1024 {
            return Err("kernel executable exceeds digest limit".into());
        }
        ledger.append(EventBody::Genesis {
            ledger_version: 1,
            kernel_binary: *ExternalDigest::of(&bytes).as_bytes(),
            kernel_key: signer.key_id()?,
        })?;
    }
    let registry = CheckerRegistry::new(signer.public_key().clone(), args.get(6).map(Path::new))?;
    registry.install(&mut ledger, &signer)?;
    let handlers = CertificationHandlers::new(
        registry,
        logos_core::Budget::new(20_000_000, 128 * 1024 * 1024, 64 * 1024 * 1024, 5000),
    );
    let staging = Staging::open(
        Path::new(&args[3]),
        Path::new(&args[4]),
        uid,
        64 * 1024 * 1024,
    )?;
    let server = Server::bind(Path::new(&args[0]), uid, ConnectionLimits::default())?;
    server.serve(&mut ledger, &handlers, &staging, &AtomicBool::new(false))?;
    Ok(())
}

fn main() -> std::process::ExitCode {
    #[cfg(target_os = "linux")]
    let result = run();
    #[cfg(not(target_os = "linux"))]
    let result: Result<(), Box<dyn std::error::Error>> =
        Err("kernel transport requires Linux SO_PEERCRED".into());
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("logos-kernel: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
