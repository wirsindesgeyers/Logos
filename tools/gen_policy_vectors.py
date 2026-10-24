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
    }
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
