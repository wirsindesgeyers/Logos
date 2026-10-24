"""Confere os vetores de hash/KeyId/assinatura com domínio contra implementações independentes.

Os arquivos em spec/vectors/ são gerados por tools/gen_crypto_vectors.py (hashlib + cryptography
+ canon.py, sem o crate Rust); o Rust os confere em crates/logos-crypto/tests/vectors.rs.
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey, Ed25519PublicKey

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))

import gen_crypto_vectors as gen  # noqa: E402


def rows(rel: str) -> list[list[str]]:
    path = ROOT / "spec/vectors" / rel
    return [
        line.split("\t")
        for line in path.read_text().splitlines()
        if line and not line.startswith("#")
    ]


def test_committed_vectors_are_up_to_date() -> None:
    assert gen.OUT_HASH.read_text() == gen.render_hash()
    assert gen.OUT_KEYID.read_text() == gen.render_keyid()
    assert gen.OUT_SIG.read_text() == gen.render_sig()


def test_rfc8032_vectors_hold_for_the_python_reference() -> None:
    vectors = rows("crypto/ed25519-rfc8032.tsv")
    assert len(vectors) == 5
    for name, seed, pub, msg, sig in vectors:
        key = Ed25519PrivateKey.from_private_bytes(bytes.fromhex(seed))
        assert key.public_key().public_bytes_raw().hex() == pub, name
        assert key.sign(bytes.fromhex(msg)).hex() == sig, name
        Ed25519PublicKey.from_public_bytes(bytes.fromhex(pub)).verify(
            bytes.fromhex(sig), bytes.fromhex(msg)
        )


def test_spec_tags_are_nul_terminated_and_prefix_free() -> None:
    hash_tags, sig_tags = gen.spec_tags()
    assert len(hash_tags) == 23
    assert len(sig_tags) == 8
    tags = [*hash_tags.values(), *sig_tags.values()]
    for i, a in enumerate(tags):
        assert a.endswith(b"\x00") and a.count(b"\x00") == 1
        for b in tags[i + 1 :]:
            assert not a.startswith(b)
            assert not b.startswith(a)


def test_hash_domains_separate() -> None:
    by_msg: dict[str, set[str]] = {}
    for _dom, mname, _msg, digest in rows("domains/hash.tsv"):
        by_msg.setdefault(mname, set()).add(digest)
    # 23 domínios x mesma mensagem => 23 digests distintos
    assert all(len(v) == 23 for v in by_msg.values())
    # e nenhum coincide com o SHA-256 puro (sem tag)
    plain = {hashlib.sha256(bytes.fromhex(r[2])).hexdigest() for r in rows("domains/hash.tsv")}
    assert not plain & {r[3] for r in rows("domains/hash.tsv")}
