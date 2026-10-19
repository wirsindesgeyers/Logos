//! Captura ancorada de uploads; snapshot privado em memória antes do CAS.

use crate::{
    TransportError as Error,
    wire::{ObjectRef, valid_staging_name},
};
use logos_crypto::{Domain, Hash, hash_raw};
use logos_ledger::Cas;
use rustix::fs::{
    AtFlags, FileType, Mode, OFlags, RenameFlags, open, openat, renameat_with, statat, unlinkat,
};
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
                let cleanup = self.release_capture(captured.as_str(), name.as_str());
                if let Err(cleanup) = cleanup {
                    return Err(Error::Io(std::io::Error::other(format!(
                        "staging capture {captured}: {}; cleanup: {cleanup}",
                        match &result {
                            Ok(_) => "validated".to_owned(),
                            Err(error) => error.to_string(),
                        }
                    ))));
                }
                result
            }
        }
    }

    fn release_capture(&self, captured: &str, original: &str) -> Result<(), Error> {
        self.release_capture_with(captured, original, || {
            let mut nonce = [0; 16];
            File::open("/dev/urandom")?.read_exact(&mut nonce)?;
            Ok(format!("rejected-{:032x}", u128::from_be_bytes(nonce)))
        })
    }

    fn release_capture_with(
        &self,
        captured: &str,
        original: &str,
        mut rejected_name: impl FnMut() -> Result<String, Error>,
    ) -> Result<(), Error> {
        let meta = statat(&self.private, captured, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(std::io::Error::from)?;
        if FileType::from_raw_mode(meta.st_mode).is_dir() {
            // Diretórios podem conter dados arbitrários e descritores do agente.
            // Devolver sem percorrê-los e sem sobrescrever um upload novo.
            let mut name = original.to_owned();
            let mut restored = false;
            for _ in 0..8 {
                match renameat_with(
                    &self.private,
                    captured,
                    &self.source,
                    name.as_str(),
                    RenameFlags::NOREPLACE,
                ) {
                    Ok(()) => {
                        restored = true;
                        break;
                    }
                    Err(rustix::io::Errno::EXIST) => {
                        name = rejected_name()?;
                    }
                    Err(error) => return Err(std::io::Error::from(error).into()),
                }
            }
            if !restored {
                return Err(Error::Limit("staging return collisions"));
            }
            self.source.sync_all()?;
        } else {
            unlinkat(&self.private, captured, AtFlags::empty()).map_err(std::io::Error::from)?;
        }
        self.private.sync_all()?;
        Ok(())
    }
    fn charge(&self, quota: &mut u64, count: u64) -> Result<(), Error> {
        *quota = quota
            .checked_sub(count)
            .ok_or(Error::Limit("import quota"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn directory_return_preserves_a_new_upload_and_does_not_follow_child_links() {
        let root = std::env::temp_dir().join(format!(
            "logos-staging-return-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let source = root.join("source");
        let private = root.join("private");
        for path in [&source, &private] {
            fs::create_dir(path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let staging =
            Staging::open(&source, &private, rustix::process::geteuid().as_raw(), 100).unwrap();
        let captured = private.join("captured");
        fs::create_dir(&captured).unwrap();
        fs::write(captured.join("child"), b"rejected").unwrap();
        let outside = root.join("outside");
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, captured.join("link")).unwrap();
        let original = "a".repeat(64);
        fs::write(source.join(&original), b"new upload").unwrap();
        staging.release_capture("captured", &original).unwrap();
        assert_eq!(fs::read_dir(&private).unwrap().count(), 0);
        assert_eq!(fs::read(source.join(&original)).unwrap(), b"new upload");
        let returned = fs::read_dir(&source)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("rejected-")
            })
            .unwrap();
        assert_eq!(fs::read(returned.join("child")).unwrap(), b"rejected");
        assert!(
            fs::symlink_metadata(returned.join("link"))
                .unwrap()
                .is_symlink()
        );
        assert_eq!(fs::read(outside).unwrap(), b"outside");
        fs::create_dir(private.join("collisions")).unwrap();
        fs::write(source.join("forced-collision"), b"collision").unwrap();
        let mut attempts = 0;
        assert!(matches!(
            staging.release_capture_with("collisions", &original, || {
                attempts += 1;
                Ok("forced-collision".to_owned())
            }),
            Err(Error::Limit("staging return collisions"))
        ));
        assert_eq!(attempts, 8);
        assert!(
            matches!(staging.release_capture_with("collisions", &original, || Err(Error::Io(std::io::Error::other("fixture entropy failure")))), Err(Error::Io(e)) if e.to_string() == "fixture entropy failure")
        );
        assert!(private.join("collisions").is_dir());
        assert_eq!(fs::read(source.join(&original)).unwrap(), b"new upload");
        // Falha de devolução é explícita, nunca sucesso que esconda resíduos.
        fs::create_dir(private.join("second")).unwrap();
        fs::remove_dir_all(&source).unwrap();
        assert!(staging.release_capture("second", &original).is_err());
        assert!(private.join("second").is_dir());
        drop(staging);
        fs::remove_dir_all(root).unwrap();
    }
}
