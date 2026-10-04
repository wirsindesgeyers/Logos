#!/usr/bin/env python3
"""Generate air/v1 TSV fixtures using only the Python standard library.

No LOGOS encoder is imported. Run without arguments to write the fixtures, or
with --check to compare the entire generated file (including errors and hashes).
The outcome column describes decoded syntax, not sort-checking or normalization.
"""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
from typing import Any

type WireArray = list[Any]

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "spec/vectors/air/vectors.tsv"
LOGIC = "FOL-ms-eq/v1"
FORMULA_TAG = b"logos/formula/v1\0"


def head(major: int, argument: int) -> bytes:
    """Shortest definite CBOR header, for arguments in the unsigned u64 range."""
    if not 0 <= argument < 1 << 64:
        raise ValueError("CBOR argument outside u64")
    if argument < 24:
        return bytes([(major << 5) | argument])
    for width, additional in ((1, 24), (2, 25), (4, 26), (8, 27)):
        if argument < 1 << (8 * width):
            return bytes([(major << 5) | additional]) + argument.to_bytes(width, "big")
    raise ValueError("unreachable CBOR width")


def encode(value: object) -> bytes:
    """Minimal independent CBOR encoder for the scalar/array fixture subset."""
    if value is None:
        return b"\xf6"
    if isinstance(value, bool):
        return b"\xf5" if value else b"\xf4"
    if isinstance(value, int):
        return head(0, value) if value >= 0 else head(1, -1 - value)
    if isinstance(value, str):
        raw = value.encode("utf-8")
        return head(3, len(raw)) + raw
    if isinstance(value, list):
        return head(4, len(value)) + b"".join(encode(item) for item in value)
    raise TypeError(f"unsupported fixture value {type(value)}")


def sort(name: str) -> WireArray:
    return [1, 0, name]


def symbol(name: str, kind: object, args: object, result: object) -> WireArray:
    return [1, 1, name, kind, args, result]


def term(*nodes: WireArray) -> WireArray:
    return [1, 2, LOGIC, list(nodes)]


def formula(*nodes: WireArray) -> WireArray:
    return [1, 3, LOGIC, list(nodes)]


def describe(target: str, value: WireArray) -> str:
    """Independent semantic expectation; every field participates, in wire order."""
    if target == "Sort":
        return f"S({value[2]})"
    if target == "Symbol":
        _, _, name, kind, args, result = value
        result_text = describe("Sort", result[1]) if result[0] == 0 else "Bool"
        return (
            f"Y({name};{('Const', 'Func', 'Pred')[kind]};"
            f"[{','.join(describe('Sort', arg) for arg in args)}];{result_text})"
        )
    parts = []
    for node in value[3]:
        tag = node[0]
        if target == "Term":
            if tag == 0:
                parts.append(f"V({node[1]};{describe('Sort', node[2])})")
            else:
                parts.append(f"A({describe('Symbol', node[1])};{node[2]})")
        elif tag in (0, 1, 4, 7, 8):
            parts.append({0: "True", 1: "False", 4: "Not", 7: "Implies", 8: "Iff"}[tag])
        elif tag == 2:
            parts.append(f"Eq({describe('Term', node[1])};{describe('Term', node[2])})")
        elif tag == 3:
            parts.append(
                f"Pred({describe('Symbol', node[1])};"
                f"[{','.join(describe('Term', arg) for arg in node[2])}])"
            )
        elif tag in (5, 6):
            parts.append(f"{('And' if tag == 5 else 'Or')}({node[1]})")
        else:
            parts.append(f"{('Forall' if tag == 9 else 'Exists')}({describe('Sort', node[1])})")
    prefix = "T" if target == "Term" else "F"
    return f"{prefix}({value[2]};[{','.join(parts)}])"


def build_vectors() -> list[tuple[str, str, str, str, str]]:
    rows = []

    def ok(case: str, target: str, value: WireArray) -> None:
        raw = encode(value)
        digest = hashlib.sha256(FORMULA_TAG + raw).hexdigest() if target == "Formula" else "-"
        rows.append((case, target, raw.hex(), digest, "ok:" + describe(target, value)))

    def bad(case: str, target: str, value: object, error: str = "Canon:WrongType") -> None:
        raw = encode(value)
        rows.append((case, target, raw.hex(), "-", "err:" + error))

    u, v = sort("U"), sort("V")
    c = symbol("c", 0, [], [0, u])
    f = symbol("f", 1, [u, v], [0, v])
    p = symbol("P", 2, [u, v], [1])
    x, y = term([0, 0, u]), term([0, 1, v])
    for name in ("", "U", "V", "café"):
        ok("sort-" + (name or "empty"), "Sort", sort(name))
    for name, value in (("const", c), ("func", f), ("pred", p)):
        ok("symbol-" + name, "Symbol", value)
    # Signatures are syntax, not judgments: incongruent kind/result remain legal.
    ok("symbol-const-bool", "Symbol", symbol("odd", 0, [u], [1]))
    ok("symbol-pred-sort", "Symbol", symbol("odd", 2, [], [0, v]))
    for index in (0, 1, 23, 24, 255, 256, 65535, 65536, (1 << 64) - 1):
        ok(f"term-var-{index}", "Term", term([0, index, u]))
    ok("term-const", "Term", term([1, c, 0]))
    ok("term-app", "Term", term([1, f, 2], [0, 0, u], [1, c, 0]))
    ok("term-nested-app", "Term", term([1, f, 2], [1, f, 1], [0, 1, v], [0, 0, u]))
    ok("term-predicate-app", "Term", term([1, p, 0]))
    cases: list[tuple[str, list[WireArray]]] = [
        ("true", [[0]]),
        ("false", [[1]]),
        ("eq", [[2, x, y]]),
        ("pred", [[3, p, [x, y]]]),
        ("pred-empty", [[3, c, []]]),
        ("not", [[4], [0]]),
        ("and-zero", [[5, 0]]),
        ("or-zero", [[6, 0]]),
        ("and", [[5, 2], [0], [1]]),
        ("or", [[6, 2], [1], [0]]),
        ("implies", [[7], [0], [1]]),
        ("iff", [[8], [1], [0]]),
        ("forall", [[9, u], [2, x, x]]),
        ("forall-pred", [[9, u], [3, symbol("P", 2, [u], [1]), [x]]]),
        ("exists", [[10, v], [2, y, x]]),
        ("nested-binders", [[9, u], [10, v], [2, x, y]]),
        ("ordered-connectives", [[5, 3], [4], [0], [7], [1], [0], [6, 0]]),
    ]
    for name, nodes in cases:
        ok("formula-" + name, "Formula", formula(*nodes))

    envelopes = {"Sort": u, "Symbol": c, "Term": x, "Formula": formula([0])}
    for target, envelope in envelopes.items():
        for version in (0, 2, -1, "1", None):
            bad(f"err-{target.lower()}-version-{version}", target, [version, *envelope[1:]])
        for tag in (99, -1, "0", None):
            bad(f"err-{target.lower()}-tag-{tag}", target, [1, tag, *envelope[2:]])
        bad(f"err-{target.lower()}-arity-short", target, envelope[:-1])
        bad(f"err-{target.lower()}-arity-long", target, [*envelope, 0])
        bad(f"err-{target.lower()}-not-array", target, 1)
    for target, make in (("Term", term), ("Formula", formula)):
        leaf = [0, 0, u] if target == "Term" else [0]
        for logic in ("FOL-ms-eq/v2", "", "fol-ms-eq/v1"):
            bad(
                f"err-{target.lower()}-logic-{logic or 'empty'}",
                target,
                [1, 2 if target == "Term" else 3, logic, [leaf]],
                "UnsupportedLogic",
            )
        bad(
            f"err-{target.lower()}-logic-type", target, [1, 2 if target == "Term" else 3, 0, [leaf]]
        )
        bad(f"err-{target.lower()}-empty", target, make(), "InvalidStructure")
        bad(f"err-{target.lower()}-extra-root", target, make(leaf, leaf), "InvalidStructure")
        bad(
            f"err-{target.lower()}-nodes-type",
            target,
            [1, 2 if target == "Term" else 3, LOGIC, None],
        )
    for kind in (3, 255, -1, "Const", None):
        bad(f"err-symbol-kind-{kind}", "Symbol", symbol("c", kind, [], [0, u]))
    for name, result in (
        ("empty", []),
        ("reserved", [2]),
        ("sort-missing", [0]),
        ("sort-extra", [0, u, v]),
        ("bool-extra", [1, u]),
        ("negative", [-1]),
        ("text", ["Bool"]),
        ("scalar", 1),
    ):
        bad(f"err-symbol-result-{name}", "Symbol", symbol("c", 0, [], result))
    bad("err-sort-name-type", "Sort", [1, 0, 9])
    bad("err-symbol-name-type", "Symbol", [1, 1, 9, 0, [], [0, u]])
    bad("err-symbol-arity-type", "Symbol", [1, 1, "c", 0, 0, [0, u]])
    bad("err-symbol-invalid-sort", "Symbol", [1, 1, "c", 0, [[2, 0, "U"]], [0, u]])
    for name, node in (
        ("reserved", [2]),
        ("var-short", [0, 0]),
        ("var-extra", [0, 0, u, 0]),
        ("var-negative", [0, -1, u]),
        ("var-text", [0, "0", u]),
        ("app-short", [1, c]),
        ("app-extra", [1, c, 0, 0]),
        ("app-negative", [1, c, -1]),
        ("app-text", [1, c, "0"]),
        ("missing-child", [1, c, 1]),
    ):
        error = (
            "InvalidStructure"
            if name == "missing-child"
            else "Canon:IntOutOfRange"
            if name in ("var-negative", "app-negative")
            else "Canon:WrongType"
        )
        bad("err-term-" + name, "Term", term(node), error)
    node_variants: list[WireArray] = [
        [0],
        [1],
        [2, x, y],
        [3, p, [x]],
        [4],
        [5, 0],
        [6, 0],
        [7],
        [8],
        [9, u],
        [10, u],
    ]
    for tag, record in enumerate(node_variants):
        if tag == 0 or len(record) > 1:
            bad(f"err-formula-node-{tag}-short", "Formula", formula(record[:-1]))
        bad(f"err-formula-node-{tag}-long", "Formula", formula([*record, 0]))
    bad_formulas: list[tuple[str, list[WireArray]]] = [
        ("reserved", [[11]]),
        ("negative-tag", [[-1]]),
        ("tag-type", [["True"]]),
        ("not-missing", [[4]]),
        ("and-missing", [[5, 2], [0]]),
        ("or-missing", [[6, 1]]),
        ("implies-missing", [[7], [0]]),
        ("iff-missing", [[8], [0]]),
        ("forall-missing", [[9, u]]),
        ("exists-missing", [[10, u]]),
        ("and-negative", [[5, -1]]),
        ("or-text", [[6, "0"]]),
        ("pred-args-type", [[3, p, 0]]),
        ("eq-invalid-term", [[2, term(), x]]),
        ("pred-invalid-second-term", [[3, p, [x, term()]]]),
        ("invalid-binder-sort", [[9, [2, 0, "U"]], [0]]),
    ]
    for name, bad_nodes in bad_formulas:
        shape_errors = {
            "not-missing",
            "and-missing",
            "or-missing",
            "implies-missing",
            "iff-missing",
            "forall-missing",
            "exists-missing",
            "eq-invalid-term",
            "pred-invalid-second-term",
        }
        error = (
            "InvalidStructure"
            if name in shape_errors
            else "Canon:IntOutOfRange"
            if name == "and-negative"
            else "Canon:WrongType"
        )
        bad("err-formula-" + name, "Formula", formula(*bad_nodes), error)
    bad(
        "err-formula-default-depth",
        "Formula",
        formula(*([[4]] * 128 + [[0]])),
        "DepthExceeded:128:129",
    )
    bad(
        "err-term-default-depth",
        "Term",
        term(*([[1, c, 1]] * 128 + [[0, 0, u]])),
        "DepthExceeded:128:129",
    )
    for name, target, raw, error in (
        ("empty-input", "Formula", b"", "Eof"),
        ("trailing-byte", "Formula", encode(formula([0])) + b"\x00", "TrailingBytes"),
        ("nonminimal-version", "Sort", b"\x83\x18\x01\x00\x61U", "NonMinimalHead"),
        ("indefinite-array", "Sort", b"\x9f\x01\x00\x61U\xff", "Indefinite"),
    ):
        rows.append(("err-canon-" + name, target, raw.hex(), "-", "err:Canon:" + error))
    return rows


def render_tsv(rows: list[tuple[str, str, str, str, str]]) -> str:
    lines = [
        "# case\ttype\thex\thash\toutcome — air/v1",
        "# hash = SHA-256(logos/formula/v1\\0 || hex bytes) for accepted Formula; '-' otherwise",
        "# outcome = ok:<syntax descriptor> | err:<AirError or Canon:name>;"
        " depth errors include limit:actual",
    ]
    return "\n".join(lines + ["\t".join(row) for row in rows]) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare the entire generated TSV")
    args = parser.parse_args()
    rows = build_vectors()
    rendered = render_tsv(rows)
    if args.check:
        if not OUT.exists() or OUT.read_bytes() != rendered.encode("utf-8"):
            print(f"error: stale or missing {OUT.relative_to(ROOT)}; run tools/gen_air_vectors.py")
            return 1
        print(f"ok: air vectors up to date ({len(rows)} cases)")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(rendered.encode("utf-8"))
    print(f"written {OUT.relative_to(ROOT)} ({len(rows)} cases)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
