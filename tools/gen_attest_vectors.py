#!/usr/bin/env python3
"""Vetores do SDK de atestador (6.3): o ``clock`` de Python e o de Rust produzem os mesmos bytes.

Sementes públicas de fixture, nunca credenciais de produção. --check confere
spec/vectors/attest/clock.json sem regravar; o lado Rust (logos-attest/tests/vectors.rs)
reproduz cada caso com o atestador de referência e compara os bytes.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "py"))
from logos_attest import Ed25519Signer, attest  # noqa: E402
from logos_attest.clock import Clock  # noqa: E402

OUT = ROOT / "spec/vectors/attest/clock.json"
SEED = b"\xa0" * 32
# Longe no futuro: o kernel recusa fato com janela vencida, e o vetor precisa continuar registrável.
NOW = 4_000_000_000
TOLERANCE = 10
TTL = 3600
# (id, desvio da referência em segundos)
CASES = [("synced", 3), ("exact", 0), ("edge", TOLERANCE), ("skewed", TOLERANCE + 1)]


def build() -> dict[str, Any]:
    signer = Ed25519Signer.from_seed(SEED)
    cases = []
    for name, skew in CASES:
        attestation = attest(Clock(NOW + skew, TOLERANCE, TTL), signer, NOW)
        cases.append(
            {
                "id": name,
                "reference_unix": NOW + skew,
                "synced": skew <= TOLERANCE,
                "fact_canon_hex": attestation.fact.hex(),
                "fact_hash_hex": attestation.fact_hash.hex(),
                "vocabulary_canon_hex": attestation.vocabulary.hex(),
                "vocabulary_hash_hex": attestation.vocabulary_hash.hex(),
            }
        )
    return {
        "format": "attest-clock/v1",
        "attester_seed_hex": SEED.hex(),
        "now": NOW,
        "tolerance_secs": TOLERANCE,
        "ttl_secs": TTL,
        "cases": cases,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    rendered = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            raise SystemExit("attest vectors differ; run tools/gen_attest_vectors.py")
        print("ok: attest vectors up to date")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(rendered, encoding="utf-8")
        print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
