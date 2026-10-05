//! CAS `cas/v1`: bytes opacos, endereços tipados e publicação atômica durável.
//!
//! Contrato normativo: `spec/cas.md`, ADR 0005. Filesystem local Unix e ancestrais
//! confiáveis; não é sandbox. Erro após rename não promete rollback.

use crate::cas_domain::{DOMAINS, resolve};
use logos_crypto::{Domain, Hash, RawHasher, hash_raw};
use std::{
    error::Error,
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Falha de armazenamento, distinta de um veredicto de certificado.
#[derive(Debug)]
pub enum CasError {
    /// Bytes presentes não correspondem ao endereço pedido.
    Integrity,
    /// Tamanho conhecido ou observado excedeu limite do domínio.
    TooLarge {
        /// Limite configurado, em bytes.
        limit: u64,
        /// Tamanho total conhecido ou bytes observados até a recusa.
        actual: u64,
    },
    /// I/O, tag desconhecida, limite irrepresentável ou reserva falha.
    Io(io::Error),
}
impl fmt::Display for CasError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integrity => f.write_str("CAS integrity mismatch"),
            Self::TooLarge { limit, actual } => {
                write!(f, "CAS size {actual} exceeds limit {limit}")
            }
            Self::Io(e) => write!(f, "CAS I/O: {e}"),
        }
    }
}
impl Error for CasError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}
impl From<io::Error> for CasError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Limites independentes por tag reservada, em bytes.
#[derive(Clone, Debug)]
pub struct Limits {
    domains: [u64; DOMAINS.len()],
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            domains: DOMAINS.map(|entry| entry.limit),
        }
    }
}
impl Limits {
    /// Substitui limite do domínio; zero aceita apenas objeto vazio.
    ///
    /// # Errors
    /// `Io(InvalidInput)` para tag desconhecida ou limite acima de `isize::MAX`.
    pub fn with_domain_limit<T: Domain>(mut self, limit: u64) -> Result<Self, CasError> {
        let index = domain::<T>()?;
        if limit > isize::MAX as u64 {
            return Err(
                io::Error::new(io::ErrorKind::InvalidInput, "limit exceeds isize::MAX").into(),
            );
        }
        self.domains[index] = limit;
        Ok(self)
    }
}
fn domain<T: Domain>() -> Result<usize, CasError> {
    resolve(T::TAG)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "unknown CAS domain").into())
}

#[derive(Debug)]
struct CleanupError {
    primary: io::Error,
    cleanup: io::Error,
}
impl fmt::Display for CleanupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; cleanup failed: {}", self.primary, self.cleanup)
    }
}
impl Error for CleanupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.primary)
    }
}

/// Handle independente; pode compartilhar raiz com outros handles/writers.
pub struct Cas {
    root: PathBuf,
    limits: Limits,
    #[cfg(test)]
    hook: Option<tests::Hook>,
}
static NEXT: AtomicU64 = AtomicU64::new(0);

impl Cas {
    /// Cria/abre raiz CAS e sincroniza ancestrais até raiz do filesystem.
    ///
    /// # Errors
    /// Qualquer erro de criação, resolução ou fsync retorna `Io`.
    pub fn open(root: impl AsRef<Path>, limits: Limits) -> Result<Self, CasError> {
        Self::open_inner(
            root.as_ref(),
            limits,
            #[cfg(test)]
            None,
        )
    }
    fn open_inner(
        root: &Path,
        limits: Limits,
        #[cfg(test)] hook: Option<tests::Hook>,
    ) -> Result<Self, CasError> {
        fs::create_dir_all(root)?;
        let root = root.canonicalize()?;
        let cas = Self {
            root,
            limits,
            #[cfg(test)]
            hook,
        };
        for ancestor in cas.root.ancestors() {
            cas.sync_directory(ancestor)?;
        }
        Ok(cas)
    }
    fn sync_directory(&self, path: &Path) -> io::Result<()> {
        #[cfg(test)]
        self.event(tests::Event::SyncDirectory(path))?;
        File::open(path)?.sync_all()
    }
    #[cfg(test)]
    fn event(&self, event: tests::Event<'_>) -> io::Result<()> {
        match &self.hook {
            Some(hook) => {
                let mut callback = hook
                    .lock()
                    .map_err(|_| io::Error::other("CAS test hook poisoned"))?;
                callback(event)
            }
            None => Ok(()),
        }
    }

    /// Publica bytes sem copiá-los para buffer intermediário.
    ///
    /// # Errors
    /// Tag desconhecida, excesso, escrita ou sincronização falha.
    pub fn put<T: Domain>(&self, bytes: &[u8]) -> Result<Hash<T>, CasError> {
        let index = domain::<T>()?;
        let limit = self.limits.domains[index];
        let actual = bytes.len() as u64;
        if actual > limit {
            return Err(CasError::TooLarge { limit, actual });
        }
        let hash = hash_raw::<T>(bytes);
        self.publish(index, bytes, &hash)?;
        Ok(hash)
    }

    /// Lê fluxo limitado em memória e publica somente após EOF aceito.
    ///
    /// # Errors
    /// Tag desconhecida, excesso, leitura, reserva, escrita ou fsync falha.
    pub fn put_reader<T: Domain>(&self, mut reader: impl Read) -> Result<Hash<T>, CasError> {
        let index = domain::<T>()?;
        let (bytes, hash) = self.read_bounded::<T>(&mut reader, self.limits.domains[index])?;
        self.publish(index, &bytes, &hash)?;
        Ok(hash)
    }

    /// Lê e verifica hash antes de entregar bytes; ausência só na abertura.
    ///
    /// # Errors
    /// Excesso precede `Integrity`; demais falhas de leitura/reserva retornam `Io`.
    pub fn get<T: Domain>(&self, hash: &Hash<T>) -> Result<Option<Vec<u8>>, CasError> {
        let index = domain::<T>()?;
        let path = self.object_path(index, hash)?;
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let (bytes, actual) = self.read_bounded::<T>(&mut file, self.limits.domains[index])?;
        if actual != *hash {
            return Err(CasError::Integrity);
        }
        Ok(Some(bytes))
    }

    fn read_bounded<T: Domain>(
        &self,
        reader: &mut impl Read,
        limit: u64,
    ) -> Result<(Vec<u8>, Hash<T>), CasError> {
        // Limits garante representabilidade; Read garante n <= comprimento solicitado.
        let maximum = limit as usize;
        let mut bytes = Vec::new();
        let mut chunk = [0; 8192];
        let mut hasher = RawHasher::<T>::new();
        while bytes.len() < maximum {
            let available = chunk.len().min(maximum - bytes.len());
            let n = read_retry(reader, &mut chunk[..available])?;
            if n == 0 {
                return Ok((bytes, hasher.finalize()));
            }
            let required = bytes.len() + n; // <= maximum <= isize::MAX
            if required > bytes.capacity() {
                let target = bytes
                    .capacity()
                    .saturating_mul(2)
                    .max(required)
                    .min(maximum);
                #[cfg(test)]
                self.event(tests::Event::Reserve)?;
                bytes
                    .try_reserve_exact(target - bytes.len())
                    .map_err(|e| io::Error::new(io::ErrorKind::OutOfMemory, e))?;
            }
            bytes.extend_from_slice(&chunk[..n]);
            hasher.update(&chunk[..n]);
        }
        // Probe externo: nunca entra no Vec, nunca reserva, nunca alimenta hash.
        if read_retry(reader, &mut [0; 1])? != 0 {
            return Err(CasError::TooLarge {
                limit,
                actual: limit + 1,
            });
        }
        Ok((bytes, hasher.finalize()))
    }

    fn object_path<T: Domain>(&self, index: usize, hash: &Hash<T>) -> io::Result<PathBuf> {
        // Hex em stack; evita uma alocação/format por byte de Hash::to_hex.
        let mut hex = [0u8; 64];
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        for (byte, pair) in hash.as_bytes().iter().zip(hex.as_chunks_mut::<2>().0) {
            pair[0] = DIGITS[usize::from(byte >> 4)];
            pair[1] = DIGITS[usize::from(byte & 15)];
        }
        let hex =
            std::str::from_utf8(&hex).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut path = self.root.join(DOMAINS[index].directory);
        path.push(&hex[..2]);
        path.push(&hex[2..]);
        Ok(path)
    }

    fn publish<T: Domain>(
        &self,
        index: usize,
        bytes: &[u8],
        hash: &Hash<T>,
    ) -> Result<(), CasError> {
        let destination = self.object_path(index, hash)?;
        let shard = destination
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing shard"))?;
        let directory = shard.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "missing domain directory")
        })?;
        fs::create_dir_all(shard)?;
        #[cfg(test)]
        self.event(tests::Event::DirectoriesCreated)?;
        let (mut file, temporary) = loop {
            let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
            let temporary = shard.join(format!(".tmp.{}.{}", std::process::id(), sequence));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(file) => break (file, temporary),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        };
        let result: io::Result<()> = (|| {
            // Pausa somente em testes, depois de prefixo escrito no arquivo real.
            #[cfg(test)]
            {
                let split = bytes.len() / 2;
                file.write_all(&bytes[..split])?;
                self.event(tests::Event::PrefixWritten)?;
                file.write_all(&bytes[split..])?;
            }
            #[cfg(not(test))]
            file.write_all(bytes)?;
            #[cfg(test)]
            self.event(tests::Event::SyncFile)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, &destination)?;
            self.sync_directory(shard)?;
            self.sync_directory(directory)?;
            self.sync_directory(&self.root)?;
            Ok(())
        })();
        if let Err(primary) = result {
            #[cfg(test)]
            let cleanup = self
                .event(tests::Event::Cleanup)
                .and_then(|()| fs::remove_file(&temporary));
            #[cfg(not(test))]
            let cleanup = fs::remove_file(&temporary);
            return Err(match cleanup {
                Ok(()) => primary,
                Err(e) if e.kind() == io::ErrorKind::NotFound => primary,
                Err(cleanup) => io::Error::new(primary.kind(), CleanupError { primary, cleanup }),
            }
            .into());
        }
        Ok(())
    }
}
fn read_retry(reader: &mut impl Read, bytes: &mut [u8]) -> io::Result<usize> {
    loop {
        match reader.read(bytes) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use logos_crypto::tags;
    use std::sync::{Arc, Mutex, mpsc::sync_channel};

    pub(super) enum Event<'a> {
        SyncDirectory(&'a Path),
        DirectoriesCreated,
        PrefixWritten,
        SyncFile,
        Reserve,
        Cleanup,
    }
    pub(super) type Hook = Arc<Mutex<dyn FnMut(Event<'_>) -> io::Result<()> + Send>>;
    struct Certificate;
    impl Domain for Certificate {
        const TAG: &'static [u8] = tags::CERTIFICATE;
    }
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            loop {
                let path = std::env::temp_dir().join(format!(
                    "logos-cas-unit-{}-{}",
                    std::process::id(),
                    NEXT.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(e) => panic!("{e}"),
                }
            }
        }
        fn hooked(
            &self,
            callback: impl FnMut(Event<'_>) -> io::Result<()> + Send + 'static,
        ) -> Cas {
            let cas = Cas::open(&self.0, Limits::default()).unwrap();
            Cas {
                hook: Some(Arc::new(Mutex::new(callback))),
                ..cas
            }
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    #[derive(Debug)]
    struct PrimaryFailure;
    impl fmt::Display for PrimaryFailure {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("injected primary sync failure")
        }
    }
    impl Error for PrimaryFailure {}
    fn failure() -> io::Error {
        io::Error::other(PrimaryFailure)
    }
    fn assert_io<T>(result: Result<T, CasError>, kind: io::ErrorKind) {
        assert!(matches!(result, Err(CasError::Io(e)) if e.kind() == kind));
    }

    #[test]
    fn allocation_failure_is_io() {
        let temp = Temp::new();
        let healthy = Cas::open(&temp.0, Limits::default()).unwrap();
        let hash = healthy.put::<Certificate>(b"valid bytes").unwrap();
        let cas = temp.hooked(|event| match event {
            Event::Reserve => Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "injected allocation failure",
            )),
            _ => Ok(()),
        });
        assert_io(
            cas.put_reader::<Certificate>(&b"new bytes"[..]),
            io::ErrorKind::OutOfMemory,
        );
        assert!(
            healthy
                .get(&hash_raw::<Certificate>(b"new bytes"))
                .unwrap()
                .is_none()
        );
        assert_io(cas.get(&hash), io::ErrorKind::OutOfMemory);
        // Probe de excesso e EOF no limite zero não devem tentar reservar.
        let zero = Cas {
            limits: Limits::default()
                .with_domain_limit::<Certificate>(0)
                .unwrap(),
            ..cas
        };
        assert!(matches!(
            zero.put_reader::<Certificate>(&b"x"[..]),
            Err(CasError::TooLarge {
                limit: 0,
                actual: 1
            })
        ));
        assert_eq!(
            zero.put_reader::<Certificate>(&b""[..]).unwrap(),
            hash_raw::<Certificate>(b"")
        );
    }

    #[test]
    fn every_open_ancestor_sync_failure_is_public_io() {
        let temp = Temp::new();
        let ancestors = temp
            .0
            .canonicalize()
            .unwrap()
            .ancestors()
            .map(Path::to_path_buf)
            .collect::<Vec<_>>();
        for failing in ancestors {
            let hook: Hook = Arc::new(Mutex::new(move |event: Event<'_>| match event {
                Event::SyncDirectory(path) if path == failing => Err(failure()),
                _ => Ok(()),
            }));
            assert_io(
                Cas::open_inner(&temp.0, Limits::default(), Some(hook)),
                io::ErrorKind::Other,
            );
        }
    }

    #[test]
    fn every_publication_sync_failure_is_public_io() {
        for fail_at in 0..4 {
            let temp = Temp::new();
            let mut calls = 0;
            let cas = temp.hooked(move |event| {
                if matches!(event, Event::SyncFile | Event::SyncDirectory(_)) {
                    let current = calls;
                    calls += 1;
                    if current == fail_at {
                        return Err(failure());
                    }
                }
                Ok(())
            });
            let bytes = b"object";
            assert_io(cas.put::<Certificate>(bytes), io::ErrorKind::Other);
            let reader = Cas::open(&temp.0, Limits::default()).unwrap();
            let observed = reader.get(&hash_raw::<Certificate>(bytes)).unwrap();
            assert_eq!(
                observed.as_deref(),
                if fail_at == 0 { None } else { Some(&bytes[..]) }
            );
        }
    }

    #[test]
    fn read_during_publication_never_sees_partial_object() {
        for replacement in [false, true] {
            let temp = Temp::new();
            let reader = Cas::open(&temp.0, Limits::default()).unwrap();
            let bytes = b"0123456789abcdefghijklmnop";
            let hash = hash_raw::<Certificate>(bytes);
            if replacement {
                reader.put::<Certificate>(bytes).unwrap();
            }
            let (paused_tx, paused_rx) = sync_channel(0);
            let (resume_tx, resume_rx) = sync_channel(0);
            let writer = temp.hooked(move |event| {
                if matches!(event, Event::PrefixWritten) {
                    paused_tx.send(()).unwrap();
                    resume_rx.recv().unwrap();
                }
                Ok(())
            });
            let thread = std::thread::spawn(move || writer.put::<Certificate>(bytes));
            paused_rx.recv().unwrap();
            let during = reader.get(&hash);
            resume_tx.send(()).unwrap();
            let published = thread.join().unwrap().unwrap();
            assert_eq!(published, hash);
            assert_eq!(
                during.unwrap().as_deref(),
                if replacement { Some(&bytes[..]) } else { None }
            );
            assert_eq!(reader.get(&hash).unwrap().as_deref(), Some(&bytes[..]));
        }
    }

    #[test]
    fn concurrent_directory_creation_is_durable() {
        // A cria exatamente o shard usado por B, pausa antes de qualquer fsync, depois falha.
        for failing_parent in 0..3 {
            let temp = Temp::new();
            let bytes = b"same object and same shard";
            let (paused_tx, paused_rx) = sync_channel(0);
            let (resume_tx, resume_rx) = sync_channel(0);
            let writer_a = temp.hooked(move |event| {
                if matches!(event, Event::DirectoriesCreated) {
                    paused_tx.send(()).unwrap();
                    resume_rx.recv().unwrap();
                    return Err(failure());
                }
                Ok(())
            });
            let thread = std::thread::spawn(move || writer_a.put::<Certificate>(bytes));
            paused_rx.recv().unwrap();
            let mut directory_calls = 0;
            let writer_b = temp.hooked(move |event| {
                if matches!(event, Event::SyncDirectory(_)) {
                    let current = directory_calls;
                    directory_calls += 1;
                    if current == failing_parent {
                        return Err(failure());
                    }
                }
                Ok(())
            });
            let failed = writer_b.put::<Certificate>(bytes);
            // Outro handle B conclui sozinho, antes de A retomar.
            let healthy = Cas::open(&temp.0, Limits::default()).unwrap();
            let published = healthy.put::<Certificate>(bytes);
            resume_tx.send(()).unwrap();
            assert_io(thread.join().unwrap(), io::ErrorKind::Other);
            assert_io(failed, io::ErrorKind::Other);
            assert_eq!(
                healthy.get(&published.unwrap()).unwrap().as_deref(),
                Some(&bytes[..])
            );
        }
    }

    #[test]
    fn cleanup_preserves_primary_and_secondary_errors() {
        for cleanup_failure in [false, true] {
            let temp = Temp::new();
            let cas = temp.hooked(move |event| match event {
                Event::SyncFile => Err(failure()),
                Event::Cleanup if cleanup_failure => Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "injected cleanup failure",
                )),
                _ => Ok(()),
            });
            let Err(CasError::Io(error)) = cas.put::<Certificate>(b"cleanup") else {
                panic!("expected Io");
            };
            assert_eq!(error.kind(), io::ErrorKind::Other);
            if cleanup_failure {
                let composite = error
                    .get_ref()
                    .unwrap()
                    .downcast_ref::<CleanupError>()
                    .unwrap();
                assert_eq!(composite.primary.kind(), io::ErrorKind::Other);
                assert_eq!(composite.cleanup.kind(), io::ErrorKind::PermissionDenied);
                assert!(composite.primary.get_ref().unwrap().is::<PrimaryFailure>());
                let diagnostic = error.to_string();
                assert!(diagnostic.contains(&composite.primary.to_string()));
                assert!(diagnostic.contains(&composite.cleanup.to_string()));
            } else {
                assert!(error.get_ref().unwrap().is::<PrimaryFailure>());
                let hash = hash_raw::<Certificate>(b"cleanup");
                let path = cas
                    .object_path(domain::<Certificate>().unwrap(), &hash)
                    .unwrap();
                assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 0);
            }
        }
    }
}
