"""Completude finita contra enumeração independente e aceitação do checker real."""

from __future__ import annotations

from collections.abc import Callable
from types import SimpleNamespace
from typing import Any, cast

import pytest

from logos_client import air, canon
from logos_provers import model_context as context
from logos_provers import model_finder as finder
from logos_provers.model_context import ProducerError
from logos_provers.model_finder import Limits, SearchStatus, refute
from tests.model_finder_cases import (
    enumerate_primitive_models_s_c_f_p,
    get_deterministic_corpus,
    has_independent_countermodel,
    make_dense_axis_case,
    make_empty_enum_case,
    make_empty_vocab_case,
    make_enum_fixed_invalid_ordinal_case,
    make_enum_fixed_valid_case,
    make_mixed_sorts_case,
    make_non_unique_defined_func_case,
    make_nullary_case,
    make_out_of_order_defined_case,
    make_strictly_domain_3_case,
    make_unused_invalid_defined_case,
)
from tests.z3_models_cases import Case, case, check, document_case, obligation


def test_independent_corpus_differential_and_rust_checker() -> None:
    models = enumerate_primitive_models_s_c_f_p()
    assert len(models) == 682  # Entire n*n**n*2**n interpretation space, not a sample.
    corpus = get_deterministic_corpus()
    refutable = 0
    absent = 0
    identities: set[tuple[bytes, bytes]] = set()
    rust_cases: list[tuple[str, Case]] = []
    for item in corpus:
        o_bytes, v_bytes = document_case(item.source)
        identities.add((o_bytes, v_bytes))
        vocabulary = context._validate_vocabulary(v_bytes)
        parsed = context._validate_obligation(o_bytes, vocabulary)
        exists = has_independent_countermodel(parsed.premises, parsed.goal, models)
        result = refute(o_bytes, v_bytes, limits=Limits(max_domain=3))
        if exists:
            assert result.status is SearchStatus.FOUND, item.source
            assert result.model is not None, item.source
            rust_cases.append(("refute", case(v_bytes, o_bytes, result.model)))
            refutable += 1
        else:
            assert result.status is SearchStatus.NO_MODEL_WITHIN_BOUND, item.source
            assert result.model is None
            absent += 1
    assert len(identities) == len(corpus)  # No renamed/repeated obligations padding coverage.
    assert refutable > 0 and absent > 0
    outcomes = check(rust_cases)
    assert outcomes == ["Accepted"] * refutable
    print(
        f"EXHAUSTIVE_CORPUS_OK cases={len(corpus)} models={len(models)} "
        f"refutable={refutable} no_model={absent} accepted={len(outcomes)}"
    )


def test_three_and_four_element_boundary() -> None:
    o_bytes, v_bytes = make_strictly_domain_3_case()
    absent = refute(o_bytes, v_bytes, limits=Limits(max_domain=2))
    assert absent.status is SearchStatus.NO_MODEL_WITHIN_BOUND and absent.model is None
    three = refute(o_bytes, v_bytes, limits=Limits(max_domain=3))
    assert three.status is SearchStatus.FOUND and three.model is not None
    o_four, v_four = document_case(
        "vocab v1 { sort S; } claim H:exists a:S. exists b:S. exists c:S. exists d:S. "
        "not(a=b) and not(a=c) and not(a=d) and not(b=c) and not(b=d) and not(c=d); "
        "claim G:false;"
    )
    bound_three = refute(o_four, v_four, limits=Limits(max_domain=3))
    assert bound_three.status is SearchStatus.NO_MODEL_WITHIN_BOUND and bound_three.model is None
    four = refute(o_four, v_four, limits=Limits(max_domain=4))
    assert four.status is SearchStatus.FOUND and four.model is not None
    assert check(
        [
            ("refute", case(v_bytes, o_bytes, three.model)),
            ("refute", case(v_four, o_four, four.model)),
        ]
    ) == ["Accepted", "Accepted"]


@pytest.mark.parametrize(
    "fixture",
    [
        make_mixed_sorts_case,
        make_enum_fixed_valid_case,
        make_out_of_order_defined_case,
        make_nullary_case,
        make_dense_axis_case,
        make_empty_vocab_case,
    ],
)
def test_structural_edges_produce_real_refutations(
    fixture: Callable[[], tuple[bytes, bytes]],
) -> None:
    o_bytes, v_bytes = fixture()
    result = refute(o_bytes, v_bytes, limits=Limits(max_domain=3))
    assert result.status is SearchStatus.FOUND and result.model is not None
    assert check([("refute", case(v_bytes, o_bytes, result.model))]) == ["Accepted"]


@pytest.mark.parametrize(
    "fixture",
    [
        make_enum_fixed_invalid_ordinal_case,
        make_unused_invalid_defined_case,
        make_non_unique_defined_func_case,
        make_empty_enum_case,
    ],
)
def test_invalid_global_definitions_and_empty_enum_have_no_model(
    fixture: Callable[[], tuple[bytes, bytes]],
) -> None:
    o_bytes, v_bytes = fixture()
    result = refute(o_bytes, v_bytes, limits=Limits(max_domain=2))
    assert result.status is SearchStatus.NO_MODEL_WITHIN_BOUND and result.model is None


def test_enum_above_bound_and_singleton_unique_definition() -> None:
    o_bytes, v_bytes = make_enum_fixed_valid_case()
    result = refute(o_bytes, v_bytes, limits=Limits(max_domain=1))
    assert result.status is SearchStatus.NO_MODEL_WITHIN_BOUND and result.model is None
    # A true defining relation is unique in a singleton; rejecting it unconditionally is wrong.
    o_single, v_single = document_case(
        "vocab v1 { sort S; func f(S):S = defined(true); } claim G:false;"
    )
    singleton = refute(o_single, v_single, limits=Limits(max_domain=1))
    assert singleton.status is SearchStatus.FOUND and singleton.model is not None
    assert check([("refute", case(v_single, o_single, singleton.model))]) == ["Accepted"]


def test_defined_parameters_and_nested_binders_are_isolated() -> None:
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; sort T; const c:S; const d:S; const t:T; pred R(S,T); "
        "pred D(S,T) = defined(exists y:T. "
        'pred(symbol("R",pred,["S","T"],bool),[var(1,"S"),var(2,"T")])); '
        'func select(S,T):S = defined(eq(var(0,"S"),var(1,"S"))); } '
        "claim H:D(c,t) and not D(d,t) and select(c,t)=c; "
        "claim G:forall x:S. exists y:T. R(x,y);"
    )
    result = refute(o_bytes, v_bytes, limits=Limits(max_domain=2))
    assert result.status is SearchStatus.FOUND and result.model is not None
    assert check([("refute", case(v_bytes, o_bytes, result.model))]) == ["Accepted"]


def test_invalid_limits_and_context_are_errors() -> None:
    for limits in (
        {"max_domain": True},
        {"max_domain": 0},
        {"max_domain": 5},
        {"max_steps": True},
        {"max_steps": 0},
        {"max_steps": 2**64},
        {"timeout_ms": False},
        {"timeout_ms": 0},
        {"timeout_ms": 2**32},
    ):
        with pytest.raises(ProducerError):
            Limits(**limits)
    o_bytes, v_bytes = make_strictly_domain_3_case()
    with pytest.raises(ProducerError):
        refute(o_bytes, v_bytes, limits=cast(Any, "invalid_limits"))
    for invalid_o, invalid_v in (
        (cast(Any, "not_bytes"), v_bytes),
        (o_bytes, cast(Any, "not_bytes")),
        (o_bytes + b"\x00", v_bytes),
        (o_bytes, v_bytes + b"\x00"),
    ):
        with pytest.raises(ProducerError):
            refute(invalid_o, invalid_v)
    raw = canon.decode(v_bytes)
    assert isinstance(raw, list)
    raw[0] = True
    with pytest.raises(ProducerError):
        refute(o_bytes, canon.encode(raw))
    unbound_goal = air.parse_formula('eq(var(0,"S"),var(0,"S"))')
    with pytest.raises(ProducerError):
        refute(obligation(v_bytes, [], unbound_goal), v_bytes)
    raw_o = canon.decode(o_bytes)
    assert isinstance(raw_o, list)
    raw_o[4] = b"\x00" * 32
    with pytest.raises(ProducerError):
        refute(canon.encode(raw_o), v_bytes)


def test_work_time_and_output_caps_never_claim_absence(monkeypatch: pytest.MonkeyPatch) -> None:
    o_bytes, v_bytes = make_strictly_domain_3_case()
    exhausted = refute(o_bytes, v_bytes, limits=Limits(max_steps=1))
    assert exhausted.status is SearchStatus.EXHAUSTED and exhausted.model is None
    o_big, v_big = document_case(
        'vocab v1 { sort E = enum [a,b] from fact "' + "11" * 32 + '"; '
        "const a:E; const b:E; func huge(" + ",".join(["E"] * 17) + "):E; } claim G:false;"
    )
    output_cap = refute(o_big, v_big, limits=Limits(max_domain=2))
    assert output_cap.status is SearchStatus.EXHAUSTED and output_cap.model is None
    calls = 0

    def clock() -> float:
        nonlocal calls
        calls += 1
        return 0.0 if calls < 12 else 2.0

    # Replace only finder clock, not stdlib time shared with pytest/subprocess.
    monkeypatch.setattr(finder, "time", SimpleNamespace(monotonic=clock))
    timed = refute(o_bytes, v_bytes, limits=Limits(timeout_ms=1000))
    assert timed.status is SearchStatus.EXHAUSTED and timed.model is None
