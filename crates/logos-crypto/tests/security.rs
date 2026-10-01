//! Propriedades de segurança: separação de domínios, algoritmo identificado, chaves fracas,
//! maleabilidade, e carga de chave a partir de arquivo.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use logos_canon::{Map, Text, Value, from_canon, to_canon};
use logos_crypto::{
    CryptoError, FileSigner, PublicKey, SigAlg, Signature, Signer, hash, hash_raw, tags, verify,
};
use std::path::PathBuf;
use support::{RawSig, SEvent, SToken, hex, unhex};

const SEED: &str = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";

fn signer() -> FileSigner {
    FileSigner::from_seed(&unhex(SEED).try_into().unwrap()).unwrap()
}

#[test]
fn event_signature_does_not_verify_as_token() {
    let s = signer();
    let sig = s.sign::<SEvent>(b"payload").unwrap();
    verify::<SEvent>(b"payload", &sig, s.public_key()).unwrap();
    assert_eq!(
        verify::<SToken>(b"payload", &sig, s.public_key()),
        Err(CryptoError::BadSignature)
    );
    assert_eq!(
        verify::<RawSig>(b"payload", &sig, s.public_key()),
        Err(CryptoError::BadSignature)
    );
}

#[test]
fn tampered_message_or_signature_is_rejected() {
    let s = signer();
    let sig = s.sign::<SEvent>(b"payload").unwrap();
    assert_eq!(
        verify::<SEvent>(b"payloae", &sig, s.public_key()),
        Err(CryptoError::BadSignature)
    );
    for i in [0usize, 31, 32, 63] {
        let mut bytes = sig.as_bytes().to_vec();
        bytes[i] ^= 1;
        let bad = Signature::new(SigAlg::Ed25519, &bytes).unwrap();
        assert!(
            verify::<SEvent>(b"payload", &bad, s.public_key()).is_err(),
            "byte {i}"
        );
    }
}

#[test]
fn wrong_key_is_rejected() {
    let other = FileSigner::from_seed(&[7u8; 32]).unwrap();
    let sig = signer().sign::<SEvent>(b"x").unwrap();
    assert_eq!(
        verify::<SEvent>(b"x", &sig, other.public_key()),
        Err(CryptoError::BadSignature)
    );
}

/// s' = s + L (L = ordem do subgrupo): a mesma assinatura “maleada” não pode verificar.
#[test]
fn malleable_signature_is_rejected() {
    const L: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10,
    ];
    let s = signer();
    let sig = s.sign::<RawSig>(b"").unwrap();
    verify::<RawSig>(b"", &sig, s.public_key()).unwrap();
    let mut bytes = sig.as_bytes().to_vec();
    let mut carry = 0u16;
    for i in 0..32 {
        let sum = u16::from(bytes[32 + i]) + u16::from(L[i]) + carry;
        bytes[32 + i] = sum as u8;
        carry = sum >> 8;
    }
    assert_eq!(carry, 0, "s + L deve caber em 256 bits");
    let malleated = Signature::new(SigAlg::Ed25519, &bytes).unwrap();
    assert_eq!(
        verify::<RawSig>(b"", &malleated, s.public_key()),
        Err(CryptoError::BadSignature)
    );
}

#[test]
fn weak_and_malformed_public_keys_are_rejected() {
    let mut identity = [0u8; 32];
    identity[0] = 1; // ponto identidade: ordem 1
    assert_eq!(
        PublicKey::new(SigAlg::Ed25519, &identity),
        Err(CryptoError::InvalidPublicKey)
    );
    assert_eq!(
        PublicKey::new(SigAlg::Ed25519, &[0u8; 31]),
        Err(CryptoError::BadKeyLength)
    );
    assert_eq!(
        PublicKey::new(SigAlg::Ed25519, &[0u8; 33]),
        Err(CryptoError::BadKeyLength)
    );
    assert_eq!(
        Signature::new(SigAlg::Ed25519, &[0u8; 63]),
        Err(CryptoError::BadSignatureLength)
    );
}

#[test]
fn algorithm_is_identified_and_unknown_or_unimplemented_ones_are_typed_errors() {
    assert_eq!(SigAlg::from_name("rsa"), Err(CryptoError::UnknownAlgorithm));
    assert_eq!(
        SigAlg::from_name("ED25519"),
        Err(CryptoError::UnknownAlgorithm)
    );
    assert_eq!(SigAlg::from_name("es256"), Ok(SigAlg::Es256));
    let es256 = CryptoError::UnsupportedAlgorithm(SigAlg::Es256);
    assert_eq!(
        PublicKey::new(SigAlg::Es256, &[0u8; 32]).unwrap_err(),
        es256
    );
    assert_eq!(
        Signature::new(SigAlg::Es256, &[0u8; 64]).unwrap_err(),
        es256
    );
}

#[test]
fn tampering_the_algorithm_in_the_envelope_is_rejected() {
    let s = signer();
    let sig = s.sign::<SEvent>(b"m").unwrap();
    let t = |x: &str| Value::Text(Text::new(x).unwrap());
    for alg in ["es256", "rsa", "ed448", ""] {
        let forged = Value::Map(
            Map::new(vec![
                (t("alg"), t(alg)),
                (t("sig"), Value::Bytes(sig.as_bytes().to_vec())),
            ])
            .unwrap(),
        );
        assert!(
            from_canon::<Signature>(&to_canon(&forged).unwrap()).is_err(),
            "alg={alg:?}"
        );
    }
    // campos extras ou faltando
    let extra = Value::Map(
        Map::new(vec![
            (t("alg"), t("ed25519")),
            (t("sig"), Value::Bytes(sig.as_bytes().to_vec())),
            (t("x"), Value::Null),
        ])
        .unwrap(),
    );
    assert!(from_canon::<Signature>(&to_canon(&extra).unwrap()).is_err());
}

#[test]
fn key_id_binds_the_algorithm() {
    let s = signer();
    let id = s.key_id().unwrap();
    // O mesmo conjunto de bytes sob outro nome de algoritmo daria outra identidade.
    let t = |x: &str| Value::Text(Text::new(x).unwrap());
    let as_es256 = Value::Map(
        Map::new(vec![
            (t("alg"), t("es256")),
            (t("key"), Value::Bytes(s.public_key().as_bytes().to_vec())),
        ])
        .unwrap(),
    );
    let other = hash_raw::<PublicKey>(&to_canon(&as_es256).unwrap());
    assert_ne!(id, other);
    assert_eq!(id, hash(s.public_key()).unwrap());
    assert_eq!(
        id,
        hash_raw::<PublicKey>(&to_canon(s.public_key()).unwrap())
    );
    let _ = tags::KEY;
}

// ---------- arquivo de chave ----------

struct TempFile(PathBuf);

impl TempFile {
    fn new(tag: &str, content: &[u8], mode: u32) -> Self {
        use std::os::unix::fs::PermissionsExt;
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("logos-key-{}-{n}-{tag}", std::process::id()));
        std::fs::write(&path, content).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        Self(path)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn key_file_with_mode_0600_loads() {
    let f = TempFile::new("ok", format!("{SEED}\n").as_bytes(), 0o600);
    let loaded = FileSigner::load(&f.0).unwrap();
    assert_eq!(
        hex(loaded.public_key().as_bytes()),
        hex(signer().public_key().as_bytes())
    );
    let f400 = TempFile::new("ro", SEED.as_bytes(), 0o400);
    assert!(FileSigner::load(&f400.0).is_ok());
}

#[test]
fn key_file_with_open_permissions_is_refused() {
    for mode in [0o644, 0o640, 0o604, 0o660, 0o666, 0o700 | 0o001] {
        let f = TempFile::new("open", SEED.as_bytes(), mode);
        assert_eq!(
            FileSigner::load(&f.0).unwrap_err(),
            CryptoError::InsecurePermissions(mode),
            "modo {mode:o}"
        );
    }
}

#[test]
fn key_file_symlink_directory_and_missing_are_refused() {
    let target = TempFile::new("target", SEED.as_bytes(), 0o600);
    let link = std::env::temp_dir().join(format!("logos-key-link-{}", std::process::id()));
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&target.0, &link).unwrap();
    assert_eq!(
        FileSigner::load(&link).unwrap_err(),
        CryptoError::KeyFileNotRegular
    );
    std::fs::remove_file(&link).unwrap();

    assert_eq!(
        FileSigner::load(&std::env::temp_dir()).unwrap_err(),
        CryptoError::KeyFileNotRegular
    );
    let missing = std::env::temp_dir().join("logos-key-does-not-exist");
    assert!(matches!(
        FileSigner::load(&missing).unwrap_err(),
        CryptoError::Io(_)
    ));
}

#[test]
fn key_file_with_bad_content_is_refused() {
    for (tag, content) in [
        ("short", &b"abcd"[..]),
        ("upper", SEED.to_uppercase().as_bytes()),
        ("big", &[b'a'; 4096][..]),
    ] {
        let f = TempFile::new(tag, content, 0o600);
        assert_eq!(
            FileSigner::load(&f.0).unwrap_err(),
            CryptoError::KeyFileFormat,
            "{tag}"
        );
    }
}

#[test]
fn debug_output_never_contains_the_private_key() {
    let s = format!("{:?}", signer());
    assert!(
        !s.contains(SEED) && !s.to_lowercase().contains("secret"),
        "{s}"
    );
}
