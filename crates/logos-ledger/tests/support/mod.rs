use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use logos_canon::Text;
use logos_crypto::{FileSigner, Hash, Signer};
use logos_ledger::{EventBody, Route};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Temp(pub PathBuf);
impl Temp {
    pub fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "logos-ledger-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => panic!("create temp: {e}"),
            }
        }
    }
    pub fn database(&self) -> PathBuf {
        self.0.join("ledger.sqlite3")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

pub fn signer() -> FileSigner {
    FileSigner::from_seed(&[0x11; 32]).unwrap()
}

pub fn genesis(signer: &impl Signer) -> EventBody {
    EventBody::Genesis {
        ledger_version: 1,
        kernel_binary: [0x10; 32],
        kernel_key: signer.key_id().unwrap(),
    }
}

pub fn ordinary(id: u8) -> EventBody {
    EventBody::ObligationCreated {
        obligation: Hash::from_bytes([id; 32]),
        route: Route::Model,
        artifact: [id; 32],
    }
}

pub fn epoch(new_epoch: u64) -> EventBody {
    EventBody::EpochAdvanced {
        new_epoch,
        reason: Text::new("fixture").unwrap(),
    }
}

pub fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
        .collect()
}
