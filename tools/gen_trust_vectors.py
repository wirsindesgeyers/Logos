#!/usr/bin/env python3
"""Vetores trust/v1 (6.2, ADR0063): canon, hashlib e Ed25519 independentes do Rust.

Sementes públicas de fixture, nunca credenciais de produção. --check confere
spec/vectors/trust/trust.json sem regravar.
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

OUT = ROOT / "spec/vectors/trust/trust.json"
HASH_TAG = b"logos/trust/v1\0"
KEY_TAG = b"logos/key/v1\0"


def public_key(seed: bytes) -> canon.CanonMap:
    raw = Ed25519PrivateKey.from_private_bytes(seed).public_key().public_bytes_raw()
    return canon.CanonMap([("alg", "ed25519"), ("key", raw)])


def key_id(seed: bytes) -> bytes:
    return hashlib.sha256(KEY_TAG + canon.encode(public_key(seed))).digest()


def root(seed: bytes, name: str, topics: list[str], level: int = 1) -> list[Any]:
    return [public_key(seed), name, topics, level]


def by_key(roots: list[tuple[bytes, list[Any]]]) -> list[list[Any]]:
    return [r for _, r in sorted(roots, key=lambda pair: key_id(pair[0]))]


DB = b"\xdb" * 32
BACKUP = b"\xba" * 32


def build() -> dict[str, Any]:
    db = root(DB, "db-monitor", ["db"])
    backup = root(BACKUP, "backup-svc", ["backup", "backup/offsite"], 2)
    both = by_key([(DB, db), (BACKUP, backup)])
    valid_cases: list[tuple[str, list[Any]]] = [
        ("single", [1, 1, [db]]),
        ("two-roots", [1, 7, both]),
        ("max-version", [1, 2**64 - 1, [root(DB, "d", ["a", "b", "c"], 2**64 - 1)]]),
    ]
    valid = []
    for name, config in valid_cases:
        wire = canon.encode(config)
        valid.append(
            {
                "id": name,
                "trust_canon_hex": wire.hex(),
                "trust_hash_hex": hashlib.sha256(HASH_TAG + wire).digest().hex(),
            }
        )
    reversed_roots = list(reversed(both))
    invalid_cases: list[tuple[str, Any]] = [
        ("format-2", [2, 1, [db]]),
        ("version-0", [1, 0, [db]]),
        ("arity-2", [1, 1]),
        ("no-roots", [1, 1, []]),
        ("roots-out-of-order", [1, 1, reversed_roots]),
        ("root-twice", [1, 1, [db, db]]),
        ("no-topics", [1, 1, [root(DB, "db-monitor", [])]]),
        ("topics-out-of-order", [1, 1, [root(DB, "db-monitor", ["db", "backup"])]]),
        ("topic-twice", [1, 1, [root(DB, "db-monitor", ["db", "db"])]]),
        ("empty-topic", [1, 1, [root(DB, "db-monitor", [""])]]),
        ("empty-name", [1, 1, [root(DB, "", ["db"])]]),
        ("root-arity-3", [1, 1, [[public_key(DB), "db-monitor", ["db"]]]]),
    ]
    rule = [
        {"seed_hex": DB.hex(), "topic": "db", "trusted": True},
        {"seed_hex": DB.hex(), "topic": "backup", "trusted": False},
        {"seed_hex": BACKUP.hex(), "topic": "backup", "trusted": True},
        {"seed_hex": BACKUP.hex(), "topic": "backup/offsite", "trusted": True},
        {"seed_hex": BACKUP.hex(), "topic": "backup/", "trusted": False},
        {"seed_hex": BACKUP.hex(), "topic": "Backup", "trusted": False},
    ]
    return {
        "format": "trust/v1",
        "hash_tag": HASH_TAG.decode("ascii").rstrip("\0"),
        "valid": valid,
        "invalid": [
            {"id": name, "trust_canon_hex": canon.encode(c).hex()} for name, c in invalid_cases
        ],
        "rule": {"config": "two-roots", "cases": rule},
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rendered = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            raise SystemExit("trust vectors differ; run tools/gen_trust_vectors.py")
        print("ok: trust vectors up to date")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(rendered, encoding="utf-8")
        print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
