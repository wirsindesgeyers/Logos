//! Captura ancorada de uploads; snapshot privado em memória antes do CAS.

use crate::{
    TransportError as Error,
    wire::{ObjectRef, valid_staging_name},
};
use logos_crypto::{Domain, Hash, hash_raw};
use logos_ledger::Cas;
use rustix::fs::{AtFlags, Mode, OFlags, RenameFlags, open, openat, renameat_with, unlinkat};
use std::fs::File;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Diretórios fixados por descritor; nenhuma resolução de caminho do frame.
pub struct Staging {
    source: File,
    private: File,
    uid: u32,
    max_bytes: u64,
}

fn directory(path: &Path) -> Result<File, Error> {
    let fd = open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(std::io::Error::from)?;
    Ok(File::from(fd))
}

impl Staging {
    /// Abre diretório do agente e área0700 do kernel, sem seguir symlinks.
    /// # Errors
    /// UID/mode/tipo inválido ou I/O. Pais precisam ser provisionados pelo operador.
    pub fn open(
        source: &Path,
        private: &Path,
        agent_uid: u32,
        max_bytes: u64,
    ) -> Result<Self, Error> {
        if max_bytes == 0 || max_bytes > 64 * 1024 * 1024 {
            return Err(Error::Limit("staging object limit"));
        }
        let source = directory(source)?;
        let private = directory(private)?;
        let sm = source.metadata()?;
        let pm = private.metadata()?;
        if sm.uid() != agent_uid
            || sm.mode() & 0o077 != 0
            || pm.uid() != rustix::process::geteuid().as_raw()
            || pm.mode() & 0o077 != 0
        {
            return Err(Error::Invalid("staging directory owner/mode"));
        }
        if sm.dev() != pm.dev() || (sm.dev() == pm.dev() && sm.ino() == pm.ino()) {
            return Err(Error::Invalid(
                "staging requires distinct directories on one filesystem",
            ));
        }
        Ok(Self {
            source,
            private,
            uid: agent_uid,
            max_bytes,
        })
    }

    /// Confere referência/CAS ou captura upload e publica somente snapshot de hash exato.
    /// A quota é consumida inclusive quando o hash final diverge.
    /// # Errors
    /// Referência ausente, arquivo/UID/links inválidos, limite, hash ou I/O/CAS.
    pub fn import<T: Domain>(
        &self,
        reference: &ObjectRef<T>,
        cas: &Cas,
        quota: &mut u64,
    ) -> Result<Hash<T>, Error> {
        match reference {
            ObjectRef::Cas(hash) => {
                let bytes = cas.get(hash)?.ok_or(Error::Invalid("missing CAS object"))?;
                self.charge(quota, bytes.len() as u64)?;
                Ok(*hash)
            }
            ObjectRef::Staged { name, hash } => {
                if !valid_staging_name(name.as_str()) {
                    return Err(Error::Invalid("staging name"));
                }
                let captured = format!(
                    "upload-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                );
                renameat_with(
                    &self.source,
                    name.as_str(),
                    &self.private,
                    captured.as_str(),
                    RenameFlags::NOREPLACE,
                )
                .map_err(std::io::Error::from)?;
                let result = (|| {
                    let fd = openat(
                        &self.private,
                        captured.as_str(),
                        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map_err(std::io::Error::from)?;
                    let mut file = File::from(fd);
                    let meta = file.metadata()?;
                    if !meta.is_file()
                        || meta.uid() != self.uid
                        || meta.nlink() != 1
                        || meta.mode() & 0o022 != 0
                    {
                        return Err(Error::Invalid("staged object type/owner/links/mode"));
                    }
                    if meta.len() > self.max_bytes || meta.len() > *quota {
                        return Err(Error::Limit("staged object size/quota"));
                    }
                    let mut bytes = Vec::new();
                    let mut chunk = [0u8; 65536];
                    loop {
                        let count = match file.read(&mut chunk) {
                            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                            other => other?,
                        };
                        if count == 0 {
                            break;
                        }
                        self.charge(quota, count as u64)?;
                        if bytes
                            .len()
                            .checked_add(count)
                            .is_none_or(|n| n as u64 > self.max_bytes)
                        {
                            return Err(Error::Limit("growing staged object"));
                        }
                        bytes
                            .try_reserve(count)
                            .map_err(|_| Error::Limit("staging memory"))?;
                        bytes.extend_from_slice(&chunk[..count]);
                    }
                    if hash_raw::<T>(&bytes) != *hash {
                        return Err(Error::Invalid("staged object digest"));
                    }
                    let published = cas.put::<T>(&bytes)?;
                    if published != *hash {
                        return Err(Error::Invalid("CAS snapshot digest"));
                    }
                    Ok(published)
                })();
                // Só remove o nome capturado no diretório privado; nunca reabre o
                // nome do agente, que pode já apontar para outro upload.
                unlinkat(&self.private, captured.as_str(), AtFlags::empty())
                    .map_err(std::io::Error::from)?;
                self.private.sync_all()?;
                result
            }
        }
    }
    fn charge(&self, quota: &mut u64, count: u64) -> Result<(), Error> {
        *quota = quota
            .checked_sub(count)
            .ok_or(Error::Limit("import quota"))?;
        Ok(())
    }
}
