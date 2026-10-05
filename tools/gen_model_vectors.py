#!/usr/bin/env python3
"""Generate model/v1 vectors with stdlib CBOR and hashlib, independent of Rust."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
from pathlib import Path
from typing import Any

from gen_vocabulary_vectors import (
    bool_result,
    encode,
    fact_ref,
    formula,
    grounding_defined,
    grounding_glossed,
    sort,
    sort_decl_enum,
    sort_decl_open,
    sort_result,
    symbol,
    symbol_decl,
    vocabulary,
)

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "spec/vectors/model/vectors.json"
VOCAB_TAG = b"logos/vocab/v1\0"
MODEL_TAG = b"logos/model/v1\0"


def primitive(signature: list[Any]) -> list[Any]:
    return symbol_decl(signature, grounding_glossed("primitivo"), "glosa")


def packed(outputs: list[int]) -> bytes:
    return b"".join(output.to_bytes(4, "big") for output in outputs)


def model(
    vocab: list[Any],
    domains: list[int],
    consts: list[Any],
    funcs: list[tuple[int, list[int]]],
    preds: list[Any],
) -> list[Any]:
    return [
        1,
        hashlib.sha256(VOCAB_TAG + encode(vocab)).digest(),
        domains,
        consts,
        [[index, packed(outputs)] for index, outputs in funcs],
        preds,
    ]


def fixtures() -> dict[str, Any]:
    vectors: list[dict[str, Any]] = []

    def add(
        name: str,
        vocab: list[Any],
        value: list[Any],
        error: str | None = None,
        raw: bytes | None = None,
    ) -> None:
        wire = encode(value) if raw is None else raw
        row: dict[str, Any] = {
            "id": name,
            "vocabulary_hex": encode(vocab).hex(),
            "model_hex": wire.hex(),
            "model_hash": hashlib.sha256(MODEL_TAG + wire).hexdigest() if error is None else None,
            "expected_error": error,
        }
        if error is None:
            row["expected"] = {
                "vocab": value[1].hex(),
                "domains": value[2],
                "consts": value[3],
                "funcs": [
                    [
                        index,
                        [int.from_bytes(data[i : i + 4], "big") for i in range(0, len(data), 4)],
                    ]
                    for index, data in value[4]
                ],
                "preds": value[5],
            }
        vectors.append(row)

    empty_v = vocabulary(0, [], [])
    add("empty_vocabulary", empty_v, model(empty_v, [], [], [], []))

    s, t = sort("S"), sort("T")
    c = symbol("c", 0, [], sort_result("S"))
    f = symbol("f", 1, [s, t], sort_result("S"))
    p = symbol("P", 2, [t, s], bool_result())
    mixed_v = vocabulary(
        7,
        [sort_decl_open("S"), sort_decl_open("T")],
        [primitive(c), primitive(f), primitive(p)],
    )
    mixed = model(mixed_v, [2, 3], [[0, 1]], [(1, [0, 1, 0, 1, 0, 1])], [[2, [[0, 1], [2, 0]]]])
    add("mixed_axes", mixed_v, mixed)
    changes: list[tuple[str, int, Any, str]] = [
        ("partial_function", 4, [[1, packed([0, 1])]], "NonTotalFunction"),
        ("extra_function_output", 4, [[1, packed([0] * 7)]], "NonTotalFunction"),
        ("function_out_of_domain", 4, [[1, packed([0, 1, 0, 1, 0, 2])]], "ValueOutOfDomain"),
        ("constant_out_of_domain", 3, [[0, 2]], "ValueOutOfDomain"),
        ("missing_constant", 3, [], "InvalidInterpretation"),
        ("duplicate_constant_id", 3, [[0, 1], [0, 1]], "InvalidInterpretation"),
        ("wrong_function_kind", 4, [[0, packed([0] * 6)]], "InvalidInterpretation"),
        ("unknown_symbol", 5, [[3, []]], "InvalidInterpretation"),
        ("predicate_wrong_axis", 5, [[2, [[0, 2]]]], "ValueOutOfDomain"),
        ("predicate_wrong_arity", 5, [[2, [[0]]]], "InvalidTuple"),
        ("duplicate_tuple", 5, [[2, [[0, 1], [0, 1]]]], "InvalidTuple"),
        ("reversed_tuple_order", 5, [[2, [[2, 0], [0, 1]]]], "InvalidTuple"),
        ("missing_sort", 2, [2], "InvalidDomains"),
        ("empty_domain", 2, [0, 3], "InvalidDomains"),
        ("domain_size_limit", 2, [65537, 3], "Unknown(Size)"),
        ("expected_product_limit", 2, [65536, 65536], "Unknown(Size)"),
        ("vocabulary_mismatch", 1, bytes(32), "VocabularyMismatch"),
        ("unknown_version", 0, 2, "Canon(WrongType)"),
        ("short_packed_output", 4, [[1, b"\0\0\0"]], "NonTotalFunction"),
    ]
    for name, field, replacement, error in changes:
        changed = copy.deepcopy(mixed)
        changed[field] = replacement
        add(name, mixed_v, changed, error)
    add(
        "nonminimal_version",
        mixed_v,
        mixed,
        "Canon(NonMinimalHead)",
        encode(mixed)[:1] + b"\x18\x01" + encode(mixed)[2:],
    )
    add("trailing_bytes", mixed_v, mixed, "Canon(TrailingBytes)", encode(mixed) + b"\0")
    add("truncated", mixed_v, mixed, "Canon(Eof)", encode(mixed)[:-1])

    a = symbol("a", 0, [], sort_result("S"))
    b = symbol("b", 0, [], sort_result("S"))
    defined = symbol("D", 2, [], bool_result())
    enum_v = vocabulary(
        1,
        [sort_decl_enum("S", [a, b], fact_ref(bytes.fromhex("ab" * 32)))],
        [
            primitive(b),
            symbol_decl(defined, grounding_defined(formula([0])), "definido"),
            primitive(a),
        ],
    )
    enum_m = model(enum_v, [2], [[0, 1], [2, 0]], [], [])
    add("enumeration_declaration_order", enum_v, enum_m)
    changed = copy.deepcopy(enum_m)
    changed[2] = [3]
    add("enumeration_wrong_cardinality", enum_v, changed, "InvalidEnumeration")
    changed = copy.deepcopy(enum_m)
    changed[3] = [[0, 0], [2, 1]]
    add("enumeration_wrong_labels", enum_v, changed, "InvalidEnumeration")
    changed = copy.deepcopy(enum_m)
    changed[5] = [[1, [[]]]]
    add("defined_table_injected", enum_v, changed, "InvalidInterpretation")

    null_v = vocabulary(
        1,
        [sort_decl_open("S")],
        [primitive(symbol("f0", 1, [], sort_result("S"))), primitive(defined)],
    )
    add("nullary_true", null_v, model(null_v, [256], [], [(0, [255])], [[1, [[]]]]))
    add("nullary_false", null_v, model(null_v, [256], [], [(0, [255])], [[1, []]]))

    defined_enum_v = vocabulary(
        1,
        [sort_decl_enum("S", [a], fact_ref(bytes(32)))],
        [symbol_decl(a, grounding_defined(formula([0])), "definido")],
    )
    add("defined_enum_intrinsic_label", defined_enum_v, model(defined_enum_v, [1], [], [], []))
    invalid_v = vocabulary(
        1, [sort_decl_open("S")], [primitive(symbol("bad", 2, [], sort_result("S")))]
    )
    add(
        "invalid_predicate_signature",
        invalid_v,
        model(invalid_v, [1], [], [], [[0, []]]),
        "InvalidSignature",
    )
    return {"format": "model/v1", "vectors": vectors}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    result = fixtures()
    content = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if OUT.read_text(encoding="utf-8") != content:
            raise SystemExit("model vectors differ; regenerate")
        print(f"ok: model vectors up to date ({len(result['vectors'])} cases)")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(content, encoding="utf-8")
        print(f"wrote model vectors ({len(result['vectors'])} cases)")


if __name__ == "__main__":
    main()
