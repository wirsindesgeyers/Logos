//! Vetores normativos: RFC 8032 (Ed25519 puro), hashes por domínio, KeyId e assinaturas com
//! domínio (estes três gerados de forma independente por `tools/gen_crypto_vectors.py`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod support;

use logos_canon::{from_canon, to_canon};
use logos_crypto::{FileSigner, PublicKey, SigAlg, Signature, Signer, verify};
use support::{RawSig, hash_by_name, hex, sign_by_name, unhex, vectors};

fn seed(s: &str) -> [u8; 32] {
    unhex(s).try_into().unwrap()
}

#[test]
fn rfc8032_ed25519_vectors() {
    let rows = vectors("crypto/ed25519-rfc8032.tsv");
    assert_eq!(rows.len(), 5);
    for row in rows {
        let (name, seed_hex, pub_hex, msg, sig_hex) = (&row[0], &row[1], &row[2], &row[3], &row[4]);
        let signer = FileSigner::from_seed(&seed(seed_hex)).unwrap();
        assert_eq!(
            hex(signer.public_key().as_bytes()),
            *pub_hex,
            "chave pública, vetor {name}"
        );
        let msg = unhex(msg);
        let sig = signer.sign::<RawSig>(&msg).unwrap();
        assert_eq!(hex(sig.as_bytes()), *sig_hex, "assinatura, vetor {name}");
        verify::<RawSig>(&msg, &sig, signer.public_key()).unwrap();
    }
}

#[test]
fn hash_vectors_match_independent_sha256() {
    let rows = vectors("domains/hash.tsv");
    assert!(rows.len() >= 60);
    for row in rows {
        let msg = unhex(&row[2]);
        assert_eq!(
            hash_by_name(&row[0], &msg),
            row[3],
            "{} / {}",
            row[0],
            row[1]
        );
    }
}

#[test]
fn keyid_vectors() {
    for row in vectors("domains/keyid.tsv") {
        let key = PublicKey::new(SigAlg::Ed25519, &unhex(&row[1])).unwrap();
        assert_eq!(
            hex(&to_canon(&key).unwrap()),
            row[2],
            "canon da chave {}",
            row[0]
        );
        assert_eq!(key.key_id().unwrap().to_hex(), row[3], "KeyId {}", row[0]);
        assert_eq!(from_canon::<PublicKey>(&unhex(&row[2])), Ok(key));
    }
}

#[test]
fn signature_with_domain_vectors() {
    let rows = vectors("crypto/sig-domain.tsv");
    assert_eq!(rows.len(), 48);
    for row in rows {
        let signer = FileSigner::from_seed(&seed(&row[0])).unwrap();
        let msg = unhex(&row[2]);
        let sig = sign_by_name(&signer, &row[1], &msg);
        assert_eq!(hex(sig.as_bytes()), row[4], "{} / {}", row[0], row[1]);
        assert_eq!(hex(signer.public_key().as_bytes()), row[3]);
        // a assinatura também sobrevive ao round-trip canônico
        assert_eq!(from_canon::<Signature>(&to_canon(&sig).unwrap()), Ok(sig));
    }
}
