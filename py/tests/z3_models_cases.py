"""Fixtures não autoritativas e ponte Rust da 2.4; AST/oráculo reutilizados da 2.2."""

from __future__ import annotations

import hashlib
import random
import subprocess
from dataclasses import dataclass
from pathlib import Path

from logos_client import air, canon
from tests import test_model_differential as differential

ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class Case:
    vocabulary: bytes
    obligation: bytes
    expected_hash: bytes
    model: bytes


def obligation(vocabulary: bytes, premises: list[canon.Value], goal: canon.Value) -> bytes:
    """Fixture wire; não autentica eventos nem substitui generate do monitor."""
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + vocabulary).digest()
    entries: list[canon.Value] = [
        [(index + 1).to_bytes(32, "big"), formula] for index, formula in enumerate(premises)
    ]
    return canon.encode([1, 0, air.LOGIC_ID, 1, h_v, entries, goal])


def case(vocabulary: bytes, o_bytes: bytes, model: bytes) -> Case:
    return Case(
        vocabulary, o_bytes, hashlib.sha256(b"logos/obligation/v1\x00" + o_bytes).digest(), model
    )


def document_case(source: str) -> tuple[bytes, bytes]:
    """Último claim é goal, claims anteriores são premissas."""
    document = air.parse_document(source)
    v_bytes = canon.encode(document.vocabulary)
    o_bytes = obligation(
        v_bytes, [claim.formula for claim in document.claims[:-1]], document.claims[-1].formula
    )
    return o_bytes, v_bytes


def check(cases: list[tuple[str, Case]]) -> list[str]:
    rows = [
        f"{mode} {item.vocabulary.hex()} {item.obligation.hex()} "
        f"{item.expected_hash.hex()} {item.model.hex()}"
        for mode, item in cases
    ]
    result = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "logos-model", "--example", "checker_diff", "--"],
        input="\n".join(rows) + "\n",
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
        timeout=300,
    )
    outcomes = result.stdout.splitlines()
    assert len(outcomes) == len(cases), result.stderr
    assert all(outcome in ("Accepted", "Rejected", "Unknown") for outcome in outcomes), (
        result.stderr
    )
    return outcomes


def corpus_200() -> list[Case]:
    """200 fórmulas/contextos distintos, sem repetir templates mudando só event IDs.

    Modelo completo conhecido e oráculo dev independente orientam Γ/goal; o
    checker real deve aceitar todos os controles antes de medir o produtor.
    """
    rng = random.Random(20261006)
    cases: list[Case] = []
    seen: set[bytes] = set()
    while len(cases) < 200:
        model = differential.generate_model(rng, len(cases))
        oracle = differential.Z3Oracle(model)
        v_bytes = canon.encode(differential.vocab_to_canon(model.vocab))
        goal = differential.generate_formula(rng, model.vocab, depth=3, binders=[])
        if oracle.eval_formula(goal):
            goal = differential.NotFormula(goal)
        premises: list[canon.Value] = []
        for _ in range(len(cases) % 3):
            formula = differential.generate_formula(rng, model.vocab, depth=3, binders=[])
            if not oracle.eval_formula(formula):
                formula = differential.NotFormula(formula)
            premises.append(differential.formula_to_canon(formula))
        o_bytes = obligation(v_bytes, premises, differential.formula_to_canon(goal))
        identity = v_bytes + o_bytes  # event IDs are fixed by index, not corpus case name.
        if identity in seen:
            continue
        seen.add(identity)
        cases.append(case(v_bytes, o_bytes, canon.encode(differential.model_to_canon(model))))
    return cases


def fake_z3_10k() -> tuple[list[Case], list[str]]:
    """Simula bytes devolvidos por Z3 falso na fronteira candidato → checker.

    Oráculo é apenas P[f(c,d)] and c!=d and not Q[c,f(d,c)], calculado por
    índices inteiros/sets independentes. Inclui 1000 controles positivos,
    7000 valorações aleatórias e 2000 defeitos estruturais isolados.
    """
    document = air.parse_document(
        "vocab v1 { sort S; const c:S; const d:S; func f(S,S):S; pred P(S); pred Q(S,S); } "
        "claim H: P(f(c,d)) and not (c=d); claim G: Q(c,f(d,c));"
    )
    v_bytes = canon.encode(document.vocabulary)
    o_bytes = obligation(v_bytes, [document.claims[0].formula], document.claims[1].formula)
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + v_bytes).digest()
    rng = random.Random(20261005)
    cases: list[Case] = []
    expected: list[str] = []
    for index in range(10_000):
        size = rng.randint(2, 4)
        c, d = rng.randrange(size), rng.randrange(size)
        outputs = [rng.randrange(size) for _ in range(size * size)]
        p_values = {value for value in range(size) if rng.getrandbits(1)}
        q_values = {
            (left, right) for left in range(size) for right in range(size) if rng.getrandbits(1)
        }
        if index < 1000 or index >= 8000:
            c, d = 0, 1
            p_values.add(outputs[size * c + d])
            q_values.discard((c, outputs[size * d + c]))
        legitimate = (
            c != d
            and outputs[size * c + d] in p_values
            and (c, outputs[size * d + c]) not in q_values
        )
        consts: list[canon.Value] = [[0, c], [1, d]]
        packed = b"".join(value.to_bytes(4, "big") for value in outputs)
        funcs: list[canon.Value] = [[2, packed]]
        p_rows: list[canon.Value] = [[value] for value in sorted(p_values)]
        q_rows: list[canon.Value] = [list(values) for values in sorted(q_values)]
        preds: list[canon.Value] = [[3, p_rows], [4, q_rows]]
        value: list[canon.Value] = [1, h_v, [size], consts, funcs, preds]
        if index >= 8000:
            defect = index % 6
            if defect == 0:
                value[1] = b"\x00" * 32
            elif defect == 1:
                consts[0] = [0, size]
            elif defect == 2:
                value[2] = [0]
            elif defect == 3:
                preds[0] = [3, [[0], [0]]]
            elif defect == 4:
                funcs[0] = [2, packed[:-4]]
            model_bytes = b"\xff" if defect == 5 else canon.encode(value)
            verdict = "Rejected"
        else:
            model_bytes = canon.encode(value)
            verdict = "Accepted" if legitimate else "Rejected"
        cases.append(case(v_bytes, o_bytes, model_bytes))
        expected.append(verdict)
    return cases, expected
