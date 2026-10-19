//! Captura real de arquivos adversários e snapshot conferido antes do CAS.
#![cfg(target_os = "linux")]
mod support;
use logos_canon::Text;
use logos_crypto::hash_raw;
use logos_kernel::{staging::Staging, wire::ObjectRef};
use logos_ledger::{Cas, Certificate, Limits};
use std::{
    fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
};
use support::*;
fn reference(data: &[u8]) -> ObjectRef<Certificate> {
    ObjectRef::Staged {
        name: Text::new("a".repeat(64)).unwrap(),
        hash: hash_raw(data),
    }
}

#[test]
fn large_upload_is_a_snapshot_even_when_the_agent_keeps_an_open_descriptor() {
    let temp = Temp::new();
    let source = temp.dir("source");
    let private = temp.dir("private");
    let cas = Cas::open(temp.0.join("cas"), Limits::default()).unwrap();
    let uid = rustix::process::geteuid().as_raw();
    let staging = Staging::open(&source, &private, uid, 4 * 1024 * 1024).unwrap();
    let data = vec![0x52; 3 * 1024 * 1024];
    let path = source.join("a".repeat(64));
    let mut writer = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    writer.write_all(&data).unwrap();
    let mut quota = 4 * 1024 * 1024;
    let hash = staging.import(&reference(&data), &cas, &mut quota).unwrap();
    assert_eq!(quota, 1024 * 1024);
    assert!(!path.exists());
    writer.write_all(b"late agent mutation").unwrap();
    assert_eq!(cas.get(&hash).unwrap(), Some(data));
    assert_eq!(fs::read_dir(&private).unwrap().count(), 0);
}

#[test]
fn hash_mismatch_never_publishes_the_wrong_object_and_consumes_quota() {
    let temp = Temp::new();
    let source = temp.dir("source");
    let private = temp.dir("private");
    let cas = Cas::open(temp.0.join("cas"), Limits::default()).unwrap();
    let staging =
        Staging::open(&source, &private, rustix::process::geteuid().as_raw(), 100).unwrap();
    fs::write(source.join("a".repeat(64)), b"wrong").unwrap();
    let mut quota = 100;
    assert!(
        staging
            .import(&reference(b"expected"), &cas, &mut quota)
            .is_err()
    );
    assert_eq!(quota, 95);
    assert_eq!(cas.get(&hash_raw::<Certificate>(b"wrong")).unwrap(), None);
    assert_eq!(
        cas.get(&hash_raw::<Certificate>(b"expected")).unwrap(),
        None
    );
    assert_eq!(fs::read_dir(&private).unwrap().count(), 0);
}

#[test]
fn symlinks_hardlinks_fifo_directories_and_traversal_do_not_reach_the_cas() {
    let temp = Temp::new();
    let source = temp.dir("source");
    let private = temp.dir("private");
    let cas = Cas::open(temp.0.join("cas"), Limits::default()).unwrap();
    let staging =
        Staging::open(&source, &private, rustix::process::geteuid().as_raw(), 100).unwrap();
    let path = source.join("a".repeat(64));
    let target = temp.0.join("target");
    fs::write(&target, b"secret").unwrap();
    symlink(&target, &path).unwrap();
    assert!(
        staging
            .import(&reference(b"secret"), &cas, &mut 100)
            .is_err()
    );
    assert_eq!(fs::read(&target).unwrap(), b"secret");
    fs::hard_link(&target, &path).unwrap();
    assert!(
        staging
            .import(&reference(b"secret"), &cas, &mut 100)
            .is_err()
    );
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    assert!(staging.import(&reference(b""), &cas, &mut 100).is_err());
    fs::create_dir(&path).unwrap();
    assert!(staging.import(&reference(b""), &cas, &mut 100).is_err());
    assert_eq!(fs::read_dir(&private).unwrap().count(), 0);
    let bad = ObjectRef::<Certificate>::Staged {
        name: Text::new("../target").unwrap(),
        hash: hash_raw(b"secret"),
    };
    assert!(staging.import(&bad, &cas, &mut 100).is_err());
    assert_eq!(cas.get(&hash_raw::<Certificate>(b"secret")).unwrap(), None);
}

#[test]
fn rejected_directories_are_returned_intact_without_private_orphans() {
    let temp = Temp::new();
    let source = temp.dir("source");
    let private = temp.dir("private");
    let cas = Cas::open(temp.0.join("cas"), Limits::default()).unwrap();
    let staging =
        Staging::open(&source, &private, rustix::process::geteuid().as_raw(), 100).unwrap();
    let path = source.join("a".repeat(64));
    let target = temp.0.join("target");
    fs::write(&target, b"outside").unwrap();
    for populated in [false, true] {
        fs::create_dir(&path).unwrap();
        if populated {
            fs::write(path.join("child"), b"inside").unwrap();
            symlink(&target, path.join("link")).unwrap();
        }
        let mut quota = 100;
        let error = staging
            .import(&reference(b""), &cas, &mut quota)
            .unwrap_err();
        assert!(matches!(
            error,
            logos_kernel::TransportError::Invalid("staged object type/owner/links/mode")
        ));
        assert_eq!(quota, 100);
        assert_eq!(fs::read_dir(&private).unwrap().count(), 0);
        assert!(path.is_dir());
        if populated {
            assert_eq!(fs::read(path.join("child")).unwrap(), b"inside");
            assert!(
                fs::symlink_metadata(path.join("link"))
                    .unwrap()
                    .is_symlink()
            );
        }
        assert_eq!(fs::read(&target).unwrap(), b"outside");
        fs::remove_dir_all(&path).unwrap();
    }
}

#[test]
fn directory_ownership_permissions_limits_and_absent_cas_are_explicit_errors() {
    let temp = Temp::new();
    let source = temp.dir("source");
    let private = temp.dir("private");
    let uid = rustix::process::geteuid().as_raw();
    assert!(Staging::open(&source, &private, uid.wrapping_add(1), 10).is_err());
    fs::set_permissions(&private, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Staging::open(&source, &private, uid, 10).is_err());
    fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
    let cas = Cas::open(temp.0.join("cas"), Limits::default()).unwrap();
    let staging = Staging::open(&source, &private, uid, 3).unwrap();
    fs::write(source.join("a".repeat(64)), b"four").unwrap();
    assert!(staging.import(&reference(b"four"), &cas, &mut 100).is_err());
    assert!(
        staging
            .import(
                &ObjectRef::<Certificate>::Cas(hash_raw(b"missing")),
                &cas,
                &mut 100
            )
            .is_err()
    );
    assert!(Staging::open(&source, &source, uid, 10).is_err());
}
