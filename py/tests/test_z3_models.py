"""Etapa 2.4: produtor real, checker Rust, corpus distinto e adversário independente."""

from __future__ import annotations

import hashlib
import time
from collections import Counter
from dataclasses import replace
from typing import Any

import pytest
import z3  # type: ignore[import-untyped]

from logos_client import air, canon
from logos_provers import z3_models as producer
from logos_provers.z3_models import Limits, ProducerError, refute, witness
from tests.z3_models_cases import (
    Case,
    case,
    check,
    corpus_200,
    document_case,
    fake_z3_10k,
    obligation,
)


@pytest.fixture(scope="module")
def corpus() -> list[Case]:
    return corpus_200()


def test_200_refutable_obligations_at_least_95_percent(corpus: list[Case]) -> None:
    controls = check([("refute", item) for item in corpus])
    assert controls == ["Accepted"] * 200  # Establish every case really is refutable.
    candidates = [
        replace(item, model=model)
        for item in corpus
        if (model := refute(item.obligation, item.vocabulary, limits=Limits(max_domain=4)))
        is not None
    ]
    outcomes = check([("refute", item) for item in candidates])
    assert len(candidates) >= 190
    assert outcomes == ["Accepted"] * len(candidates)
    print(f"Z3_CORPUS_OK controls=200 accepted={len(candidates)}/200")


def test_10k_fake_z3_zero_undue_accepted() -> None:
    """Fake Z3 supplies arbitrary certificate bytes at producer → checker boundary."""
    cases, expected = fake_z3_10k()
    outcomes = check([("refute", item) for item in cases])
    assert len(outcomes) == 10_000
    assert outcomes == expected
    counts = Counter(outcomes)
    assert counts["Accepted"] >= 1000  # Reject-everything checker cannot pass.
    print(
        f"FAKE_Z3_OK attempts=10000 legitimate={counts['Accepted']} "
        f"rejected={counts['Rejected']} undue=0"
    )


def test_inverted_negation_in_translator_rejected(monkeypatch: pytest.MonkeyPatch) -> None:
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; const c:S; pred P(S); } claim H:P(c); claim G:forall x:S. P(x);"
    )
    correct = refute(o_bytes, v_bytes, limits=Limits(max_domain=2))
    assert correct is not None
    assert check([("refute", case(v_bytes, o_bytes, correct))]) == ["Accepted"]
    translate = producer.to_smtlib

    def inverted(o: bytes, v: bytes, domains: tuple[int, ...], *, mode: str = "refute") -> str:
        if mode == "refute":
            wire = canon.decode(o)
            assert isinstance(wire, list)
            goal = wire[6]
            assert isinstance(goal, list) and isinstance(goal[3], list)
            # Bug in translator: negate goal before applying the required outer
            # negation, turning Γ ∧ ¬goal into Γ ∧ ¬¬goal. No SMT text matching.
            wire[6] = [goal[0], goal[1], goal[2], [[4], *goal[3]]]
            o = canon.encode(wire)
        return translate(o, v, domains, mode=mode)

    monkeypatch.setattr(producer, "to_smtlib", inverted)
    wrong = refute(o_bytes, v_bytes, limits=Limits(max_domain=2))
    assert wrong is not None
    candidate = case(v_bytes, o_bytes, wrong)
    assert check([("witness", candidate), ("refute", candidate)]) == ["Accepted", "Rejected"]
    print("INVERTED_NEGATION_OK valid_model=True refutation=Rejected")


def test_witness_ignores_goal_truth_but_refute_does_not() -> None:
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; const c:S; pred P(S); } claim H:P(c); claim G:P(c);"
    )
    assert refute(o_bytes, v_bytes, limits=Limits(max_domain=2)) is None
    model = witness(o_bytes, v_bytes, limits=Limits(max_domain=2))
    assert model is not None
    candidate = case(v_bytes, o_bytes, model)
    assert check([("witness", candidate), ("refute", candidate)]) == ["Accepted", "Rejected"]


def test_open_search_requires_mixed_cardinalities() -> None:
    # S must have exactly one element; T must have at least two. A lockstep
    # (1,1),(2,2) search misses this model, not merely produces a larger model.
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; sort T; const s:S; const a:T; const b:T; } "
        "claim H:(forall x:S. x=s) and not(a=b); claim G:false;"
    )
    assert refute(o_bytes, v_bytes, limits=Limits(max_domain=1)) is None
    model = refute(o_bytes, v_bytes, limits=Limits(max_domain=2))
    assert model is not None
    assert check([("refute", case(v_bytes, o_bytes, model))]) == ["Accepted"]


ENUM_SOURCE = (
    'vocab v1 { sort E = enum [a,b] from fact "' + "22" * 32 + '"; '
    'const a:E; const b:E = defined(eq(var(0,"E"),app(symbol("base",const,[],"E"),[]))); '
    "const base:E; } claim H:not(a=base); claim G:b=a;"
)


def test_enum_closure_and_defined_ordinal() -> None:
    o_bytes, v_bytes = document_case(ENUM_SOURCE)
    assert refute(o_bytes, v_bytes, limits=Limits(max_domain=1)) is None
    model = refute(o_bytes, v_bytes, limits=Limits(max_domain=2))
    assert model is not None
    assert check([("refute", case(v_bytes, o_bytes, model))]) == ["Accepted"]
    # A closed two-member domain cannot contain a third individual.
    o_closed, v_closed = document_case(
        ENUM_SOURCE.rsplit("claim G:", 1)[0] + "claim G:forall x:E. x=a or x=b;"
    )
    assert refute(o_closed, v_closed, limits=Limits(max_domain=4)) is None
    bad_source = ENUM_SOURCE.replace('symbol("base",const', 'symbol("a",const')
    o_bad, v_bad = document_case(bad_source)
    assert refute(o_bad, v_bad, limits=Limits(max_domain=2)) is None
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + v_bad).digest()
    bad_model = canon.encode([1, h_v, [2], [[0, 0], [2, 1]], [], []])
    assert check([("refute", case(v_bad, o_bad, bad_model))]) == ["Rejected"]


@pytest.mark.parametrize("body", ["true", "false"])
def test_unused_defined_value_requires_total_unique_relation(body: str) -> None:
    o_bytes, v_bytes = document_case(
        f"vocab v1 {{ sort S; const c:S; const d:S; func bad(S):S = defined({body}); }} "
        "claim H:not(c=d); claim G:false;"
    )
    assert refute(o_bytes, v_bytes, limits=Limits(max_domain=2)) is None
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + v_bytes).digest()
    model = canon.encode([1, h_v, [2], [[0, 0], [1, 1]], [], []])
    assert check([("refute", case(v_bytes, o_bytes, model))]) == ["Rejected"]


def test_nullary_and_dense_tables_preserve_ordinals_and_axes() -> None:
    o_bytes, v_bytes = document_case(
        'vocab v1 { sort S = enum [a,b] from fact "' + "22" * 32 + '"; '
        'sort T = enum [x,y,z] from policy "' + "33" * 32 + '"; '
        "const a:S; const b:S; const x:T; const y:T; const z:T; "
        "func unit():S; func f(S,T):S; pred P0(); pred P(S); pred Q(S,T); } "
        "claim H: unit()=b and P0() and "
        "f(a,x)=a and f(a,y)=b and f(a,z)=a and "
        "f(b,x)=b and f(b,y)=a and f(b,z)=b and "
        "(forall u:S. P(u) <-> u=b) and "
        "(forall u:S. forall v:T. Q(u,v) <-> (u=a and v=z) or (u=b and v=x)); "
        "claim G:false;"
    )
    model = refute(o_bytes, v_bytes, limits=Limits(max_domain=3))
    assert model is not None
    assert check([("refute", case(v_bytes, o_bytes, model))]) == ["Accepted"]


def test_defined_parameters_and_nested_binder_are_isolated() -> None:
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; sort T; const c:S; const d:S; const t:T; pred R(S,T); "
        "pred D(S,T) = defined(exists y:T. "
        'pred(symbol("R",pred,["S","T"],bool),[var(1,"S"),var(2,"T")])); '
        'func select(S,T):S = defined(eq(var(0,"S"),var(1,"S"))); } '
        "claim H:D(c,t) and not D(d,t) and select(c,t)=c; "
        "claim G:forall x:S. exists y:T. R(x,y);"
    )
    model = refute(o_bytes, v_bytes, limits=Limits(max_domain=2))
    assert model is not None
    assert check([("refute", case(v_bytes, o_bytes, model))]) == ["Accepted"]


def test_air_names_cannot_inject_smtlib() -> None:
    o_bytes, v_bytes = document_case(
        'vocab v1 { sort "S) (assert false) ;"; '
        'const "c|λ": "S) (assert false) ;"; pred "Q;|"(); } '
        'claim H:"Q;|"(); claim G:false;'
    )
    model = refute(o_bytes, v_bytes)
    assert model is not None
    assert check([("refute", case(v_bytes, o_bytes, model))]) == ["Accepted"]


def test_empty_vocabulary_and_empty_enumeration() -> None:
    o_bytes, v_bytes = document_case("vocab v1 {} claim G:false;")
    model = refute(o_bytes, v_bytes)
    assert model is not None
    assert check([("refute", case(v_bytes, o_bytes, model))]) == ["Accepted"]
    o_empty, v_empty = document_case(
        'vocab v1 { sort E = enum [] from fact "' + "22" * 32 + '"; } claim G:false;'
    )
    assert refute(o_empty, v_empty) is None
    assert witness(o_empty, v_empty) is None


@pytest.mark.parametrize("tag", [True, False])
def test_boolean_tags_are_not_unsigned_versions(tag: bool) -> None:
    v_bytes = canon.encode([tag, 1, [], []])
    o_bytes = obligation(v_bytes, [], [1, 3, air.LOGIC_ID, [[1]]])
    with pytest.raises(ProducerError):
        refute(o_bytes, v_bytes)
    good_v = canon.encode([1, 1, [], []])
    bad_o = canon.decode(obligation(good_v, [], [1, 3, air.LOGIC_ID, [[1]]]))
    assert isinstance(bad_o, list)
    bad_o[1] = tag
    with pytest.raises(ProducerError):
        witness(canon.encode(bad_o), good_v)


def test_flat_air_depth_is_bounded_before_recursion() -> None:
    v_bytes = canon.encode([1, 1, [], []])
    records: list[canon.Value] = [[4] for _ in range(1500)]
    records.append([1])
    o_bytes = obligation(v_bytes, [], [1, 3, air.LOGIC_ID, records])
    with pytest.raises(ProducerError):
        refute(o_bytes, v_bytes)


def test_malformed_context_and_goal_rejected_even_in_witness() -> None:
    o_bytes, v_bytes = document_case("vocab v1 {sort S; const c:S;} claim G:c=c;")
    wire = canon.decode(o_bytes)
    assert isinstance(wire, list)
    wire[4] = b"\x00" * 32
    with pytest.raises(ProducerError):
        refute(canon.encode(wire), v_bytes)
    wire[4] = hashlib.sha256(b"logos/vocab/v1\x00" + v_bytes).digest()
    wire[6] = air.parse_formula('eq(var(0,"S"),var(0,"S"))')
    with pytest.raises(ProducerError):
        witness(canon.encode(wire), v_bytes)
    with pytest.raises(ProducerError):
        refute(o_bytes + b"\x00", v_bytes)
    with pytest.raises(ProducerError):
        Limits(max_domain=0)
    with pytest.raises(ProducerError):
        Limits(timeout_ms=2**32)


def test_real_z3_unknown_returns_no_certificate(monkeypatch: pytest.MonkeyPatch) -> None:
    # Native Z3 reaches a deterministic resource limit, not a mock echo of status.
    solver_factory = z3.Solver

    def limited_solver(*args: Any, **kwargs: Any) -> Any:
        solver = solver_factory(*args, **kwargs)
        solver.set("rlimit", 1)
        return solver

    monkeypatch.setattr(z3, "Solver", limited_solver)
    o_bytes, v_bytes = document_case("vocab v1 {sort S; const c:S;} claim G:false;")
    assert refute(o_bytes, v_bytes) is None


def test_timeout_is_shared_across_search(monkeypatch: pytest.MonkeyPatch) -> None:
    clock = [0.0]
    translate = producer.to_smtlib
    monkeypatch.setattr(time, "monotonic", lambda: clock[0])

    def slow_translation(*args: Any, **kwargs: Any) -> str:
        result = translate(*args, **kwargs)
        clock[0] += 3.0
        return result

    monkeypatch.setattr(producer, "to_smtlib", slow_translation)
    o_bytes, v_bytes = document_case(
        "vocab v1 {sort S; const c:S; const d:S;} claim H:not(c=d); claim G:false;"
    )
    assert refute(o_bytes, v_bytes, limits=Limits(max_domain=2, timeout_ms=5000)) is None


def test_extraction_expiry_discards_otherwise_valid_certificate(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    clock = [0.0]
    extract = producer._extract_model
    monkeypatch.setattr(time, "monotonic", lambda: clock[0])

    def late_extraction(*args: Any, **kwargs: Any) -> bytes | None:
        result = extract(*args, **kwargs)
        clock[0] = 10.0
        return result

    monkeypatch.setattr(producer, "_extract_model", late_extraction)
    o_bytes, v_bytes = document_case("vocab v1 {} claim G:false;")
    assert refute(o_bytes, v_bytes, limits=Limits(timeout_ms=5000)) is None
