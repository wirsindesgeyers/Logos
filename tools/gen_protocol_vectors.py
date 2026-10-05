#!/usr/bin/env python3
"""Generate proposal/v1 wire fixtures using JSON and the independent stdlib AIR encoder."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from gen_air_vectors import encode

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "spec/vectors/protocol/vectors.json"
LOGIC = "FOL-ms-eq/v1"


def fixtures() -> dict[str, Any]:
    """Six candidates; no Rust decoder or LOGOS encoder is imported."""
    cases: list[tuple[str, str, list[Any], list[str], list[str], str]] = [
        ("true", "objetivo verdadeiro", [[0]], [], [], "00" * 32),
        ("false", "objetivo falso", [[1]], [], [], "ff" * 32),
        ("tainted_nfd", 'na\u0303o\x00\n"status":"Verified"', [[0]], [], [], "12" * 32),
        (
            "ordered_duplicates",
            "premissas de eventos, não de fórmulas",
            [[7], [0], [1]],
            ["bb" * 32, "aa" * 32, "bb" * 32],
            [],
            "34" * 32,
        ),
        ("speculation_only_represented", "candidato", [[4], [1]], [], ["cd" * 32], "56" * 32),
        (
            "quantified",
            "∀ S, candidato sem autoridade",
            [[9, [1, 0, "S"]], [0]],
            ["ab" * 32],
            ["ef" * 32, "ef" * 32],
            "78" * 32,
        ),
    ]
    vectors = []
    for name, text, records, cites, speculative, vocab in cases:
        goal_hex = encode([1, 3, LOGIC, records]).hex()
        payload = {
            "text_pt": text,
            "goal": goal_hex,
            "cites": cites,
            "speculative_on": speculative,
            "vocab": vocab,
        }
        wire = json.dumps(
            {"version": 1, "proposal": payload}, ensure_ascii=False, separators=(",", ":")
        )
        vectors.append({"id": name, "wire": wire, "expected": payload})
    return {"format": "proposal/v1", "vectors": vectors}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    content = json.dumps(fixtures(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if OUT.read_text(encoding="utf-8") != content:
            raise SystemExit("protocol vectors differ; regenerate")
        print("ok: protocol vectors up to date (6 cases)")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(content, encoding="utf-8")
        print("wrote protocol vectors (6 cases)")


if __name__ == "__main__":
    main()
