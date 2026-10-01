//! Assinantes: a trait [`Signer`] e a implementação em arquivo [`FileSigner`].
//!
//! A chave privada só existe em quem assina (kernel, gateway, atestadores, usuário). O HSM/TEE e
//! a rotação de chaves (Etapa 13.1) são outras implementações da mesma trait.

use crate::error::CryptoError;
use crate::sig::{KeyId, PublicKey, SigAlg, SigDomain, Signature, tagged_message};
use ed25519_dalek::{Signer as _, SigningKey};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

/// Quem sabe assinar em nome de uma chave pública.
pub trait Signer {
    /// A chave pública correspondente.
    fn public_key(&self) -> &PublicKey;

    /// Assina `D::TAG ‖ bytes`.
    ///
    /// # Errors
    /// Falha do backend de assinatura.
    fn sign<D: SigDomain>(&self, bytes: &[u8]) -> Result<Signature, CryptoError>;

    /// Identidade da chave.
    ///
    /// # Errors
    /// Ver [`PublicKey::key_id`].
    fn key_id(&self) -> Result<KeyId, CryptoError> {
        self.public_key().key_id()
    }
}

/// Assinante Ed25519 cuja chave vem de um arquivo (desenvolvimento e papéis simples).
///
/// Formato do arquivo: a semente de 32 bytes em 64 dígitos hexadecimais minúsculos, com um `\n`
/// final opcional, e nada mais. O arquivo precisa ser regular (aberto com `O_NOFOLLOW`: link
/// simbólico no último componente é recusado) e não ter nenhuma permissão para grupo/outros
/// (`mode & 0o077 == 0`). A chave fica em memória do
/// processo; mantê-la fora dele é o objetivo da Etapa 13.1.
pub struct FileSigner {
    key: SigningKey,
    public: PublicKey,
}

impl core::fmt::Debug for FileSigner {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Nunca imprime a chave privada.
        f.debug_struct("FileSigner")
            .field("public", &self.public)
            .finish_non_exhaustive()
    }
}

impl FileSigner {
    /// Cria o assinante a partir da semente de 32 bytes (RFC 8032: “secret key”).
    ///
    /// # Errors
    /// Só se a chave pública derivada for recusada (não ocorre para sementes válidas).
    pub fn from_seed(seed: &[u8; 32]) -> Result<Self, CryptoError> {
        let key = SigningKey::from_bytes(seed);
        let public = PublicKey::new(SigAlg::Ed25519, key.verifying_key().as_bytes())?;
        Ok(Self { key, public })
    }

    /// Carrega a chave de um arquivo, recusando permissões abertas demais.
    ///
    /// # Errors
    /// [`CryptoError::InsecurePermissions`], [`CryptoError::KeyFileNotRegular`],
    /// [`CryptoError::KeyFileFormat`] ou [`CryptoError::Io`].
    pub fn load(path: &Path) -> Result<Self, CryptoError> {
        let io = |e: std::io::Error| CryptoError::Io(e.kind());
        // Abre sem seguir link simbólico no último componente (O_NOFOLLOW) e sem bloquear se o
        // caminho virou uma FIFO (O_NONBLOCK); só depois confere o arquivo *aberto* (fstat). Não há
        // checagem prévia do caminho: qualquer uma deixaria uma janela entre checar e abrir.
        let file = open_key_file(path)?;
        let meta = file.metadata().map_err(io)?;
        if !meta.is_file() {
            return Err(CryptoError::KeyFileNotRegular);
        }
        let mode = permission_bits(&meta)?;
        if mode & 0o077 != 0 {
            return Err(CryptoError::InsecurePermissions(mode & 0o777));
        }
        let mut buf = Vec::with_capacity(66);
        file.take(66).read_to_end(&mut buf).map_err(io)?;
        Self::from_key_file_bytes(&buf)
    }

    /// Interpreta o *conteúdo* de um arquivo de chave (formato descrito em [`FileSigner`]).
    /// É o parser usado por [`FileSigner::load`]; fica público para o alvo de fuzz.
    ///
    /// # Errors
    /// [`CryptoError::KeyFileFormat`] se o conteúdo não segue o formato.
    pub fn from_key_file_bytes(content: &[u8]) -> Result<Self, CryptoError> {
        Self::from_seed(&parse_seed(content)?)
    }
}

/// `O_NOFOLLOW | O_NONBLOCK` da plataforma. Valores fixos (sem a crate `libc`, que somaria ~113 k
/// LOC à TCB só por constantes); os testes de symlink e FIFO de `tests/security.rs` quebram se um
/// valor estiver errado. Plataforma desconhecida: recusa (falha fechada).
#[cfg(all(target_os = "linux", any(target_arch = "x86_64", target_arch = "x86")))]
const OPEN_FLAGS: Option<i32> = Some(0o400000 | 0o4000);
/// aarch64/arm usam `O_NOFOLLOW = 0o100000`; **a confirmar no runner aarch64 (Etapa 0.6)**: os
/// testes de symlink/FIFO fazem essa confirmação.
#[cfg(all(target_os = "linux", any(target_arch = "aarch64", target_arch = "arm")))]
const OPEN_FLAGS: Option<i32> = Some(0o100000 | 0o4000);
#[cfg(target_os = "macos")]
const OPEN_FLAGS: Option<i32> = Some(0x0100 | 0x0004);
#[cfg(not(any(
    all(
        target_os = "linux",
        any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm"
        )
    ),
    target_os = "macos"
)))]
const OPEN_FLAGS: Option<i32> = None;

#[cfg(unix)]
fn open_key_file(path: &Path) -> Result<File, CryptoError> {
    use std::os::unix::fs::OpenOptionsExt;
    let Some(flags) = OPEN_FLAGS else {
        return Err(CryptoError::Io(std::io::ErrorKind::Unsupported));
    };
    match fs::OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)
    {
        Ok(file) => Ok(file),
        // Só para dar o erro certo: o último componente é um link simbólico?
        Err(_) if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) => {
            Err(CryptoError::KeyFileNotRegular)
        }
        Err(e) => Err(CryptoError::Io(e.kind())),
    }
}

#[cfg(not(unix))]
fn open_key_file(_: &Path) -> Result<File, CryptoError> {
    Err(CryptoError::Io(std::io::ErrorKind::Unsupported))
}

#[cfg(unix)]
fn permission_bits(meta: &fs::Metadata) -> Result<u32, CryptoError> {
    use std::os::unix::fs::PermissionsExt;
    Ok(meta.permissions().mode())
}

#[cfg(not(unix))]
fn permission_bits(_: &fs::Metadata) -> Result<u32, CryptoError> {
    // Sem modo POSIX não há como garantir 0600: recusa em vez de aceitar sem checar.
    Err(CryptoError::InsecurePermissions(0o777))
}

/// 64 hex minúsculos, `\n` final opcional, nada mais.
fn parse_seed(content: &[u8]) -> Result<[u8; 32], CryptoError> {
    let hex = match content.len() {
        64 => content,
        65 if content.last() == Some(&b'\n') => &content[..64],
        _ => return Err(CryptoError::KeyFileFormat),
    };
    let mut seed = [0u8; 32];
    let (pairs, _) = hex.as_chunks::<2>();
    for (byte, [hi, lo]) in seed.iter_mut().zip(pairs) {
        *byte = (nibble(*hi)? << 4) | nibble(*lo)?;
    }
    Ok(seed)
}

fn nibble(c: u8) -> Result<u8, CryptoError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        _ => Err(CryptoError::KeyFileFormat),
    }
}

impl Signer for FileSigner {
    fn public_key(&self) -> &PublicKey {
        &self.public
    }

    fn sign<D: SigDomain>(&self, bytes: &[u8]) -> Result<Signature, CryptoError> {
        let signature = self.key.sign(&tagged_message(D::TAG, bytes));
        Signature::new(SigAlg::Ed25519, &signature.to_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_parser_is_strict() {
        let good = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        assert!(parse_seed(good.as_bytes()).is_ok());
        assert!(parse_seed(format!("{good}\n").as_bytes()).is_ok());
        for bad in [
            String::new(),
            good[..63].to_string(),
            format!("{good}\n\n"),
            format!("{good} "),
            format!("{good}\r\n"),
            good.to_uppercase(),
            format!("g{}", &good[1..]),
        ] {
            assert_eq!(
                parse_seed(bad.as_bytes()),
                Err(CryptoError::KeyFileFormat),
                "{bad:?}"
            );
        }
    }
}
