#!/usr/bin/env python3
"""Generate deterministic obligation/v1 test vectors using only Python standard library.

No LOGOS Rust crates are imported. Run without arguments to write:
  spec/vectors/obligation/vectors.json
Run with --check to compare the generated output against the file on disk.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

type WireArray = list[Any]

ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = ROOT / "spec/vectors/obligation"
OUT_JSON = OUT_DIR / "vectors.json"
SCHEMA_JSON = OUT_DIR / "schema.json"

LOGIC = "FOL-ms-eq/v1"
VOCAB_TAG = b"logos/vocab/v1\0"
OBLIGATION_TAG = b"logos/obligation/v1\0"


def head(major: int, argument: int) -> bytes:
    """Shortest definite CBOR header for unsigned argument."""
    if not 0 <= argument < 1 << 64:
        raise ValueError("CBOR argument outside u64")
    if argument < 24:
        return bytes([(major << 5) | argument])
    for width, additional in ((1, 24), (2, 25), (4, 26), (8, 27)):
        if argument < 1 << (8 * width):
            return bytes([(major << 5) | additional]) + argument.to_bytes(width, "big")
    raise ValueError("unreachable CBOR width")


def encode(value: object) -> bytes:
    """Minimal independent CBOR encoder for the fixture subset."""
    if value is None:
        return b"\xf6"
    if isinstance(value, bool):
        return b"\xf5" if value else b"\xf4"
    if isinstance(value, int):
        return head(0, value) if value >= 0 else head(1, -1 - value)
    if isinstance(value, str):
        raw = value.encode("utf-8")
        return head(3, len(raw)) + raw
    if isinstance(value, (bytes, bytearray)):
        return head(2, len(value)) + bytes(value)
    if isinstance(value, list):
        return head(4, len(value)) + b"".join(encode(item) for item in value)
    raise TypeError(f"unsupported fixture value {type(value)}")


# Vocabulary AST builders
def sort(name: str) -> WireArray:
    return [1, 0, name]


def sort_decl_open(name: str) -> WireArray:
    return [name, [0]]


def symbol(name: str, kind: int, arity: list[WireArray], result: WireArray) -> WireArray:
    return [1, 1, name, kind, arity, result]


def symbol_decl(sym: WireArray, grounding: WireArray, gloss: str) -> WireArray:
    return [sym, grounding, gloss]


def grounding_glossed(text: str) -> WireArray:
    return [2, text]


def vocabulary(version: int, sorts: list[WireArray], symbols: list[WireArray]) -> WireArray:
    return [1, version, sorts, symbols]


# Formula & Term AST builders
def term(*nodes: WireArray) -> WireArray:
    return [1, 2, LOGIC, list(nodes)]


def term_const(sym: WireArray) -> WireArray:
    return term([1, sym, 0])


def term_var(index: int, s: WireArray) -> WireArray:
    return term([0, index, s])


def formula(*nodes: WireArray) -> WireArray:
    return [1, 3, LOGIC, list(nodes)]


# Hash helpers
def h_vocab(v_wire: WireArray) -> bytes:
    return hashlib.sha256(VOCAB_TAG + encode(v_wire)).digest()


def h_obligation(ob_wire: WireArray) -> bytes:
    return hashlib.sha256(OBLIGATION_TAG + encode(ob_wire)).digest()


def make_ref(byte_val: int) -> bytes:
    return bytes([byte_val] * 32)


def make_vector(
    vector_id: str,
    description: str,
    epoch: int,
    target_vocab_wire: WireArray,
    goal_wire: WireArray,
    cites: list[bytes],
    premises_raw: list[tuple[bytes, str, int, bool, WireArray | None, WireArray]],
) -> dict[str, Any]:
    """Build a complete single obligation vector fixture."""
    v_target_bytes = encode(target_vocab_wire)
    v_target_hash = h_vocab(target_vocab_wire)
    goal_bytes = encode(goal_wire)

    # Map raw premises by reference
    premise_map: dict[bytes, tuple[str, int, bool, WireArray | None, WireArray]] = {}
    for ref, kind, ep, ret, orig_v, f_wire in premises_raw:
        premise_map[ref] = (kind, ep, ret, orig_v, f_wire)

    # Sort & deduplicate citations
    sorted_unique_refs = sorted(set(cites))

    canonical_premise_pairs: list[WireArray] = []
    canonical_premises_expected: list[dict[str, str]] = []

    for ref in sorted_unique_refs:
        _, _, _, _, f_wire = premise_map[ref]
        canonical_premise_pairs.append([ref, f_wire])
        canonical_premises_expected.append(
            {
                "reference": ref.hex(),
                "formula_hex": encode(f_wire).hex(),
            }
        )

    # Canonical obligation envelope:
    # [1, 0, "FOL-ms-eq/v1", 1, vocab_digest32, premises, goal]
    ob_wire = [1, 0, LOGIC, 1, v_target_hash, canonical_premise_pairs, goal_wire]
    ob_bytes = encode(ob_wire)
    ob_hash = h_obligation(ob_wire)

    premises_json = []
    for ref, kind, ep, ret, orig_v, f_wire in premises_raw:
        orig_hex = encode(orig_v).hex() if orig_v is not None else None
        premises_json.append(
            {
                "reference": ref.hex(),
                "kind": kind,
                "epoch": ep,
                "retracted": ret,
                "origin_vocabulary_hex": orig_hex,
                "formula_hex": encode(f_wire).hex(),
            }
        )

    return {
        "id": vector_id,
        "description": description,
        "epoch": epoch,
        "target_vocabulary_hex": v_target_bytes.hex(),
        "goal_hex": goal_bytes.hex(),
        "cites": [c.hex() for c in cites],
        "premises": premises_json,
        "expected": {
            "canonical_hex": ob_bytes.hex(),
            "obligation_hash": ob_hash.hex(),
            "vocab_hash": v_target_hash.hex(),
            "canonical_premises": canonical_premises_expected,
        },
    }


def build_all_vectors() -> list[dict[str, Any]]:
    vectors: list[dict[str, Any]] = []

    # 1. Base case: empty premises tautology
    s_item = sort("Item")
    sym_c = symbol("c", 0, [], [0, s_item])
    sym_p = symbol("P", 2, [s_item], [1])
    v1 = vocabulary(
        1,
        [sort_decl_open("Item")],
        [
            symbol_decl(sym_c, grounding_glossed("item c"), "Constante c"),
            symbol_decl(sym_p, grounding_glossed("pred P"), "Predicado P"),
        ],
    )
    t_c = term_const(sym_c)
    f_p_c_node = [3, sym_p, [t_c]]
    f1_goal = formula([7], f_p_c_node, f_p_c_node)
    vectors.append(
        make_vector(
            "obligation-01-empty-premises-tautology",
            "Tautological goal under sort Item with no cited premises; base envelope case",
            1,
            v1,
            f1_goal,
            [],
            [],
        )
    )

    # 2. Single Fact premise with equality between constants
    sym_a = symbol("a", 0, [], [0, s_item])
    sym_b = symbol("b", 0, [], [0, s_item])
    v2 = vocabulary(
        1,
        [sort_decl_open("Item")],
        [
            symbol_decl(sym_a, grounding_glossed("item a"), "Constante a"),
            symbol_decl(sym_b, grounding_glossed("item b"), "Constante b"),
        ],
    )
    t_a = term_const(sym_a)
    t_b = term_const(sym_b)
    f2 = formula([2, t_a, t_b])
    ref2 = make_ref(0x11)
    vectors.append(
        make_vector(
            "obligation-02-single-fact-constant-eq",
            "Single Fact premise with equality between constants",
            1,
            v2,
            f2,
            [ref2],
            [(ref2, "Fact", 1, False, None, f2)],
        )
    )

    # 3. Single Policy premise asserting authorization predicate
    s_agent = sort("Agent")
    sym_alice = symbol("alice", 0, [], [0, s_agent])
    sym_auth = symbol("Authorized", 2, [s_agent], [1])
    v3 = vocabulary(
        1,
        [sort_decl_open("Agent")],
        [
            symbol_decl(sym_alice, grounding_glossed("alice agent"), "Constante alice"),
            symbol_decl(sym_auth, grounding_glossed("authorized pred"), "Predicado Authorized"),
        ],
    )
    t_alice = term_const(sym_alice)
    f3 = formula([3, sym_auth, [t_alice]])
    ref3 = make_ref(0x22)
    vectors.append(
        make_vector(
            "obligation-03-single-policy-implication",
            "Single Policy premise asserting authorization predicate",
            1,
            v3,
            f3,
            [ref3],
            [(ref3, "Policy", 1, False, None, f3)],
        )
    )

    # 4. Single Verified premise asserting safe state
    s_state = sort("State")
    sym_s0 = symbol("s0", 0, [], [0, s_state])
    sym_safe = symbol("Safe", 2, [s_state], [1])
    v4 = vocabulary(
        1,
        [sort_decl_open("State")],
        [
            symbol_decl(sym_s0, grounding_glossed("initial state"), "Estado inicial s0"),
            symbol_decl(sym_safe, grounding_glossed("safety property"), "Predicado Safe"),
        ],
    )
    t_s0 = term_const(sym_s0)
    f4 = formula([3, sym_safe, [t_s0]])
    ref4 = make_ref(0x33)
    vectors.append(
        make_vector(
            "obligation-04-single-verified-predicate",
            "Single Verified premise asserting safe state",
            2,
            v4,
            f4,
            [ref4],
            [(ref4, "Verified", 2, False, None, f4)],
        )
    )

    # 5. Single Assumption premise preserving explicit antecedent in Gamma
    s_res = sort("Resource")
    sym_r1 = symbol("r1", 0, [], [0, s_res])
    sym_act = symbol("Active", 2, [s_res], [1])
    v5 = vocabulary(
        1,
        [sort_decl_open("Resource")],
        [
            symbol_decl(sym_r1, grounding_glossed("resource 1"), "Recurso r1"),
            symbol_decl(sym_act, grounding_glossed("active pred"), "Predicado Active"),
        ],
    )
    t_r1 = term_const(sym_r1)
    f5 = formula([3, sym_act, [t_r1]])
    ref5 = make_ref(0x44)
    vectors.append(
        make_vector(
            "obligation-05-single-assumption-hypothesis",
            "Single Assumption premise preserving explicit antecedent in Gamma",
            5,
            v5,
            f5,
            [ref5],
            [(ref5, "Assumption", 5, False, None, f5)],
        )
    )

    # 6. Conjunction combining all four citable kinds: Fact, Policy, Verified, Assumption
    s_node = sort("Node")
    sym_n1 = symbol("n1", 0, [], [0, s_node])
    sym_n2 = symbol("n2", 0, [], [0, s_node])
    sym_p6 = symbol("P", 2, [s_node], [1])
    sym_q6 = symbol("Q", 2, [s_node], [1])
    sym_r6 = symbol("R", 2, [s_node], [1])
    sym_s6 = symbol("S", 2, [s_node], [1])
    v6 = vocabulary(
        1,
        [sort_decl_open("Node")],
        [
            symbol_decl(sym_n1, grounding_glossed("n1"), "No 1"),
            symbol_decl(sym_n2, grounding_glossed("n2"), "No 2"),
            symbol_decl(sym_p6, grounding_glossed("P"), "Predicado P"),
            symbol_decl(sym_q6, grounding_glossed("Q"), "Predicado Q"),
            symbol_decl(sym_r6, grounding_glossed("R"), "Predicado R"),
            symbol_decl(sym_s6, grounding_glossed("S"), "Predicado S"),
        ],
    )
    t_n1 = term_const(sym_n1)
    t_n2 = term_const(sym_n2)
    f6_p = formula([3, sym_p6, [t_n1]])
    f6_q = formula([3, sym_q6, [t_n1]])
    f6_r = formula([3, sym_r6, [t_n2]])
    f6_s = formula([3, sym_s6, [t_n2]])
    f6_goal = formula(
        [5, 4],
        [3, sym_p6, [t_n1]],
        [3, sym_q6, [t_n1]],
        [3, sym_r6, [t_n2]],
        [3, sym_s6, [t_n2]],
    )
    ref6_a = make_ref(0x0A)
    ref6_b = make_ref(0x0B)
    ref6_c = make_ref(0x0C)
    ref6_d = make_ref(0x0D)
    vectors.append(
        make_vector(
            "obligation-06-all-four-citable-kinds",
            "Conjunction combining all four citable kinds: Fact, Policy, Verified, Assumption",
            1,
            v6,
            f6_goal,
            [ref6_a, ref6_b, ref6_c, ref6_d],
            [
                (ref6_a, "Fact", 1, False, None, f6_p),
                (ref6_b, "Policy", 1, False, None, f6_q),
                (ref6_c, "Verified", 1, False, None, f6_r),
                (ref6_d, "Assumption", 1, False, None, f6_s),
            ],
        )
    )

    # 7. Proposal citing the same premise reference three times; deduplicated to single entry
    s_nat = sort("Nat")
    sym_zero = symbol("zero", 0, [], [0, s_nat])
    sym_is_zero = symbol("IsZero", 2, [s_nat], [1])
    v7 = vocabulary(
        1,
        [sort_decl_open("Nat")],
        [
            symbol_decl(sym_zero, grounding_glossed("zero"), "Constante zero"),
            symbol_decl(sym_is_zero, grounding_glossed("is zero"), "Predicado IsZero"),
        ],
    )
    t_zero = term_const(sym_zero)
    f7 = formula([3, sym_is_zero, [t_zero]])
    ref7 = make_ref(0x55)
    vectors.append(
        make_vector(
            "obligation-07-citation-deduplication",
            "Proposal citing the same premise reference three times; deduplicated to single entry",
            1,
            v7,
            f7,
            [ref7, ref7, ref7],
            [(ref7, "Fact", 1, False, None, f7)],
        )
    )

    # 8. Proposal citing three distinct premises in reverse order; sorted by 32-byte digest
    s_t = sort("T")
    sym_c1 = symbol("c1", 0, [], [0, s_t])
    sym_c2 = symbol("c2", 0, [], [0, s_t])
    sym_c3 = symbol("c3", 0, [], [0, s_t])
    sym_valid_t = symbol("Valid", 2, [s_t], [1])
    v8 = vocabulary(
        1,
        [sort_decl_open("T")],
        [
            symbol_decl(sym_c1, grounding_glossed("c1"), "Constante c1"),
            symbol_decl(sym_c2, grounding_glossed("c2"), "Constante c2"),
            symbol_decl(sym_c3, grounding_glossed("c3"), "Constante c3"),
            symbol_decl(sym_valid_t, grounding_glossed("valid"), "Predicado Valid"),
        ],
    )
    t_c1 = term_const(sym_c1)
    t_c2 = term_const(sym_c2)
    t_c3 = term_const(sym_c3)
    f8_1 = formula([3, sym_valid_t, [t_c1]])
    f8_2 = formula([3, sym_valid_t, [t_c2]])
    f8_3 = formula([3, sym_valid_t, [t_c3]])
    ref8_1 = make_ref(0x10)
    ref8_2 = make_ref(0x50)
    ref8_3 = make_ref(0x90)
    vectors.append(
        make_vector(
            "obligation-08-citation-permutation-order",
            "Proposal citing three distinct premises in reverse order; sorted by 32-byte digest",
            1,
            v8,
            f8_1,
            [ref8_3, ref8_1, ref8_2],
            [
                (ref8_1, "Fact", 1, False, None, f8_1),
                (ref8_2, "Fact", 1, False, None, f8_2),
                (ref8_3, "Fact", 1, False, None, f8_3),
            ],
        )
    )

    # 9. Proposal citations combine arbitrary permutation and repeated references
    s_elem = sort("Elem")
    sym_e1 = symbol("e1", 0, [], [0, s_elem])
    sym_e2 = symbol("e2", 0, [], [0, s_elem])
    sym_marked = symbol("Marked", 2, [s_elem], [1])
    v9 = vocabulary(
        1,
        [sort_decl_open("Elem")],
        [
            symbol_decl(sym_e1, grounding_glossed("e1"), "Constante e1"),
            symbol_decl(sym_e2, grounding_glossed("e2"), "Constante e2"),
            symbol_decl(sym_marked, grounding_glossed("marked"), "Predicado Marked"),
        ],
    )
    t_e1 = term_const(sym_e1)
    t_e2 = term_const(sym_e2)
    f9_1 = formula([3, sym_marked, [t_e1]])
    f9_2 = formula([3, sym_marked, [t_e2]])
    ref9_1 = make_ref(0x20)
    ref9_2 = make_ref(0x70)
    vectors.append(
        make_vector(
            "obligation-09-permutation-and-dedup-combined",
            "Proposal citations combine arbitrary permutation and repeated references",
            1,
            v9,
            f9_1,
            [ref9_2, ref9_1, ref9_2, ref9_1, ref9_2],
            [
                (ref9_1, "Fact", 1, False, None, f9_1),
                (ref9_2, "Verified", 1, False, None, f9_2),
            ],
        )
    )

    # 10. Epoch invariance baseline: obligation generated at snapshot epoch 1
    s_v = sort("V")
    sym_v = symbol("v", 0, [], [0, s_v])
    sym_ok = symbol("Ok", 2, [s_v], [1])
    v10 = vocabulary(
        1,
        [sort_decl_open("V")],
        [
            symbol_decl(sym_v, grounding_glossed("v"), "Constante v"),
            symbol_decl(sym_ok, grounding_glossed("ok"), "Predicado Ok"),
        ],
    )
    t_v = term_const(sym_v)
    f10 = formula([3, sym_ok, [t_v]])
    ref10 = make_ref(0x60)
    vectors.append(
        make_vector(
            "obligation-10-epoch-invariance-epoch-1",
            "Epoch invariance baseline: obligation generated at snapshot epoch 1",
            1,
            v10,
            f10,
            [ref10],
            [(ref10, "Fact", 1, False, None, f10)],
        )
    )

    # 11. Epoch invariance counterpart: same proposal at epoch 999 produces identical bytes and hash
    vectors.append(
        make_vector(
            "obligation-11-epoch-invariance-epoch-999",
            "Epoch invariance counterpart: same proposal at epoch 999 "
            "produces identical bytes and hash",
            999,
            v10,
            f10,
            [ref10],
            [(ref10, "Fact", 999, False, None, f10)],
        )
    )

    # 12. Two distinct premise references asserting identical formula remain two distinct premises
    s_s = sort("S")
    sym_c12 = symbol("c", 0, [], [0, s_s])
    sym_p12 = symbol("P", 2, [s_s], [1])
    v12 = vocabulary(
        1,
        [sort_decl_open("S")],
        [
            symbol_decl(sym_c12, grounding_glossed("c"), "Constante c"),
            symbol_decl(sym_p12, grounding_glossed("P"), "Predicado P"),
        ],
    )
    t_c12 = term_const(sym_c12)
    f12 = formula([3, sym_p12, [t_c12]])
    ref12_1 = bytes.fromhex("1234" * 16)
    ref12_2 = bytes.fromhex("abcd" * 16)
    vectors.append(
        make_vector(
            "obligation-12-unique-refs-same-formula",
            "Two distinct premise references asserting identical formula "
            "remain two distinct premises in Gamma",
            1,
            v12,
            f12,
            [ref12_1, ref12_2],
            [
                (ref12_1, "Fact", 1, False, None, f12),
                (ref12_2, "Verified", 1, False, None, f12),
            ],
        )
    )

    # 13. Target vocabulary is compatible extension of premise origin vocabulary with changed h(V)
    s_data = sort("Data")
    sym_d0 = symbol("d0", 0, [], [0, s_data])
    sym_d1 = symbol("d1", 0, [], [0, s_data])
    sym_valid_data = symbol("Valid", 2, [s_data], [1])
    v13_orig = vocabulary(
        1,
        [sort_decl_open("Data")],
        [
            symbol_decl(sym_d0, grounding_glossed("d0"), "Constante d0"),
            symbol_decl(sym_valid_data, grounding_glossed("valid data"), "Predicado Valid"),
        ],
    )
    v13_target = vocabulary(
        2,
        [sort_decl_open("Data")],
        [
            symbol_decl(sym_d0, grounding_glossed("d0"), "Constante d0"),
            symbol_decl(sym_d1, grounding_glossed("d1"), "Constante d1"),
            symbol_decl(sym_valid_data, grounding_glossed("valid data"), "Predicado Valid"),
        ],
    )
    t_d0 = term_const(sym_d0)
    t_d1 = term_const(sym_d1)
    f13_p1 = formula([3, sym_valid_data, [t_d0]])
    f13_goal = formula(
        [5, 2],
        [3, sym_valid_data, [t_d0]],
        [3, sym_valid_data, [t_d1]],
    )
    ref13 = make_ref(0x40)
    vectors.append(
        make_vector(
            "obligation-13-compatible-extension-new-symbol",
            "Target vocabulary is compatible extension of premise origin "
            "vocabulary with changed h(V)",
            1,
            v13_target,
            f13_goal,
            [ref13],
            [(ref13, "Fact", 1, False, v13_orig, f13_p1)],
        )
    )

    # 14. Compatible extension adding new sort Role and relation HasRole(User, Role)
    s_user = sort("User")
    s_role = sort("Role")
    sym_u_alice = symbol("alice", 0, [], [0, s_user])
    sym_is_admin = symbol("IsAdmin", 2, [s_user], [1])
    sym_has_role = symbol("HasRole", 2, [s_user, s_role], [1])
    v14_orig = vocabulary(
        1,
        [sort_decl_open("User")],
        [
            symbol_decl(sym_u_alice, grounding_glossed("alice"), "Usuario alice"),
            symbol_decl(sym_is_admin, grounding_glossed("admin check"), "Predicado IsAdmin"),
        ],
    )
    v14_target = vocabulary(
        2,
        [sort_decl_open("User"), sort_decl_open("Role")],
        [
            symbol_decl(sym_u_alice, grounding_glossed("alice"), "Usuario alice"),
            symbol_decl(sym_is_admin, grounding_glossed("admin check"), "Predicado IsAdmin"),
            symbol_decl(sym_has_role, grounding_glossed("role check"), "Predicado HasRole"),
        ],
    )
    t_u_alice = term_const(sym_u_alice)
    f14_p1 = formula([3, sym_is_admin, [t_u_alice]])
    var0_role = term_var(0, s_role)
    f14_goal = formula([10, s_role], [3, sym_has_role, [t_u_alice, var0_role]])
    ref14 = make_ref(0x41)
    vectors.append(
        make_vector(
            "obligation-14-compatible-extension-new-sort-and-pred",
            "Compatible extension adding new sort Role and relation HasRole(User, Role)",
            1,
            v14_target,
            f14_goal,
            [ref14],
            [(ref14, "Verified", 1, False, v14_orig, f14_p1)],
        )
    )

    # 15. First-order universal quantifier with De Bruijn variable bound to outer sort Item
    sym_processed = symbol("Processed", 2, [s_item], [1])
    v15 = vocabulary(
        1,
        [sort_decl_open("Item")],
        [symbol_decl(sym_processed, grounding_glossed("processed"), "Predicado Processed")],
    )
    var0_item = term_var(0, s_item)
    f15 = formula([9, s_item], [3, sym_processed, [var0_item]])
    ref15 = make_ref(0x50)
    vectors.append(
        make_vector(
            "obligation-15-fol-universal-quantifier",
            "First-order universal quantifier with De Bruijn variable bound to outer sort Item",
            1,
            v15,
            f15,
            [ref15],
            [(ref15, "Assumption", 1, False, None, f15)],
        )
    )

    # 16. First-order existential quantifier asserting existence of valid token
    s_tok = sort("Token")
    sym_valid_tok = symbol("ValidToken", 2, [s_tok], [1])
    v16 = vocabulary(
        1,
        [sort_decl_open("Token")],
        [symbol_decl(sym_valid_tok, grounding_glossed("valid token"), "Predicado ValidToken")],
    )
    var0_tok = term_var(0, s_tok)
    f16 = formula([10, s_tok], [3, sym_valid_tok, [var0_tok]])
    ref16 = make_ref(0x52)
    vectors.append(
        make_vector(
            "obligation-16-fol-existential-quantifier",
            "First-order existential quantifier asserting existence of valid token",
            1,
            v16,
            f16,
            [ref16],
            [(ref16, "Verified", 1, False, None, f16)],
        )
    )

    # 17. Function application and term equality over Peano successor and zero
    sym_succ = symbol("succ", 1, [s_nat], [0, s_nat])
    sym_leq = symbol("Leq", 2, [s_nat, s_nat], [1])
    v17 = vocabulary(
        1,
        [sort_decl_open("Nat")],
        [
            symbol_decl(sym_zero, grounding_glossed("zero"), "Constante zero"),
            symbol_decl(sym_succ, grounding_glossed("succ"), "Funcao sucessor"),
            symbol_decl(sym_leq, grounding_glossed("leq"), "Predicado menor ou igual"),
        ],
    )
    t_succ_zero = term([1, sym_succ, 1], [1, sym_zero, 0])
    f17_p1 = formula([2, t_succ_zero, t_succ_zero])
    f17_goal = formula([3, sym_leq, [t_zero, t_succ_zero]])
    ref17 = make_ref(0x53)
    vectors.append(
        make_vector(
            "obligation-17-fol-function-application-and-equality",
            "Function application and term equality over Peano successor and zero",
            1,
            v17,
            f17_goal,
            [ref17],
            [(ref17, "Fact", 1, False, None, f17_p1)],
        )
    )

    # 18. Nested quantifiers Forall x. Exists y. Rel(x, y) with De Bruijn indices var1 and var0
    s_dom = sort("Domain")
    sym_rel = symbol("Rel", 2, [s_dom, s_dom], [1])
    v18 = vocabulary(
        1,
        [sort_decl_open("Domain")],
        [symbol_decl(sym_rel, grounding_glossed("relation"), "Relacao binaria Rel")],
    )
    var1_dom = term_var(1, s_dom)
    var0_dom = term_var(0, s_dom)
    f18 = formula([9, s_dom], [10, s_dom], [3, sym_rel, [var1_dom, var0_dom]])
    ref18 = make_ref(0x54)
    vectors.append(
        make_vector(
            "obligation-18-fol-nested-quantifiers-alpha-scope",
            "Nested quantifiers Forall x. Exists y. Rel(x, y) with De Bruijn indices var1 and var0",
            1,
            v18,
            f18,
            [ref18],
            [(ref18, "Policy", 1, False, None, f18)],
        )
    )

    # 19. Complex Boolean composition combining And, Or, Not, Implies, and Iff connectives
    sym_ta = symbol("a", 0, [], [0, s_t])
    sym_tb = symbol("b", 0, [], [0, s_t])
    sym_tc = symbol("c", 0, [], [0, s_t])
    sym_tp = symbol("P", 2, [s_t], [1])
    sym_tq = symbol("Q", 2, [s_t], [1])
    sym_tr = symbol("R", 2, [s_t], [1])
    v19 = vocabulary(
        1,
        [sort_decl_open("T")],
        [
            symbol_decl(sym_ta, grounding_glossed("a"), "Constante a"),
            symbol_decl(sym_tb, grounding_glossed("b"), "Constante b"),
            symbol_decl(sym_tc, grounding_glossed("c"), "Constante c"),
            symbol_decl(sym_tp, grounding_glossed("P"), "Predicado P"),
            symbol_decl(sym_tq, grounding_glossed("Q"), "Predicado Q"),
            symbol_decl(sym_tr, grounding_glossed("R"), "Predicado R"),
        ],
    )
    t_ta = term_const(sym_ta)
    t_tb = term_const(sym_tb)
    t_tc = term_const(sym_tc)
    f19_pa = [3, sym_tp, [t_ta]]
    f19_qb = [3, sym_tq, [t_tb]]
    f19_rc = [3, sym_tr, [t_tc]]
    # Premise 1: Or(2, [And(2, [P(a), Q(b)]), Not(R(c))])
    f19_p1 = formula([6, 2], [5, 2], f19_pa, f19_qb, [4], f19_rc)
    # Premise 2: Implies(P(a), Iff(Q(b), R(c)))
    f19_p2 = formula([7], f19_pa, [8], f19_qb, f19_rc)
    # Goal: And(2, [Implies(P(a), Q(b)), Implies(Q(b), R(c))])
    f19_goal = formula([5, 2], [7], f19_pa, f19_qb, [7], f19_qb, f19_rc)
    ref19_1 = make_ref(0x61)
    ref19_2 = make_ref(0x62)
    vectors.append(
        make_vector(
            "obligation-19-fol-complex-boolean-structure",
            "Complex Boolean composition combining And, Or, Not, Implies, and Iff connectives",
            1,
            v19,
            f19_goal,
            [ref19_1, ref19_2],
            [
                (ref19_1, "Fact", 1, False, None, f19_p1),
                (ref19_2, "Fact", 1, False, None, f19_p2),
            ],
        )
    )

    # 20. Binary midpoint function, ternary Collinear predicate and symmetry.
    s_pt = sort("Point")
    sym_p1 = symbol("p1", 0, [], [0, s_pt])
    sym_p2 = symbol("p2", 0, [], [0, s_pt])
    sym_p3 = symbol("p3", 0, [], [0, s_pt])
    sym_mid = symbol("midpoint", 1, [s_pt, s_pt], [0, s_pt])
    sym_col = symbol("Collinear", 2, [s_pt, s_pt, s_pt], [1])
    v20 = vocabulary(
        1,
        [sort_decl_open("Point")],
        [
            symbol_decl(sym_p1, grounding_glossed("p1"), "Ponto 1"),
            symbol_decl(sym_p2, grounding_glossed("p2"), "Ponto 2"),
            symbol_decl(sym_p3, grounding_glossed("p3"), "Ponto 3"),
            symbol_decl(sym_mid, grounding_glossed("midpoint"), "Ponto medio"),
            symbol_decl(sym_col, grounding_glossed("collinear"), "Predicado Collinear"),
        ],
    )
    t_p1 = term_const(sym_p1)
    t_p2 = term_const(sym_p2)
    t_p3 = term_const(sym_p3)
    t_mid_12 = term([1, sym_mid, 2], [1, sym_p1, 0], [1, sym_p2, 0])
    t_mid_21 = term([1, sym_mid, 2], [1, sym_p2, 0], [1, sym_p1, 0])
    # Premise 1: Collinear(p1, p2, midpoint(p1, p2))
    f20_p1 = formula([3, sym_col, [t_p1, t_p2, t_mid_12]])
    # Premise 2: Eq(midpoint(p1, p2), midpoint(p2, p1))
    f20_p2 = formula([2, t_mid_12, t_mid_21])
    # Goal: Collinear(p1, p2, p3)
    f20_goal = formula([3, sym_col, [t_p1, t_p2, t_p3]])
    ref20_1 = make_ref(0x71)
    ref20_2 = make_ref(0x72)
    vectors.append(
        make_vector(
            "obligation-20-nontrivial-multi-arg-functions-predicates",
            "Geometric reasoning with binary midpoint function, "
            "ternary Collinear predicate and symmetry",
            1,
            v20,
            f20_goal,
            [ref20_1, ref20_2],
            [
                (ref20_1, "Verified", 1, False, None, f20_p1),
                (ref20_2, "Assumption", 1, False, None, f20_p2),
            ],
        )
    )

    if len(vectors) != 20:
        raise ValueError(f"expected exactly 20 vectors, got {len(vectors)}")

    return vectors


def render_schema() -> str:
    schema = {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "ObligationVectors",
        "description": "Schema for deterministic obligation/v1 generation test vectors",
        "type": "object",
        "required": ["$schema", "version", "description", "vectors"],
        "properties": {
            "$schema": {"type": "string"},
            "version": {"type": "integer", "const": 1},
            "description": {"type": "string"},
            "vectors": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": [
                        "id",
                        "description",
                        "epoch",
                        "target_vocabulary_hex",
                        "goal_hex",
                        "cites",
                        "premises",
                        "expected",
                    ],
                    "properties": {
                        "id": {"type": "string"},
                        "description": {"type": "string"},
                        "epoch": {"type": "integer", "minimum": 0},
                        "target_vocabulary_hex": {"type": "string", "pattern": "^[0-9a-f]+$"},
                        "goal_hex": {"type": "string", "pattern": "^[0-9a-f]+$"},
                        "cites": {
                            "type": "array",
                            "items": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                        },
                        "premises": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "required": [
                                    "reference",
                                    "kind",
                                    "epoch",
                                    "retracted",
                                    "formula_hex",
                                ],
                                "properties": {
                                    "reference": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                                    "kind": {
                                        "type": "string",
                                        "enum": [
                                            "Fact",
                                            "Policy",
                                            "Verified",
                                            "Assumption",
                                            "Pending",
                                            "Refuted",
                                            "Unknown",
                                            "Rejected",
                                            "Invalidated",
                                            "Proposal",
                                            "Suspended",
                                        ],
                                    },
                                    "epoch": {"type": "integer", "minimum": 0},
                                    "retracted": {"type": "boolean"},
                                    "origin_vocabulary_hex": {"type": ["string", "null"]},
                                    "formula_hex": {"type": "string", "pattern": "^[0-9a-f]+$"},
                                },
                            },
                        },
                        "expected": {
                            "type": "object",
                            "required": [
                                "canonical_hex",
                                "obligation_hash",
                                "vocab_hash",
                                "canonical_premises",
                            ],
                            "properties": {
                                "canonical_hex": {"type": "string", "pattern": "^[0-9a-f]+$"},
                                "obligation_hash": {
                                    "type": "string",
                                    "pattern": "^[0-9a-f]{64}$",
                                },
                                "vocab_hash": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                                "canonical_premises": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "required": ["reference", "formula_hex"],
                                        "properties": {
                                            "reference": {
                                                "type": "string",
                                                "pattern": "^[0-9a-f]{64}$",
                                            },
                                            "formula_hex": {
                                                "type": "string",
                                                "pattern": "^[0-9a-f]+$",
                                            },
                                        },
                                    },
                                },
                            },
                        },
                    },
                },
            },
        },
    }
    return json.dumps(schema, indent=2) + "\n"


def render_vectors_json(vectors: list[dict[str, Any]]) -> str:
    payload = {
        "$schema": "./schema.json",
        "version": 1,
        "description": "Deterministic test vectors for obligation/v1 generation (Etapa 1.5)",
        "vectors": vectors,
    }
    return json.dumps(payload, indent=2) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check",
        action="store_true",
        help="compare the generated JSON against spec/vectors/obligation/vectors.json",
    )
    args = parser.parse_args()

    vectors = build_all_vectors()
    rendered_json = render_vectors_json(vectors)
    rendered_schema = render_schema()

    if args.check:
        if not OUT_JSON.exists():
            print(
                f"error: missing {OUT_JSON.relative_to(ROOT)}; run tools/gen_obligation_vectors.py"
            )
            return 1
        current_bytes = OUT_JSON.read_bytes()
        if current_bytes != rendered_json.encode("utf-8"):
            print(f"error: stale {OUT_JSON.relative_to(ROOT)}; run tools/gen_obligation_vectors.py")
            return 1
        if not SCHEMA_JSON.exists() or SCHEMA_JSON.read_bytes() != rendered_schema.encode("utf-8"):
            print(
                f"error: stale or missing {SCHEMA_JSON.relative_to(ROOT)}; "
                "run tools/gen_obligation_vectors.py"
            )
            return 1
        print(f"ok: obligation vectors up to date ({len(vectors)} cases)")
        return 0

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_bytes(rendered_json.encode("utf-8"))
    SCHEMA_JSON.write_bytes(rendered_schema.encode("utf-8"))
    print(f"written {OUT_JSON.relative_to(ROOT)} ({len(vectors)} cases)")
    print(f"written {SCHEMA_JSON.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
