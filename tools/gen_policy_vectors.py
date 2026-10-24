#!/usr/bin/env python3
"""Vetores policy/v1 (7.1, ADR0069): canon, SHA-256 e parser AIR independentes do Rust.

As políticas válidas são construídas a partir de texto AIR pelo parser Python
(`logos_client.air`) e codificadas aqui; as inválidas são bytes canônicos bem formados que a
validação da TCB precisa recusar pelo motivo nomeado. --check confere
spec/vectors/policy/policy.json sem regravar.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "py"))
from logos_client import air, canon  # noqa: E402
from logos_client.canon import Value  # noqa: E402

OUT = ROOT / "spec/vectors/policy/policy.json"
# Vocabulário canônico bruto: contexto e semente do fuzz policy_decode.
VOCABULARY_OUT = ROOT / "spec/vectors/policy/vocabulary.bin"
POLICY_TAG = b"logos/policy/v1\0"
RULE_TAG = b"logos/rule/v1\0"
LEAF_TAG = b"logos/merkle-leaf/v1\0"
NODE_TAG = b"logos/merkle-node/v1\0"
EMPTY_TAG = b"logos/merkle-empty/v1\0"
VOCAB_TAG = b"logos/vocab/v1\0"

VOCABULARY = """
vocab v1 {
  sort Action
  sort Source
  sort Person
  sort Moment
  sort Kind
  const respond: Kind
  const destructive: Kind
  func kind_of(Action): Kind
  func backup_of(Action): Action
  pred Allowed(Action)
  pred says_request(Source, Action)
  pred external(Source)
  pred is_drop(Action)
  pred executed(Action, Moment)
  pred recent(Moment)
  pred person(Person)
  pred signs(Person, Action)
  pred p(Action)
  pred q(Action)
  func drop(Action): Action
  func send(Action): Action
  func wipe(Action): Action
}
"""
CLASSES = {"allow": 0, "deny": 1, "require": 2, "derive": 3}

R1 = (
    "forall x:Action. forall e:Source. kind_of(x) = respond and says_request(e, x) "
    "and external(e) -> Allowed(x)"
)
R7 = (
    "forall x:Action. kind_of(x) = destructive -> "
    "(Allowed(x) <-> exists u:Person. person(u) and signs(u, x))"
)
R9 = (
    "forall x:Action. is_drop(x) and (not exists t:Moment. "
    "(executed(backup_of(x), t) and recent(t))) -> not Allowed(x)"
)


def sha(tag: bytes, *parts: bytes) -> bytes:
    return hashlib.sha256(tag + b"".join(parts)).digest()


def split(n: int) -> int:
    """Maior potência de 2 estritamente menor que n (RFC 9162 §2.1.1)."""
    k = 1
    while k * 2 < n:
        k *= 2
    return k


def mth(leaves: list[bytes]) -> bytes:
    if not leaves:
        return sha(EMPTY_TAG)
    if len(leaves) == 1:
        return leaves[0]
    k = split(len(leaves))
    return sha(NODE_TAG, mth(leaves[:k]), mth(leaves[k:]))


def audit_path(index: int, leaves: list[bytes]) -> list[bytes]:
    if len(leaves) <= 1:
        return []
    k = split(len(leaves))
    if index < k:
        return [*audit_path(index, leaves[:k]), mth(leaves[k:])]
    return [*audit_path(index - k, leaves[k:]), mth(leaves[:k])]


def merkle_section(name: str, version: int, rules: list[tuple[list[Any], str]]) -> dict[str, Any]:
    """Árvore ordenada por (classe, símbolo, h(regra)); chaves declaradas na construção."""
    entries = []
    for value, symbol in rules:
        assert isinstance(value, list)
        rule_hash = sha(RULE_TAG, canon.encode(value))
        entries.append((value[1], symbol.encode(), symbol, rule_hash, str(value[0])))
    entries.sort(key=lambda e: (e[0], e[1], e[3]))
    digests = [
        sha(LEAF_TAG, canon.encode([cls, symbol, rule_hash]))
        for cls, _, symbol, rule_hash, _ in entries
    ]
    leaves = [
        {
            "rule": rule_id,
            "class": cls,
            "symbol": symbol,
            "rule_hash_hex": rule_hash.hex(),
            "path_hex": [h.hex() for h in audit_path(i, digests)],
        }
        for i, (cls, _, symbol, rule_hash, rule_id) in enumerate(entries)
    ]
    keys = sorted(
        {(e[0], e[2]) for e in entries}
        | {(c, s) for c in range(4) for s in ("", "drop", "send", "read", "zzz")}
    )
    absent: list[dict[str, Any]] = []
    for cls, symbol in keys:
        if any((e[0], e[2]) == (cls, symbol) for e in entries):
            continue
        order = (cls, symbol.encode())
        after = sum(1 for e in entries if (e[0], e[1]) < order)
        absent.append(
            {
                "class": cls,
                "symbol": symbol,
                "left": after - 1 if after > 0 else None,
                "right": after if after < len(entries) else None,
            }
        )

    def neighbour(index: int) -> Value:
        cls, _, symbol, rule_hash, _ = entries[index]
        leaf: Value = [cls, symbol, rule_hash]
        inclusion: Value = [index, len(entries), list(audit_path(index, digests))]
        return [[leaf, inclusion]]

    for item in absent:
        left, right = item["left"], item["right"]
        proof: Value = [
            len(entries),
            neighbour(left) if left is not None else [],
            neighbour(right) if right is not None else [],
        ]
        item["proof_canon_hex"] = canon.encode(proof).hex()
    return {
        "policy": name,
        "version": version,
        "root_hex": mth(digests).hex(),
        "size": len(entries),
        "leaves": leaves,
        "absent": absent,
    }


def build() -> dict[str, Any]:
    vocabulary = air.parse_document(VOCABULARY).vocabulary
    vocabulary_bytes = canon.encode(vocabulary)
    vocabulary_hash = hashlib.sha256(VOCAB_TAG + vocabulary_bytes).digest()

    def rule(rule_id: str, cls: str, text: str, comment: str = "") -> Value:
        formula = air.parse_formula(text, vocabulary)
        return [rule_id, CLASSES[cls], formula, comment]

    def policy(version: int, rules: list[Value], vocab_hash: bytes = vocabulary_hash) -> bytes:
        return canon.encode([1, version, vocab_hash, rules])

    def case(case_id: str, data: bytes, **extra: Any) -> dict[str, Any]:
        return {"id": case_id, "policy_canon_hex": data.hex(), **extra}

    valid_rules: dict[str, list[Value]] = {
        "empty": [],
        "example": [
            rule("r1", "allow", R1, "Responder a pedido externo."),
            rule("r7", "require", R7, "Ação destrutiva exige usuário."),
            rule("r9", "deny", R9),
        ],
        "derive-chain": [
            rule("a1", "allow", "forall x:Action. q(x) -> Allowed(x)"),
            rule("d1", "derive", "forall x:Action. p(x) -> q(x)"),
            rule("d2", "derive", "forall x:Action. q(x) -> p(x)"),
        ],
        "actions": [
            rule("a1", "allow", "forall x:Action. p(x) -> Allowed(drop(x))"),
            rule("a2", "allow", "forall x:Action. p(x) -> Allowed(send(x))"),
            rule("a3", "allow", "forall x:Action. q(x) -> Allowed(x)"),
            rule("d1", "deny", "forall x:Action. q(x) -> not Allowed(wipe(x))"),
            rule("d2", "deny", "forall x:Action. p(x) -> not Allowed(wipe(x))"),
            rule("e1", "derive", "forall x:Action. p(x) -> q(x)"),
            rule("q1", "require", "forall x:Action. p(x) -> Allowed(x)"),
        ],
    }
    # Símbolo da chave de indexação de cada regra, declarado aqui por construção.
    key_symbols: dict[str, dict[str, str]] = {
        "empty": {},
        "example": {"r1": "", "r7": "", "r9": ""},
        "derive-chain": {"a1": "", "d1": "q", "d2": "p"},
        "actions": {
            "a1": "drop",
            "a2": "send",
            "a3": "",
            "d1": "wipe",
            "d2": "wipe",
            "e1": "q",
            "q1": "",
        },
    }
    merkle = [
        merkle_section(
            name,
            12,
            [(r, key_symbols[name][str(r[0])]) for r in rules if isinstance(r, list)],
        )
        for name, rules in valid_rules.items()
    ]
    valid = []
    for name, rules in valid_rules.items():
        data = policy(12, rules)
        valid.append(
            case(
                name,
                data,
                policy_hash_hex=hashlib.sha256(POLICY_TAG + data).hexdigest(),
                rules=[r[0] for r in rules if isinstance(r, list)],
            )
        )

    bad_head = rule("h1", "derive", "forall x:Action. p(x) -> Allowed(x)")
    invalid: list[dict[str, Any]] = [
        case(
            "unsorted",
            policy(
                1,
                [
                    rule("b", "derive", "forall x:Action. p(x)"),
                    rule("a", "derive", "forall x:Action. p(x)"),
                ],
            ),
            reason="UnsortedRules",
        ),
        case(
            "duplicate",
            policy(
                1,
                [
                    rule("a", "derive", "forall x:Action. p(x)"),
                    rule("a", "derive", "forall x:Action. p(x)"),
                ],
            ),
            reason="DuplicateRule",
        ),
        case("version-0", policy(0, valid_rules["example"]), reason="ZeroVersion"),
        case(
            "empty-id", policy(1, [rule("", "derive", "forall x:Action. p(x)")]), reason="BadRuleId"
        ),
        case(
            "wrong-vocabulary",
            policy(1, valid_rules["example"], b"\x00" * 32),
            reason="VocabularyMismatch",
        ),
        case("derive-defines-allowed", policy(1, [bad_head]), reason="BadHead"),
        case(
            "allow-or-body",
            policy(1, [rule("a", "allow", "forall x:Action. (p(x) or q(x)) -> Allowed(x)")]),
            reason="NotHorn",
        ),
        case(
            "allow-nested-negation",
            policy(1, [rule("a", "allow", "forall x:Action. (not not p(x)) -> Allowed(x)")]),
            reason="NotHorn",
        ),
        case(
            "require-without-allowed",
            policy(1, [rule("q", "require", "forall x:Action. p(x) -> q(x)")]),
            reason="RequireWithoutAllowed",
        ),
        case(
            "negation-cycle",
            policy(
                1,
                [
                    rule("d1", "derive", "forall x:Action. (not q(x)) -> p(x)"),
                    rule("d2", "derive", "forall x:Action. p(x) -> q(x)"),
                ],
            ),
            reason="NegativeCycle",
        ),
        case(
            "deny-depends-on-allowed",
            policy(
                1,
                [
                    rule("a1", "allow", "forall x:Action. p(x) -> Allowed(x)"),
                    rule(
                        "d1",
                        "deny",
                        "forall x:Action. forall y:Action. Allowed(y) and q(x) -> not Allowed(x)",
                    ),
                ],
            ),
            reason="NegativeCycle",
        ),
        case(
            "class-9",
            canon.encode(
                [
                    1,
                    1,
                    vocabulary_hash,
                    [["a", 9, air.parse_formula("forall x:Action. p(x)", vocabulary), ""]],
                ]
            ),
            reason="Canon",
        ),
        case("format-version-2", canon.encode([2, 1, vocabulary_hash, []]), reason="Canon"),
        case("arity-3", canon.encode([1, 1, vocabulary_hash]), reason="Canon"),
    ]
    return {
        "format": "policy/v1",
        "hash_tag": POLICY_TAG.decode("ascii").rstrip("\0"),
        "vocabulary_surface": VOCABULARY,
        "vocabulary_canon_hex": vocabulary_bytes.hex(),
        "vocabulary_hash_hex": vocabulary_hash.hex(),
        "valid": valid,
        "invalid": invalid,
        "merkle": merkle,
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    built = build()
    rendered = json.dumps(built, ensure_ascii=False, indent=2) + "\n"
    vocabulary = bytes.fromhex(built["vocabulary_canon_hex"])
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            raise SystemExit("policy vectors differ; run tools/gen_policy_vectors.py")
        if not VOCABULARY_OUT.exists() or VOCABULARY_OUT.read_bytes() != vocabulary:
            raise SystemExit("policy vocabulary differs; run tools/gen_policy_vectors.py")
        print("ok: policy vectors up to date")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(rendered, encoding="utf-8")
        VOCABULARY_OUT.write_bytes(vocabulary)
        print(f"wrote {OUT.relative_to(ROOT)} and {VOCABULARY_OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
