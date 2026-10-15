#!/usr/bin/env python3
"""Fixture independente audit-stream/v1; chaves públicas de teste, sem Rust."""

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

OUT = ROOT / "spec/vectors/audit/vectors.json"


def build() -> dict[str, Any]:
    """Constrói bytes a partir das specs; não importa appender nem gerador Rust."""
    private = Ed25519PrivateKey.from_private_bytes(bytes([17]) * 32)
    public = private.public_key().public_bytes_raw()
    key_wire = canon.encode(canon.CanonMap([("alg", "ed25519"), ("key", public)]))
    key_id = hashlib.sha256(b"logos/key/v1\0" + key_wire).digest()
    objects: dict[str, dict[str, str]] = {}

    def put(name: str, wire: bytes) -> bytes:
        digest = hashlib.sha256(f"logos/{name}/v1\0".encode() + wire).digest()
        objects[name + digest.hex()] = dict(domain=name, digest=digest.hex(), wire=wire.hex())
        return digest

    vocab = put("vocab", canon.encode([1, 1, [["S", [0]]], []]))
    checker = put("checker", b"public fixture model checker identity; not executable")
    events: list[str] = []
    previous: bytes | None = None
    budget: list[Any] = [1, 1_000_000, 1_000_000, 1_000_000, 0]

    def event(body: list[Any], epoch: int = 0) -> None:
        nonlocal previous
        unsigned: list[Any] = [
            1,
            0,
            len(events),
            epoch,
            [] if previous is None else [previous],
            body,
            key_id,
        ]
        sig = private.sign(b"logos/sig/event/v1\0" + canon.encode(unsigned))
        wire = canon.encode([*unsigned, canon.CanonMap([("alg", "ed25519"), ("sig", sig)])])
        previous = hashlib.sha256(b"logos/event/v1\0" + wire).digest()
        events.append(wire.hex())

    def check(nodes: list[Any], cardinality: int) -> tuple[list[Any], bytes]:
        goal: list[Any] = [1, 3, "FOL-ms-eq/v1", nodes]
        obligation = put("obligation", canon.encode([1, 0, "FOL-ms-eq/v1", 1, vocab, [], goal]))
        model_wire = canon.encode([1, vocab, [cardinality], [], [], []])
        model = put("model", model_wire)
        cert = put("certificate", model_wire)
        return [obligation, cert, checker, budget, [1, 0]], model

    event([1, 0, 1, bytes([15]) * 32, key_id])
    for i, nodes in enumerate([[[1]], [[4], [0]], [[5, 2], [0], [1]], [[6, 2], [1], [1]]]):
        record, model = check(nodes, i + 1)
        event([1, 5, record, model])
    record, model = check([[0]], 2)
    event([1, 4, record, [], [], [1, model, record]])
    inconclusive = [*record[:-1], [1, 2, [1, 1]]]
    event([1, 6, inconclusive, "fixture"])
    event([2, 10, 4, "avanço", [bytes(32)], [vocab]], 4)
    stream = b"LOGOS-AUDIT/v1\n" + b"".join(
        len(bytes.fromhex(w)).to_bytes(4, "big") + bytes.fromhex(w) for w in events
    )
    return dict(
        format="audit-stream/v1",
        public_key_hex=public.hex(),
        objects=list(objects.values()),
        events=events,
        stream_hex=stream.hex(),
        expected=dict(events=8, refutations=4, consistencies=1, unchecked_proofs=1),
    )


def main() -> None:
    """Regenera ou confere fixture byte a byte."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if OUT.read_text() != output:
            raise SystemExit("audit vectors differ")
        print("AUDIT_VECTORS_OK")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(output)


if __name__ == "__main__":
    main()
