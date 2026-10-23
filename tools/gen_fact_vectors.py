#!/usr/bin/env python3
"""Vetores fact/v1 (6.1, ADR0061): canon, hashlib e Ed25519 independentes do Rust.

Sementes públicas de fixture, nunca credenciais de produção. --check confere
spec/vectors/fact/fact.json sem regravar.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "py"))
from logos_client import canon  # noqa: E402

OUT = ROOT / "spec/vectors/fact/fact.json"
SIG_TAG = b"logos/sig/fact/v1\0"
HASH_TAG = b"logos/fact/v1\0"
TRUE: Any = [1, 3, "FOL-ms-eq/v1", [[0]]]
FALSE: Any = [1, 3, "FOL-ms-eq/v1", [[1]]]
VOCAB = b"\xa1" * 32


def public_key(private: Ed25519PrivateKey) -> canon.CanonMap:
    raw = private.public_key().public_bytes_raw()
    return canon.CanonMap([("alg", "ed25519"), ("key", raw)])


def signature(raw: bytes) -> canon.CanonMap:
    return canon.CanonMap([("alg", "ed25519"), ("sig", raw)])


def fact(private: Ed25519PrivateKey, body_tail: list[Any]) -> tuple[bytes, bytes, bytes]:
    body: list[Any] = [1, *body_tail[:1], VOCAB, public_key(private), *body_tail[1:]]
    signed = canon.encode(body)
    sig = private.sign(SIG_TAG + signed)
    wire = canon.encode([*body, signature(sig)])
    return signed, wire, hashlib.sha256(HASH_TAG + wire).digest()


def build() -> dict[str, Any]:
    issuer = Ed25519PrivateKey.from_private_bytes(b"\x61" * 32)
    other = Ed25519PrivateKey.from_private_bytes(b"\x13" * 32)
    valid: list[dict[str, Any]] = []
    cases: list[tuple[str, list[Any]]] = [
        ("epochs-3-5", [TRUE, "clima", 1_700_000_000, [0, 3, 5], []]),
        ("epochs-max", [FALSE, "limite", 0, [0, 0, 2**64 - 1], []]),
        ("until", [TRUE, "clima", 1_700_000_000, [1, 1_800_000_000], []]),
        (
            "versioned",
            [TRUE, "backup", 1_700_000_001, [1, 2**64 - 1], [["db/prod", "v42"]]],
        ),
    ]
    for name, tail in cases:
        signed, wire, digest = fact(issuer, tail)
        valid.append(
            {
                "id": name,
                "issuer_seed_hex": (b"\x61" * 32).hex(),
                "signed_canon_hex": signed.hex(),
                "fact_canon_hex": wire.hex(),
                "fact_hash_hex": digest.hex(),
            }
        )
    base = [TRUE, "clima", 1, [0, 3, 5], []]
    signed, wire, _ = fact(issuer, base)
    body: list[Any] = [1, TRUE, VOCAB, public_key(issuer), "clima", 1, [0, 3, 5], []]
    good_sig = signature(issuer.sign(SIG_TAG + signed))
    invalid = [
        ("version-2", canon.encode([2, *body[1:], good_sig])),
        ("arity-8", canon.encode(body)),
        ("inverted-range", canon.encode([*body[:6], [0, 5, 3], [], good_sig])),
        ("validity-tag-2", canon.encode([*body[:6], [2, 3], [], good_sig])),
        ("empty-topic", canon.encode([*body[:4], "", *body[5:], good_sig])),
        ("two-versions", canon.encode([*body[:7], [["a", "1"], ["b", "2"]], good_sig])),
    ]
    # Decodifica mas a assinatura não verifica: outro emissor, corpo trocado.
    wrong_signer = canon.encode([*body, signature(other.sign(SIG_TAG + signed))])
    tampered = canon.encode([*body[:4], "outro", *body[5:], good_sig])
    return {
        "format": "fact/v1",
        "signature_tag": SIG_TAG.decode("ascii").rstrip("\0"),
        "hash_tag": HASH_TAG.decode("ascii").rstrip("\0"),
        "valid": valid,
        "invalid": [{"id": name, "fact_canon_hex": data.hex()} for name, data in invalid],
        "bad_signature": [
            {"id": "wrong-signer", "fact_canon_hex": wrong_signer.hex()},
            {"id": "tampered-topic", "fact_canon_hex": tampered.hex()},
        ],
        "reference_wire_hex": wire.hex(),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rendered = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            raise SystemExit("fact vectors differ; run tools/gen_fact_vectors.py")
        print("ok: fact vectors up to date")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(rendered, encoding="utf-8")
        print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
