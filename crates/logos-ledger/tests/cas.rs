//! Comportamento público cas/v1 com arquivos reais e limites pequenos.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_crypto::{Domain, Hash, hash_raw, tags};
use logos_ledger::{Cas, CasError, Limits};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};

struct Certificate;
impl Domain for Certificate {
    const TAG: &'static [u8] = tags::CERTIFICATE;
}
struct Obligation;
impl Domain for Obligation {
    const TAG: &'static [u8] = tags::OBLIGATION;
}
struct Unknown;
impl Domain for Unknown {
    const TAG: &'static [u8] = b"../../escape\0";
}
struct MissingNul;
impl Domain for MissingNul {
    const TAG: &'static [u8] = b"logos/certificate/v1";
}

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "logos-cas-integration-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("create temp: {e}"),
            }
        }
    }
    fn cas(&self, limit: u64) -> Cas {
        Cas::open(
            &self.0,
            Limits::default()
                .with_domain_limit::<Certificate>(limit)
                .unwrap(),
        )
        .unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn object_path<T: Domain>(root: &Path, directory: &str, hash: &Hash<T>) -> PathBuf {
    let hex = hash
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    root.join(directory).join(&hex[..2]).join(&hex[2..])
}
fn too_large<T>(result: Result<T, CasError>, limit: u64, actual: u64) {
    assert!(
        matches!(result, Err(CasError::TooLarge { limit: l, actual: a }) if l == limit && a == actual)
    );
}

#[test]
fn corrupted_byte_is_integrity_error() {
    let temp = Temp::new();
    let cas = temp.cas(32);
    let hash = cas.put::<Certificate>(b"correct").unwrap();
    let path = object_path(&temp.0, "certificate-v1", &hash);
    fs::write(&path, b"correcu").unwrap();
    assert!(matches!(cas.get(&hash), Err(CasError::Integrity)));
    // A correct put repairs corruption rather than trusting existence.
    assert_eq!(cas.put::<Certificate>(b"correct").unwrap(), hash);
    assert_eq!(cas.get(&hash).unwrap(), Some(b"correct".to_vec()));
}

#[test]
fn concurrent_put_same_object() {
    let temp = Temp::new();
    let cas = Arc::new(temp.cas(4096));
    let barrier = Arc::new(Barrier::new(16));
    let bytes = vec![0x93; 2048];
    let expected = hash_raw::<Certificate>(&bytes);
    std::thread::scope(|scope| {
        let threads = (0..16)
            .map(|_| {
                let cas = Arc::clone(&cas);
                let barrier = Arc::clone(&barrier);
                let bytes = &bytes;
                scope.spawn(move || {
                    barrier.wait();
                    cas.put::<Certificate>(bytes).unwrap()
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            assert_eq!(thread.join().unwrap(), expected);
        }
    });
    drop(cas);
    assert_eq!(temp.cas(4096).get(&expected).unwrap(), Some(bytes));
    let path = object_path(&temp.0, "certificate-v1", &expected);
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn size_limits_at_boundary() {
    for limit in [0u64, 1, 7, 8192, 8193] {
        let temp = Temp::new();
        let cas = temp.cas(limit);
        for size in [limit.saturating_sub(1), limit] {
            let bytes = vec![0x42; size as usize];
            let hash = cas.put::<Certificate>(&bytes).unwrap();
            assert_eq!(cas.put_reader::<Certificate>(&bytes[..]).unwrap(), hash);
            assert_eq!(cas.get(&hash).unwrap(), Some(bytes));
        }
        let bytes = vec![0x42; (limit + 1) as usize];
        too_large(cas.put::<Certificate>(&bytes), limit, limit + 1);
        too_large(cas.put_reader::<Certificate>(&bytes[..]), limit, limit + 1);
        let expected = hash_raw::<Certificate>(&bytes);
        assert!(cas.get(&expected).unwrap().is_none());
        let path = object_path(&temp.0, "certificate-v1", &expected);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &bytes).unwrap();
        too_large(cas.get(&expected), limit, limit + 1);
        // Excesso precede integridade, mesmo num endereço deliberadamente errado.
        fs::write(&path, vec![0x43; (limit + 3) as usize]).unwrap();
        too_large(cas.get(&expected), limit, limit + 1);
        assert_eq!(
            cas.get(&cas.put::<Obligation>(&bytes).unwrap()).unwrap(),
            Some(bytes)
        );
    }
}

struct CountingReader {
    consumed: usize,
    total: usize,
}
impl Read for CountingReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        let n = out.len().min(self.total - self.consumed);
        out[..n].fill(7);
        self.consumed += n;
        Ok(n)
    }
}
#[test]
fn bounded_read_stops_at_first_excess_byte() {
    for limit in [0u64, 1, 8191, 8192, 8193] {
        let temp = Temp::new();
        let cas = temp.cas(limit);
        let mut reader = CountingReader {
            consumed: 0,
            total: (limit + 100) as usize,
        };
        too_large(cas.put_reader::<Certificate>(&mut reader), limit, limit + 1);
        assert_eq!(reader.consumed, (limit + 1) as usize);
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
    }
}

struct FailingReader {
    first: bool,
}
impl Read for FailingReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.first {
            self.first = false;
            out[0] = 1;
            Ok(1)
        } else {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "reader failed"))
        }
    }
}
#[test]
fn reader_error_does_not_publish() {
    let temp = Temp::new();
    let cas = temp.cas(32);
    assert!(
        matches!(cas.put_reader::<Certificate>(FailingReader { first: true }), Err(CasError::Io(e)) if e.kind() == io::ErrorKind::BrokenPipe)
    );
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
}

struct InterruptOnce<'a> {
    interrupted: bool,
    bytes: &'a [u8],
}
impl Read for InterruptOnce<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        self.bytes.read(out)
    }
}
#[test]
fn put_reader_matches_put() {
    let temp = Temp::new();
    let cas = temp.cas(100_000);
    for bytes in [vec![], vec![9], vec![0x23; 35_000]] {
        let hash = cas.put::<Certificate>(&bytes).unwrap();
        assert_eq!(hash, hash_raw::<Certificate>(&bytes));
        assert_eq!(
            cas.put_reader::<Certificate>(InterruptOnce {
                interrupted: false,
                bytes: &bytes
            })
            .unwrap(),
            hash
        );
        assert_eq!(temp.cas(100_000).get(&hash).unwrap(), Some(bytes));
    }
}

#[test]
fn missing_object_is_none_and_io_is_distinct() {
    let temp = Temp::new();
    let cas = temp.cas(10);
    let hash = hash_raw::<Certificate>(b"absent");
    assert!(cas.get(&hash).unwrap().is_none());
    let path = object_path(&temp.0, "certificate-v1", &hash);
    fs::create_dir_all(&path).unwrap();
    assert!(matches!(cas.get(&hash), Err(CasError::Io(_))));
    let root_file = temp.0.join("file");
    fs::write(&root_file, b"not directory").unwrap();
    assert!(matches!(
        Cas::open(&root_file, Limits::default()),
        Err(CasError::Io(_))
    ));
}

#[test]
fn domains_and_limits_are_independent() {
    let temp = Temp::new();
    let limits = Limits::default()
        .with_domain_limit::<Certificate>(0)
        .unwrap()
        .with_domain_limit::<Certificate>(3)
        .unwrap();
    let cas = Cas::open(&temp.0, limits).unwrap();
    let cert = cas.put::<Certificate>(b"abc").unwrap();
    let obligation = cas.put::<Obligation>(b"abc").unwrap();
    assert_ne!(cert.as_bytes(), obligation.as_bytes());
    assert!(object_path(&temp.0, "certificate-v1", &cert).is_file());
    assert!(object_path(&temp.0, "obligation-v1", &obligation).is_file());
    too_large(cas.put::<Certificate>(b"abcd"), 3, 4);
    assert_eq!(
        cas.get(&cas.put::<Obligation>(b"abcd").unwrap()).unwrap(),
        Some(b"abcd".to_vec())
    );
}

fn invalid_domain<T: Domain>(cas: &Cas) {
    assert!(
        matches!(Limits::default().with_domain_limit::<T>(1), Err(CasError::Io(e)) if e.kind() == io::ErrorKind::InvalidInput)
    );
    assert!(
        matches!(cas.put::<T>(b"x"), Err(CasError::Io(e)) if e.kind() == io::ErrorKind::InvalidInput)
    );
    let mut reader = CountingReader {
        consumed: 0,
        total: 10,
    };
    assert!(
        matches!(cas.put_reader::<T>(&mut reader), Err(CasError::Io(e)) if e.kind() == io::ErrorKind::InvalidInput)
    );
    assert_eq!(reader.consumed, 0);
    assert!(
        matches!(cas.get(&Hash::<T>::from_bytes([0; 32])), Err(CasError::Io(e)) if e.kind() == io::ErrorKind::InvalidInput)
    );
}
#[test]
fn unknown_and_malformed_tags_are_invalid_input() {
    let temp = Temp::new();
    let cas = temp.cas(10);
    invalid_domain::<Unknown>(&cas);
    invalid_domain::<MissingNul>(&cas);
    assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
}
#[test]
fn unrepresentable_limit_is_invalid_input() {
    for limit in [isize::MAX as u64 + 1, u64::MAX] {
        assert!(
            matches!(Limits::default().with_domain_limit::<Certificate>(limit), Err(CasError::Io(e)) if e.kind() == io::ErrorKind::InvalidInput)
        );
    }
}
