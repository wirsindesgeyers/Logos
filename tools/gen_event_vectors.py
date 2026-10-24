#!/usr/bin/env python3
"""Vetores event/v1 + epoch-body/v2: canon, hashlib e Ed25519 independentes do Rust.

Sementes públicas de fixture, nunca credenciais de produção. --check confere
spec/vectors/events/vectors.json sem regravar.
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

OUT = ROOT / "spec/vectors/events/vectors.json"
SIG_TAG = b"logos/sig/event/v1\0"


class KeyPair:
    """Identidade/assinatura determinística de uma chave pública de fixture."""

    def __init__(self, seed: bytes) -> None:
        self.seed = seed
        self.private = Ed25519PrivateKey.from_private_bytes(seed)
        self.public = self.private.public_key().public_bytes_raw()
        self.wire = canon.encode(canon.CanonMap([("alg", "ed25519"), ("key", self.public)]))
        self.key_id = hashlib.sha256(b"logos/key/v1\0" + self.wire).digest()

    def sign(self, message: bytes) -> bytes:
        return self.private.sign(SIG_TAG + message)


def signature(raw: bytes) -> canon.CanonMap:
    return canon.CanonMap([("alg", "ed25519"), ("sig", raw)])


def build_vectors() -> list[dict[str, Any]]:
    kernel = KeyPair(b"\x11" * 32)
    admin = KeyPair(b"\x22" * 32)
    author = KeyPair(b"\x33" * 32)
    vocabulary, obligation, certificate, checker, model = (bytes([b]) * 32 for b in range(161, 166))
    ref1, ref2 = b"\x01" * 32, b"\x02" * 32
    budget = [1, 1000, 1048576, 65536, 500]
    accepted = [obligation, certificate, checker, budget, [1, 0]]
    unknown = [obligation, certificate, checker, budget, [1, 2, [1, 0]]]
    rejected = [
        obligation,
        certificate,
        checker,
        budget,
        [1, 1, [1, "candidato inválido"]],
    ]
    true = [1, 3, "FOL-ms-eq/v1", [[0]]]
    false = [1, 3, "FOL-ms-eq/v1", [[1]]]
    vectors: list[dict[str, Any]] = []
    previous: bytes | None = None

    def add(name: str, body: list[Any], epoch: int = 0, authority: KeyPair | None = None) -> None:
        nonlocal previous
        auth_message = auth_sig = None
        if authority is not None:
            # Body projection retains its tag and authority KeyId, but not sig.
            auth_message = canon.encode([1, 1, body])
            auth_sig = authority.sign(auth_message)
            body = [*body[:-1], [authority.key_id, signature(auth_sig)]]
        seq = len(vectors)
        unsigned: list[Any] = [
            1,
            0,
            seq,
            epoch,
            [] if previous is None else [previous],
            body,
            kernel.key_id,
        ]
        message = canon.encode(unsigned)
        sig = kernel.sign(message)
        wire = canon.encode([*unsigned, signature(sig)])
        digest = hashlib.sha256(b"logos/event/v1\0" + wire).digest()
        vectors.append(
            {
                "id": name,
                "body_tag": body[1],
                "kernel_seed_hex": kernel.seed.hex(),
                "kernel_public_key_hex": kernel.public.hex(),
                "kernel_public_key_canon_hex": kernel.wire.hex(),
                "kernel_key_id_hex": kernel.key_id.hex(),
                "authority_seed_hex": authority.seed.hex() if authority else None,
                "authority_public_key_hex": authority.public.hex() if authority else None,
                "authority_message_hex": auth_message.hex() if auth_message else None,
                "authority_sig_hex": auth_sig.hex() if auth_sig else None,
                "unsigned_canon_hex": message.hex(),
                "event_sig_hex": sig.hex(),
                "event_wire_hex": wire.hex(),
                "event_hash_hex": digest.hex(),
            }
        )
        previous = digest

    add("Genesis", [1, 0, 1, b"\x0f" * 32, kernel.key_id])
    add("VocabPublished", [1, 1, vocabulary, True, admin.key_id], authority=admin)
    proposal = [b"Proposta inicial", true, [ref1], [ref2], vocabulary]
    add("Proposed", [1, 2, proposal, author.key_id])
    add("ObligationCreated", [1, 3, obligation, 0, b"\x8a" * 32])
    add("Verified-unknown-check", [1, 4, accepted, [ref1, ref2], [ref1], [0, [unknown]]])
    add("Refuted", [1, 5, accepted, model])
    add("UnknownRecorded", [1, 6, unknown, "x86_64-linux"])
    add("AssumptionDeclared", [1, 7, vocabulary, true, "escopo local", author.key_id])
    add("Retracted", [1, 8, ref1, "retratado", admin.key_id], authority=admin)
    add("Invalidated", [1, 9, ref2, [ref2, ref1]])
    add("EpochAdvanced", [1, 10, 1, "nova época"], epoch=1)
    add("CheckerRegistered", [1, 11, checker, "model/v1", admin.key_id], 1, admin)
    add("CheckerRevoked", [1, 12, checker, "checker revogado", admin.key_id], 1, admin)
    add("Verified-witness", [1, 4, accepted, [ref1], [ref1], [1, model, accepted]], 1)
    add("Verified-rejected-witness", [1, 4, accepted, [], [], [0, [rejected]]], 1)
    add("Verified-no-consistency-attempt", [1, 4, accepted, [], [], [0, []]], 1)
    nfd_proposal = [
        "na\u0303o cafe\u0301".encode(),
        false,
        [ref2, ref1, ref2],
        [ref1, ref1],
        vocabulary,
    ]
    add("Proposed-tainted-nfd-duplicates", [1, 2, nfd_proposal, author.key_id], 1)
    steps = [obligation, certificate, checker, budget, [1, 2, [1, 1]]]
    add("UnknownRecorded-steps", [1, 6, steps, "aarch64-linux"], 1)
    add("EpochAdvanced-root", [2, 10, 2, "raiz opaca", [b"\x71" * 32], []], 2)
    add("EpochAdvanced-vocab", [2, 10, 3, "vocabulário", [], [vocabulary]], 3)
    add("EpochAdvanced-both", [2, 10, 4, "ambos", [b"\x72" * 32], [vocabulary]], 4)
    add(
        "EpochAdvanced-max",
        [2, 10, 2**64 - 1, "limite", [bytes(32)], [bytes(32)]],
        2**64 - 1,
    )
    add(
        "ObligationCreatedCnf",
        [2, 3, obligation, 1, b"\x8b" * 32, [b"\x8c" * 32, 1]],
        epoch=(1 << 64) - 1,
    )
    add("CertificateRejected", [2, 13, rejected], epoch=(1 << 64) - 1)
    target = bytes.fromhex(vectors[-2]["event_hash_hex"])
    for name, check in [("accepted", accepted), ("rejected", rejected), ("unknown", unknown)]:
        add("ConsistencyRecorded-" + name, [3, 14, target, check, model], epoch=(1 << 64) - 1)
    # rechecked-body/v1 (5.10, ADR0056): Rechecked de uma rota com o checker novo.
    for name, check in [("accepted", accepted), ("rejected", rejected), ("unknown", unknown)]:
        add("Rechecked-" + name, [1, 15, ref1, check], epoch=(1 << 64) - 1)
    # fact-registered-body/v1 (6.1, ADR0061): só o h(Fact); o objeto fica no CAS.
    add("FactRegistered", [1, 16, b"\xfa" * 32], epoch=(1 << 64) - 1)
    # trust-config-body/v1 (6.2, ADR0063): h(TrustConfig) assinado pelo admin.
    add("TrustConfigPublished", [1, 17, b"\x7c" * 32, admin.key_id], (1 << 64) - 1, admin)
    # issuer-revoked-body/v1 (6.4, ADR0064): KeyId do emissor e época de registro.
    add("IssuerRevoked", [1, 18, b"\x5e" * 32, 3, admin.key_id], (1 << 64) - 1, admin)
    # extracted-body/v1 (6.6, ADR0068): fonte, SHA-256 do conteúdo, literal Says e extrator.
    add(
        "Extracted",
        [1, 19, vocabulary, "email1", b"\xc0" * 32, true, author.key_id],
        epoch=(1 << 64) - 1,
    )
    # policy-published-body/v1 (7.2, ADR0070): h(Policy), versão, raiz Merkle e autoridade do admin.
    add(
        "PolicyPublished",
        [1, 20, b"\x70" * 32, 4, b"\x71" * 32, admin.key_id],
        (1 << 64) - 1,
        admin,
    )
    return vectors


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    vectors = build_vectors()
    rendered = (
        json.dumps(
            {
                "format": "event/v1+epoch-body/v2+certificate-rejected-body/v2"
                "+consistency-body/v3+rechecked-body/v1+fact-registered-body/v1"
                "+trust-config-body/v1+issuer-revoked-body/v1",
                "vectors": vectors,
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n"
    )
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            raise SystemExit("event vectors differ; run tools/gen_event_vectors.py")
        print(f"ok: event vectors up to date ({len(vectors)} cases)")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(rendered, encoding="utf-8")
        print(f"wrote {OUT} ({len(vectors)} cases)")


if __name__ == "__main__":
    main()
