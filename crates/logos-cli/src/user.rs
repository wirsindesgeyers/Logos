//! Chave do usuário (6.5): par Ed25519 local, criado fora do agente.
//!
//! O arquivo segue o formato do kernel (64 dígitos hex, modo 0600). Quem assina pelo
//! usuário é um processo do usuário, nunca o agente: o sandbox (5.11) não monta o home.

use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

use logos_crypto::{FileSigner, Signer};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Cria `path` (que não pode existir) com uma semente aleatória do SO e modo 0600.
pub fn init(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut seed = [0u8; 32];
    fs::File::open("/dev/urandom")?.read_exact(&mut seed)?;
    // `create_new` (O_EXCL) não segue link simbólico nem sobrescreve chave existente.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(hex(&seed).as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);
    // Releitura pelo mesmo caminho do kernel: confere formato e permissões gravados.
    let signer = FileSigner::load(path)?;
    println!(
        "{{\"public_key\":\"{}\",\"key_id\":\"{}\"}}",
        hex(signer.public_key().as_bytes()),
        hex(signer.key_id()?.as_bytes())
    );
    Ok(())
}
