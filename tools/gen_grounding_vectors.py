#!/usr/bin/env python3
"""Vetores ground/v1 de lógica booleana; referência independente de Rust."""

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

OUT = ROOT / "spec/vectors/grounding/vectors.json"
LOGIC = "FOL-ms-eq/v1"


def build() -> dict[str, Any]:
    """Constrói AST/IR para casos pequenos, sem importar o grounder."""
    vectors = []
    symbols = [[1, 1, n, 2, [], [1]] for n in ["z", "a"]]
    vocab: list[Any] = [1, 1, [], [[s, [2, "vector"], "vector"] for s in symbols]]
    hv = hashlib.sha256(b"logos/vocab/v1\0" + canon.encode(vocab)).digest()
    # Tuplas aninhadas existem somente neste gerador não confiável.
    p: Any = ("p", 0)
    q: Any = ("p", 1)
    cases: list[tuple[Any, list[Any]]] = [
        (True, []),
        (False, []),
        (p, []),
        (p, [p]),
        (False, [False]),
        (("not", p), [q]),
        (("and", p, q), [p]),
        (("or", p, q), [p]),
        (("implies", p, q), [q]),
        (("iff", p, q), [p, q]),
        (("and",), []),
        (("or",), []),
    ]
    for index, (goal, premises) in enumerate(cases):
        nodes: list[list[Any]] = [[0], [1], [2, 0], [2, 1]]

        def intern(node: list[Any], target: list[list[Any]] = nodes) -> int:
            if node not in target:
                target.append(node)
            return target.index(node)

        def op(tag: int, xs: list[int]) -> int:
            if not xs:
                return int(tag == 4)
            if len(xs) == 1:
                return xs[0]
            return intern([tag, xs])

        def implies(a: int, b: int) -> int:
            return op(5, [intern([3, a]), b])

        def compile_ref(ast: Any) -> int:
            if isinstance(ast, bool):
                return int(ast)
            name, *args = ast
            if name == "p":
                return 2 + int(args[0])
            xs = [compile_ref(a) for a in args]
            if name == "not":
                return intern([3, xs[0]])
            if name in ("and", "or"):
                return op(4 if name == "and" else 5, xs)
            if name == "implies":
                return implies(*xs)
            return op(4, [implies(*xs), implies(*reversed(xs))])

        def air(ast: Any) -> list[Any]:
            if isinstance(ast, bool):
                return [[0 if ast else 1]]
            name, *args = ast
            if name == "p":
                return [[3, symbols[args[0]], []]]
            tag = dict(not_=4, and_=5, or_=6, implies=7, iff=8)[
                name + "_" if name in ("not", "and", "or") else name
            ]
            record = [tag, len(args)] if name in ("and", "or") else [tag]
            return [record, *[node for arg in args for node in air(arg)]]

        def formula(ast: Any) -> list[Any]:
            return [1, 3, LOGIC, air(ast)]

        refs = [i.to_bytes(4, "big") + bytes(28) for i in range(len(premises))]
        pairs: list[Any] = [[r, formula(f)] for r, f in zip(refs, premises, strict=True)]
        ob: list[Any] = [1, 0, LOGIC, 1, hv, pairs, formula(goal)]
        ho = hashlib.sha256(b"logos/obligation/v1\0" + canon.encode(ob)).digest()
        roots: list[Any] = [[r, compile_ref(f)] for r, f in zip(refs, premises, strict=True)]
        g = compile_ref(goal)
        root = implies(op(4, [1, *[n for _, n in roots]]), g)
        wire: list[Any] = [1, ho, hv, [[0, [], []], [1, [], []]], nodes, 1, roots, g, root, []]
        raw = canon.encode(wire)
        vectors.append(
            dict(
                id=f"bool-{index}",
                vocabulary=canon.encode(vocab).hex(),
                goal=canon.encode(formula(goal)).hex(),
                premises=[canon.encode(formula(f)).hex() for f in premises],
                obligation=canon.encode(ob).hex(),
                ground=raw.hex(),
                digest=hashlib.sha256(b"logos/ground/v1\0" + raw).hexdigest(),
            )
        )
    return dict(format="ground/v1", vectors=vectors)


def main() -> None:
    """Regenera ou confere o corpus fixo."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if OUT.read_text() != output:
            raise SystemExit("grounding vectors differ")
        print("GROUND_VECTORS_OK cases=12")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(output)


if __name__ == "__main__":
    main()
