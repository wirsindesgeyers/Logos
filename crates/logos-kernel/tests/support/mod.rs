#![allow(dead_code)]
use logos_crypto::{FileSigner, Signer};
use logos_ledger::{EventBody, Ledger};
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct Temp(pub PathBuf);
impl Temp {
    pub fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "logos-transport-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
                    return Self(path);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(e) => panic!("fixture: {e}"),
            }
        }
    }
    pub fn dir(&self, name: &str) -> PathBuf {
        let p = self.0.join(name);
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        p
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
pub fn signer() -> FileSigner {
    FileSigner::from_seed(&[0x52; 32]).unwrap()
}
pub fn genesis(ledger: &mut Ledger<'_, FileSigner>, s: &FileSigner) {
    ledger
        .append(EventBody::Genesis {
            ledger_version: 1,
            kernel_binary: [0x52; 32],
            kernel_key: s.key_id().unwrap(),
        })
        .unwrap();
}
