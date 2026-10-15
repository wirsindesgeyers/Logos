#!/usr/bin/env python3
"""Vetores independentes de candidatos: apenas AIR/v1 existente, sem novo wire."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "py"))
from logos_client import canon  # noqa: E402

OUT = ROOT / "spec/vectors/fragment/vectors.json"
LOGIC = "FOL-ms-eq/v1"


def build() -> dict[str, Any]:
    """Emite ASTs/bytes a partir do contrato normativo, sem Rust."""
    vectors = []
    sort: list[Any] = [1, 0, "E"]
    for source_tag, source_name in enumerate(["Fact", "Policy"]):
        for n in range(4):
            source = bytes([119 + source_tag]) * 32
            symbols = [[1, 1, name, 0, [], [0, sort]] for name in ["z", "a", "é"][:n]]
            vocab: list[Any] = [
                1,
                1,
                [["E", [1, symbols, [source_tag, source]]]],
                [[s, [2, "fixture"], "fixture"] for s in symbols],
            ]
            vwire = canon.encode(vocab)
            var: list[Any] = [1, 2, LOGIC, [[0, 0, sort]]]
            terms = [[1, 2, LOGIC, [[1, s, 0]]] for s in symbols]
            closure: list[Any] = [1, 3, LOGIC, [[9, sort], [6, n], *[[2, var, t] for t in terms]]]
            distinct: list[Any] = [
                [1, 3, LOGIC, [[4], [2, terms[i], terms[j]]]]
                for i in range(n)
                for j in range(i + 1, n)
            ]
            formulas = [closure, *distinct]
            wires = [canon.encode(f) for f in formulas]
            vectors.append(
                dict(
                    id=f"{source_name}-{n}",
                    source=source_name,
                    payload=source.hex(),
                    vocabulary=vwire.hex(),
                    hv=hashlib.sha256(b"logos/vocab/v1\0" + vwire).hexdigest(),
                    formulas=[w.hex() for w in wires],
                    hashes=[hashlib.sha256(b"logos/formula/v1\0" + w).hexdigest() for w in wires],
                )
            )
    return dict(format="finite-fragment-examples/v1", vectors=vectors)


def main() -> None:
    """Regenera ou confere bytes/hash dos oito casos de fonte/cardinalidade."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if OUT.read_text() != output:
            raise SystemExit("fragment vectors differ")
        print("FRAGMENT_VECTORS_OK cases=8")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(output)


if __name__ == "__main__":
    main()
