#!/usr/bin/env python3
"""Generate vocabulary/v1 TSV fixtures using only the Python standard library.

No LOGOS encoder is imported. Run without arguments to write the fixtures, or
with --check to compare the entire generated file (including errors and hashes).
The outcome column describes decoded syntax and semantic validation.
"""

from __future__ import annotations

import argparse
import hashlib
from collections import deque
from pathlib import Path
from typing import Any

type WireArray = list[Any]

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "spec/vectors/vocabulary/vectors.tsv"
LOGIC = "FOL-ms-eq/v1"
VOCAB_TAG = b"logos/vocab/v1\0"


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
    if isinstance(value, int):
        return head(0, value)
    if isinstance(value, str):
        payload = value.encode("utf-8")
        return head(3, len(payload)) + payload
    if isinstance(value, (bytes, bytearray)):
        return head(2, len(value)) + bytes(value)
    if isinstance(value, list):
        return head(4, len(value)) + b"".join(encode(item) for item in value)
    raise TypeError(f"unsupported fixture value {type(value)}")


def sort(name: str) -> WireArray:
    return [1, 0, name]


def symbol(name: str, kind: int, arity: list[WireArray], result: WireArray) -> WireArray:
    return [1, 1, name, kind, arity, result]


def sort_result(name: str) -> WireArray:
    return [0, sort(name)]


def bool_result() -> WireArray:
    return [1]


def term(*nodes: WireArray) -> WireArray:
    return [1, 2, LOGIC, list(nodes)]


def term_var(index: int, sort_wire: WireArray) -> WireArray:
    return [0, index, sort_wire]


def term_app(symbol_wire: WireArray, count: int) -> WireArray:
    return [1, symbol_wire, count]


def formula(*nodes: WireArray) -> WireArray:
    return [1, 3, LOGIC, list(nodes)]


def formula_true() -> WireArray:
    return [0]


def formula_false() -> WireArray:
    return [1]


def formula_eq(left: WireArray, right: WireArray) -> WireArray:
    return [2, left, right]


def formula_pred(symbol_wire: WireArray, *args: WireArray) -> WireArray:
    return [3, symbol_wire, list(args)]


def formula_not() -> WireArray:
    return [4]


def formula_and(count: int) -> WireArray:
    return [5, count]


def formula_or(count: int) -> WireArray:
    return [6, count]


def formula_implies() -> WireArray:
    return [7]


def formula_iff() -> WireArray:
    return [8]


def formula_forall(sort_wire: WireArray) -> WireArray:
    return [9, sort_wire]


def formula_exists(sort_wire: WireArray) -> WireArray:
    return [10, sort_wire]


def fact_ref(digest: bytes) -> WireArray:
    return [0, digest]


def policy_ref(digest: bytes) -> WireArray:
    return [1, digest]


def sort_decl_open(name: str) -> WireArray:
    return [name, [0]]


def sort_decl_enum(name: str, constants: list[WireArray], source: WireArray) -> WireArray:
    return [name, [1, constants, source]]


def grounding_defined(f: WireArray) -> WireArray:
    return [0, f]


def grounding_anchored(attestor: str, proc: str) -> WireArray:
    return [1, attestor, proc]


def grounding_glossed(text: str) -> WireArray:
    return [2, text]


def symbol_decl(sym: WireArray, grounding: WireArray, gloss: str) -> WireArray:
    return [sym, grounding, gloss]


def vocabulary(version: int, sorts: list[WireArray], symbols: list[WireArray]) -> WireArray:
    return [1, version, sorts, symbols]


def describe_sort(s: WireArray) -> str:
    return f"S({s[2]})"


def describe_symbol(s: WireArray) -> str:
    name = s[2]
    kind_str = ["Const", "Func", "Pred"][s[3]]
    arity_str = ",".join(describe_sort(arg) for arg in s[4])
    res = s[5]
    if res == [1]:
        res_str = "Bool"
    elif res[0] == 0:
        res_str = describe_sort(res[1])
    else:
        res_str = "Unknown"
    return f"Y({name};{kind_str};[{arity_str}];{res_str})"


def describe_term(t: WireArray) -> str:
    logic = t[2]
    nodes: list[str] = []
    for node in t[3]:
        tag = node[0]
        if tag == 0:
            nodes.append(f"V({node[1]};{describe_sort(node[2])})")
        elif tag == 1:
            nodes.append(f"A({describe_symbol(node[1])};{node[2]})")
    return f"T({logic};[{','.join(nodes)}])"


def describe_formula(f: WireArray) -> str:
    logic = f[2]
    nodes: list[str] = []
    for node in f[3]:
        tag = node[0]
        if tag == 0:
            nodes.append("True")
        elif tag == 1:
            nodes.append("False")
        elif tag == 2:
            nodes.append(f"Eq({describe_term(node[1])};{describe_term(node[2])})")
        elif tag == 3:
            sym_str = describe_symbol(node[1])
            args_str = ",".join(describe_term(a) for a in node[2])
            nodes.append(f"Pred({sym_str};[{args_str}])")
        elif tag == 4:
            nodes.append("Not")
        elif tag == 5:
            nodes.append(f"And({node[1]})")
        elif tag == 6:
            nodes.append(f"Or({node[1]})")
        elif tag == 7:
            nodes.append("Implies")
        elif tag == 8:
            nodes.append("Iff")
        elif tag == 9:
            nodes.append(f"Forall({describe_sort(node[1])})")
        elif tag == 10:
            nodes.append(f"Exists({describe_sort(node[1])})")
    return f"F({logic};[{','.join(nodes)}])"


def describe_sort_decl(decl: WireArray) -> str:
    name = decl[0]
    dom = decl[1]
    if dom == [0]:
        return f"S({name};Open)"
    if dom[0] == 1:
        consts = ",".join(c[2] for c in dom[1])
        src = dom[2]
        if src[0] == 0:
            src_str = f"Fact({src[1].hex()})"
        elif src[0] == 1:
            src_str = f"Policy({src[1].hex()})"
        else:
            src_str = "Unknown"
        return f"S({name};Enum:[{consts}];{src_str})"
    return "Unknown"


def describe_grounding(g: WireArray) -> str:
    tag = g[0]
    if tag == 0:
        return f"Defined({describe_formula(g[1])})"
    if tag == 1:
        return f"Anchored({g[1]};{g[2]})"
    if tag == 2:
        return f"Glossed({g[1]})"
    return "Unknown"


def describe_symbol_decl(decl: WireArray) -> str:
    return f"Y({describe_symbol(decl[0])};{describe_grounding(decl[1])};{decl[2]})"


def describe_vocabulary(vocab: WireArray, order: list[int]) -> str:
    version = vocab[1]
    sorts = ",".join(describe_sort_decl(s) for s in vocab[2])
    symbols = ",".join(describe_symbol_decl(s) for s in vocab[3])
    order_str = ",".join(str(i) for i in order)
    return f"V({version};[{sorts}];[{symbols}];[{order_str}])"


def compute_definition_order(symbols_wire: list[WireArray]) -> list[int] | None:
    symbol_index_by_name = {s[0][2]: i for i, s in enumerate(symbols_wire)}
    defined_indices = [i for i, s in enumerate(symbols_wire) if s[1][0] == 0]
    count = len(defined_indices)

    dependents: list[list[int]] = [[] for _ in symbols_wire]
    remaining: list[int] = [0] * len(symbols_wire)
    ready: deque[int] = deque()

    def check_term(term_wire: WireArray, deps: set[int]) -> None:
        for node in term_wire[3]:
            if node[0] == 1:
                name = node[1][2]
                idx = symbol_index_by_name.get(name)
                if idx is not None and symbols_wire[idx][1][0] == 0:
                    deps.add(idx)

    for i in defined_indices:
        formula_wire = symbols_wire[i][1][1]
        deps: set[int] = set()

        for node in formula_wire[3]:
            tag = node[0]
            if tag == 2:
                check_term(node[1], deps)
                check_term(node[2], deps)
            elif tag == 3:
                name = node[1][2]
                idx = symbol_index_by_name.get(name)
                if idx is not None and symbols_wire[idx][1][0] == 0:
                    deps.add(idx)
                for arg in node[2]:
                    check_term(arg, deps)

        remaining[i] = len(deps)
        if len(deps) == 0:
            ready.append(i)
        for dep in sorted(deps):
            dependents[dep].append(i)

    order: list[int] = []
    while ready:
        idx = ready.popleft()
        order.append(idx)
        for dep in dependents[idx]:
            remaining[dep] -= 1
            if remaining[dep] == 0:
                ready.append(dep)

    if len(order) != count:
        return None
    return order


def digest_of(payload: bytes) -> str:
    return hashlib.sha256(VOCAB_TAG + payload).hexdigest()


def build_vectors() -> list[tuple[str, str, str, str, str]]:
    rows: list[tuple[str, str, str, str, str]] = []

    def ok(name: str, vocab_wire: WireArray) -> None:
        raw = encode(vocab_wire)
        order = compute_definition_order(vocab_wire[3])
        if order is None:
            raise ValueError(f"expected acyclic definition order for {name}")
        desc = describe_vocabulary(vocab_wire, order)
        rows.append((name, "Vocabulary", raw.hex(), digest_of(raw), f"ok:{desc}"))

    def err(name: str, payload: bytes | WireArray, error_tag: str) -> None:
        raw = payload if isinstance(payload, bytes) else encode(payload)
        rows.append((name, "Vocabulary", raw.hex(), "-", f"err:{error_tag}"))

    # 1. Empty vocabularies
    ok("vocab-empty-0", vocabulary(0, [], []))
    ok("vocab-empty-1", vocabulary(1, [], []))

    # 2. u64 boundaries for version
    for v in (23, 24, 255, 256, 65535, 65536, 18446744073709551615):
        ok(f"vocab-boundary-version-{v}", vocabulary(v, [], []))

    # 3. Sorts: Open
    ok("vocab-open-sorts", vocabulary(1, [sort_decl_open("U"), sort_decl_open("V")], []))

    # 4. Sorts: Enumerated (Fact & Policy)
    fact_hash = bytes.fromhex("11" * 32)
    policy_hash = bytes.fromhex("22" * 32)
    c_active = symbol("Active", 0, [], sort_result("Status"))
    c_inactive = symbol("Inactive", 0, [], sort_result("Status"))
    ok(
        "vocab-enumerated-fact",
        vocabulary(
            1,
            [sort_decl_enum("Status", [c_active, c_inactive], fact_ref(fact_hash))],
            [
                symbol_decl(c_active, grounding_glossed("Status ativo"), "Ativo"),
                symbol_decl(c_inactive, grounding_glossed("Status inativo"), "Inativo"),
            ],
        ),
    )

    c_admin = symbol("Admin", 0, [], sort_result("Role"))
    c_user = symbol("User", 0, [], sort_result("Role"))
    ok(
        "vocab-enumerated-policy",
        vocabulary(
            1,
            [sort_decl_enum("Role", [c_admin, c_user], policy_ref(policy_hash))],
            [
                symbol_decl(c_admin, grounding_glossed("Papel administrador"), "Admin"),
                symbol_decl(c_user, grounding_glossed("Papel usuário comum"), "Usuário"),
            ],
        ),
    )

    # 5. Groundings: Glossed, Anchored, Defined
    p_human = symbol("human", 2, [sort("U")], bool_result())
    ok(
        "vocab-grounding-glossed",
        vocabulary(
            1,
            [sort_decl_open("U")],
            [symbol_decl(p_human, grounding_glossed("Entidade biológica humana"), "Humano")],
        ),
    )

    p_verified = symbol("verified", 2, [sort("U")], bool_result())
    ok(
        "vocab-grounding-anchored",
        vocabulary(
            1,
            [sort_decl_open("U")],
            [
                symbol_decl(
                    p_verified,
                    grounding_anchored("agent:did:logos:authority-1", "ed25519-v1-attest"),
                    "Verificado por autoridade",
                )
            ],
        ),
    )

    p_top = symbol("top", 2, [], bool_result())
    ok(
        "vocab-grounding-defined-base",
        vocabulary(
            1,
            [],
            [
                symbol_decl(
                    p_top,
                    grounding_defined(formula(formula_true())),
                    "Predicado tautológico verdadeiro",
                )
            ],
        ),
    )

    # 6. DAG structures
    d0 = symbol("d0", 2, [], bool_result())
    d1 = symbol("d1", 2, [], bool_result())
    ok(
        "vocab-dag-ordered",
        vocabulary(
            1,
            [],
            [
                symbol_decl(d0, grounding_defined(formula(formula_true())), "d0 def"),
                symbol_decl(
                    d1,
                    grounding_defined(formula(formula_pred(d0))),
                    "d1 depends on d0",
                ),
            ],
        ),
    )

    ok(
        "vocab-dag-inverted",
        vocabulary(
            1,
            [],
            [
                symbol_decl(
                    d1,
                    grounding_defined(formula(formula_pred(d0))),
                    "d1 declared first",
                ),
                symbol_decl(d0, grounding_defined(formula(formula_true())), "d0 declared second"),
            ],
        ),
    )

    ok(
        "vocab-dag-repeated-dependency",
        vocabulary(
            1,
            [],
            [
                symbol_decl(d0, grounding_defined(formula(formula_true())), "d0 def"),
                symbol_decl(
                    d1,
                    grounding_defined(formula(formula_and(2), formula_pred(d0), formula_pred(d0))),
                    "d1 references d0 twice",
                ),
            ],
        ),
    )

    # 7. Mixed groundings & topological order ignoring Anchored/Glossed
    s0_gloss = symbol("s0_gloss", 2, [sort("U")], bool_result())
    s1_anch = symbol("s1_anch", 2, [sort("U")], bool_result())
    s2_def = symbol("s2_def", 2, [sort("U")], bool_result())
    s3_def = symbol("s3_def", 2, [sort("U")], bool_result())
    ok(
        "vocab-mixed-groundings",
        vocabulary(
            1,
            [sort_decl_open("U")],
            [
                symbol_decl(s0_gloss, grounding_glossed("Base gloss"), "s0"),
                symbol_decl(
                    s1_anch,
                    grounding_anchored("agent:auth", "proc:check"),
                    "s1",
                ),
                symbol_decl(
                    s2_def,
                    grounding_defined(
                        formula(
                            formula_and(2),
                            formula_pred(s0_gloss, term(term_var(0, sort("U")))),
                            formula_pred(s1_anch, term(term_var(0, sort("U")))),
                        )
                    ),
                    "s2 defined using base s0 and s1",
                ),
                symbol_decl(
                    s3_def,
                    grounding_defined(formula(formula_pred(s2_def, term(term_var(0, sort("U")))))),
                    "s3 defined using s2",
                ),
            ],
        ),
    )

    # 8. Negative: format and CBOR tags
    err("err-bad-format-version", [2, 1, [], []], "Canon:WrongType")
    err("err-bad-vocab-length-3", [1, 1, []], "Canon:WrongType")
    err("err-bad-vocab-length-5", [1, 1, [], [], []], "Canon:WrongType")
    err("err-bad-sort-decl-length", [1, 1, [["U"]], []], "Canon:WrongType")
    err("err-bad-sort-domain-tag", [1, 1, [["U", [2]]], []], "Canon:WrongType")
    err("err-bad-enum-length", [1, 1, [["U", [1, []]]], []], "Canon:WrongType")
    err(
        "err-bad-source-tag",
        [1, 1, [["U", [1, [], [2, fact_hash]]]], []],
        "Canon:WrongType",
    )
    err(
        "err-bad-source-ref-length-31",
        [1, 1, [["U", [1, [], [0, bytes.fromhex("11" * 31)]]]], []],
        "Canon:WrongType",
    )
    err(
        "err-bad-source-ref-length-33",
        [1, 1, [["U", [1, [], [1, bytes.fromhex("22" * 33)]]]], []],
        "Canon:WrongType",
    )
    err("err-bad-symbol-decl-length", [1, 1, [], [["S", [0]]]], "Canon:WrongType")
    err(
        "err-bad-grounding-tag",
        [1, 1, [], [[p_human, [3, "bad"], "gloss"]]],
        "Canon:WrongType",
    )
    err(
        "err-bad-anchored-length",
        [1, 1, [], [[p_human, [1, "agent"], "gloss"]]],
        "Canon:WrongType",
    )

    # 9. Negative: duplicate names
    err(
        "err-duplicate-sort",
        vocabulary(1, [sort_decl_open("U"), sort_decl_open("U")], []),
        "DuplicateSort",
    )
    err(
        "err-duplicate-symbol",
        vocabulary(
            1,
            [sort_decl_open("U")],
            [
                symbol_decl(p_human, grounding_glossed("Human 1"), "g1"),
                symbol_decl(p_human, grounding_glossed("Human 2"), "g2"),
            ],
        ),
        "DuplicateSymbol",
    )

    # 10. Negative: unknown sort
    p_ghost_arity = symbol("p_ghost_arity", 2, [sort("Ghost")], bool_result())
    err(
        "err-unknown-sort-arity",
        vocabulary(
            1,
            [],
            [symbol_decl(p_ghost_arity, grounding_glossed("ghost arity"), "g")],
        ),
        "UnknownSort",
    )

    f_ghost_result = symbol("f_ghost_res", 1, [], sort_result("Ghost"))
    err(
        "err-unknown-sort-result",
        vocabulary(
            1,
            [],
            [symbol_decl(f_ghost_result, grounding_glossed("ghost res"), "g")],
        ),
        "UnknownSort",
    )

    p_ghost_var = symbol("p_ghost_var", 2, [], bool_result())
    err(
        "err-unknown-sort-var",
        vocabulary(
            1,
            [],
            [
                symbol_decl(
                    p_ghost_var,
                    grounding_defined(
                        formula(
                            formula_eq(
                                term(term_var(0, sort("Ghost"))),
                                term(term_var(0, sort("Ghost"))),
                            )
                        )
                    ),
                    "ghost sort in var",
                )
            ],
        ),
        "UnknownSort",
    )

    p_ghost_binder = symbol("p_ghost_binder", 2, [], bool_result())
    err(
        "err-unknown-sort-binder",
        vocabulary(
            1,
            [],
            [
                symbol_decl(
                    p_ghost_binder,
                    grounding_defined(formula(formula_forall(sort("Ghost")), formula_true())),
                    "ghost sort in forall",
                )
            ],
        ),
        "UnknownSort",
    )

    # 11. Negative: unknown symbol & signature mismatch
    c_undeclared = symbol("UndeclaredConst", 0, [], sort_result("Status"))
    err(
        "err-unknown-symbol-enum",
        vocabulary(
            1,
            [sort_decl_enum("Status", [c_undeclared], fact_ref(fact_hash))],
            [],
        ),
        "UnknownSymbol",
    )

    err(
        "err-unknown-symbol-formula",
        vocabulary(
            1,
            [],
            [
                symbol_decl(
                    p_top,
                    grounding_defined(formula(formula_pred(d0))),
                    "d0 not declared",
                )
            ],
        ),
        "UnknownSymbol",
    )

    c_active_wrong_res = symbol("Active", 0, [], sort_result("OtherSort"))
    err(
        "err-signature-mismatch-enum",
        vocabulary(
            1,
            [
                sort_decl_open("OtherSort"),
                sort_decl_enum("Status", [c_active_wrong_res], fact_ref(fact_hash)),
            ],
            [
                symbol_decl(c_active, grounding_glossed("Active in Status"), "g1"),
            ],
        ),
        "SignatureMismatch",
    )

    d0_wrong_arity = symbol("d0", 2, [sort("U")], bool_result())
    err(
        "err-signature-mismatch-formula",
        vocabulary(
            1,
            [sort_decl_open("U")],
            [
                symbol_decl(d0, grounding_defined(formula(formula_true())), "d0 nullary"),
                symbol_decl(
                    d1,
                    grounding_defined(
                        formula(formula_pred(d0_wrong_arity, term(term_var(0, sort("U")))))
                    ),
                    "d0 used with wrong arity",
                ),
            ],
        ),
        "SignatureMismatch",
    )

    f_app = symbol("f_app", 1, [], sort_result("U"))
    f_app_wrong = symbol("f_app", 1, [sort("U")], sort_result("U"))
    err(
        "err-signature-mismatch-term",
        vocabulary(
            1,
            [sort_decl_open("U")],
            [
                symbol_decl(f_app, grounding_glossed("f nullary"), "g"),
                symbol_decl(
                    p_top,
                    grounding_defined(
                        formula(
                            formula_eq(
                                term(term_app(f_app_wrong, 1), term_var(0, sort("U"))),
                                term(term_var(0, sort("U"))),
                            )
                        )
                    ),
                    "f used with arity 1 in term app",
                ),
            ],
        ),
        "SignatureMismatch",
    )

    # 12. Negative: invalid domain constants
    f_not_const = symbol("f_func", 1, [], sort_result("Status"))
    err(
        "err-invalid-domain-constant-func",
        vocabulary(
            1,
            [sort_decl_enum("Status", [f_not_const], fact_ref(fact_hash))],
            [symbol_decl(f_not_const, grounding_glossed("func constant"), "g")],
        ),
        "InvalidDomainConstant",
    )

    c_with_arity = symbol("c_with_arity", 0, [sort("U")], sort_result("Status"))
    err(
        "err-invalid-domain-constant-arity",
        vocabulary(
            1,
            [
                sort_decl_open("U"),
                sort_decl_enum("Status", [c_with_arity], fact_ref(fact_hash)),
            ],
            [symbol_decl(c_with_arity, grounding_glossed("const with arity"), "g")],
        ),
        "InvalidDomainConstant",
    )

    c_wrong_sort = symbol("c_wrong_sort", 0, [], sort_result("U"))
    err(
        "err-invalid-domain-constant-sort",
        vocabulary(
            1,
            [
                sort_decl_open("U"),
                sort_decl_enum("Status", [c_wrong_sort], fact_ref(fact_hash)),
            ],
            [symbol_decl(c_wrong_sort, grounding_glossed("wrong sort"), "g")],
        ),
        "InvalidDomainConstant",
    )

    err(
        "err-invalid-domain-constant-duplicate",
        vocabulary(
            1,
            [sort_decl_enum("Status", [c_active, c_active], fact_ref(fact_hash))],
            [symbol_decl(c_active, grounding_glossed("active"), "g")],
        ),
        "InvalidDomainConstant",
    )

    # 13. Negative: circular definitions
    p_self = symbol("p_self", 2, [], bool_result())
    err(
        "err-cycle-self",
        vocabulary(
            1,
            [],
            [
                symbol_decl(
                    p_self,
                    grounding_defined(formula(formula_pred(p_self))),
                    "self cycle",
                )
            ],
        ),
        "CircularDefinition",
    )

    q_mut = symbol("q_mut", 2, [], bool_result())
    p_mut = symbol("p_mut", 2, [], bool_result())
    err(
        "err-cycle-mutual",
        vocabulary(
            1,
            [],
            [
                symbol_decl(
                    p_mut,
                    grounding_defined(formula(formula_pred(q_mut))),
                    "p depends on q",
                ),
                symbol_decl(
                    q_mut,
                    grounding_defined(formula(formula_pred(p_mut))),
                    "q depends on p",
                ),
            ],
        ),
        "CircularDefinition",
    )

    f_eq1 = symbol("f_eq1", 1, [], sort_result("U"))
    f_eq2 = symbol("f_eq2", 1, [], sort_result("U"))
    err(
        "err-cycle-eq-terms",
        vocabulary(
            1,
            [sort_decl_open("U")],
            [
                symbol_decl(
                    f_eq1,
                    grounding_defined(
                        formula(
                            formula_eq(
                                term(term_app(f_eq2, 0)),
                                term(term_var(0, sort("U"))),
                            )
                        )
                    ),
                    "f_eq1 uses f_eq2 in eq",
                ),
                symbol_decl(
                    f_eq2,
                    grounding_defined(
                        formula(
                            formula_eq(
                                term(term_app(f_eq1, 0)),
                                term(term_var(0, sort("U"))),
                            )
                        )
                    ),
                    "f_eq2 uses f_eq1 in eq",
                ),
            ],
        ),
        "CircularDefinition",
    )

    p_pred1 = symbol("p_pred1", 2, [], bool_result())
    p_base = symbol("p_base", 2, [sort("U")], bool_result())
    err(
        "err-cycle-pred-terms",
        vocabulary(
            1,
            [sort_decl_open("U")],
            [
                symbol_decl(p_base, grounding_glossed("base pred"), "base"),
                symbol_decl(
                    p_pred1,
                    grounding_defined(
                        formula(
                            formula_pred(
                                p_base,
                                term(term_app(f_eq1, 0)),
                            )
                        )
                    ),
                    "pred1 uses f_eq1",
                ),
                symbol_decl(
                    f_eq1,
                    grounding_defined(
                        formula(
                            formula_and(1),
                            formula_pred(p_pred1),
                        )
                    ),
                    "f_eq1 uses pred1",
                ),
            ],
        ),
        "CircularDefinition",
    )

    # 14. Negative: trailing bytes, non-canonical int, air invalid structure
    valid_empty = encode(vocabulary(1, [], []))
    err("err-trailing-bytes", valid_empty + b"\x00", "Canon:TrailingBytes")

    noncanon_int = bytes([0x84, 0x01, 0x18, 0x00, 0x80, 0x80])
    err("err-noncanonical-int", noncanon_int, "Canon:NonMinimalHead")

    err(
        "err-air-invalid-structure",
        vocabulary(
            1,
            [],
            [
                symbol_decl(
                    p_top,
                    grounding_defined([1, 3, LOGIC, []]),
                    "empty formula prefix",
                )
            ],
        ),
        "Air:InvalidStructure",
    )

    return rows


def render_tsv(rows: list[tuple[str, str, str, str, str]]) -> str:
    lines = [
        "# case\ttype\thex\thash\toutcome — vocabulary/v1",
        "# hash = SHA-256(logos/vocab/v1\\0 || hex bytes) for accepted Vocabulary; '-' otherwise",
        "# outcome = ok:<vocabulary descriptor> | err:<VocabularyError or Canon:name>",
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
            print(
                f"error: stale or missing {OUT.relative_to(ROOT)}; "
                "run tools/gen_vocabulary_vectors.py"
            )
            return 1
        print(f"ok: vocabulary vectors up to date ({len(rows)} cases)")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(rendered.encode("utf-8"))
    print(f"written {OUT.relative_to(ROOT)} ({len(rows)} cases)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
