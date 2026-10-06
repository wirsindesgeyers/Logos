"""Testes permanentes da Etapa 2.5: fallback Z3 → finder exaustivo."""

from __future__ import annotations

from typing import Any, cast
from unittest.mock import patch

import pytest
import z3  # type: ignore[import-untyped]

from logos_client import canon
from logos_provers import model_fallback, model_finder, z3_models
from logos_provers.model_context import ProducerError
from tests.z3_models_cases import Case, case, check, document_case


def test_real_z3_unknown_fallback_model_accepted_by_real_checker() -> None:
    """Z3 atinge limite nativo de recursos (unknown real) e o fallback entrega modelo aceito."""
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; const c: S; pred P(S); } claim H: P(c); claim G: forall x:S. P(x);"
    )
    orig_check = z3.Solver.check
    observed_unknown: Any = None

    def wrapped_check(self: Any, *args: Any, **kwargs: Any) -> Any:
        nonlocal observed_unknown
        self.set("rlimit", 1)
        actual = orig_check(self, *args, **kwargs)
        assert actual == z3.unknown
        observed_unknown = actual
        return actual

    with patch.object(z3.Solver, "check", wrapped_check):
        model = model_fallback.refute(
            o_bytes, v_bytes, limits=model_finder.Limits(max_domain=2, timeout_ms=5000)
        )

    assert observed_unknown == z3.unknown
    assert model is not None
    decoded = canon.decode(model)
    assert isinstance(decoded, list) and decoded[2] == [2]

    # Checker Rust real aceita com h(O) correto e rejeita com h(O) incorreto
    outcomes = check(
        [
            ("refute", case(v_bytes, o_bytes, model)),
            ("refute", Case(v_bytes, o_bytes, b"\x00" * 32, model)),
        ]
    )
    assert outcomes == ["Accepted", "Rejected"]


def test_fallback_no_model_within_bound_returns_none() -> None:
    """Espaço finito completamente explorado sem modelo retorna None (nunca modelo falso)."""
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; const c: S; pred P(S); } claim H: P(c); claim G: forall x:S. P(x);"
    )
    # Domínio 1 não é suficiente para refutar ∀x. P(x) sob P(c).
    # Z3 com max_domain=1 retorna None (unsat no domínio 1).
    # Finder com max_domain=1 completa NO_MODEL_WITHIN_BOUND.
    result = model_fallback.refute(
        o_bytes, v_bytes, limits=model_finder.Limits(max_domain=1, timeout_ms=5000)
    )
    assert result is None


def test_fallback_exhaustion_returns_none() -> None:
    """Interrupção de passos ou tempo no finder retorna None sem emitir modelo parcial."""
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; const c: S; pred P(S); } claim H: P(c); claim G: forall x:S. P(x);"
    )
    orig_check = z3.Solver.check

    def limited_check(self: Any, *args: Any, **kwargs: Any) -> Any:
        self.set("rlimit", 1)
        return orig_check(self, *args, **kwargs)

    # Com Z3 retornando unknown e max_steps=1 no finder, esgota imediatamente
    with patch.object(z3.Solver, "check", limited_check):
        result = model_fallback.refute(
            o_bytes,
            v_bytes,
            limits=model_finder.Limits(max_domain=2, max_steps=1, timeout_ms=5000),
        )
    assert result is None


def test_invalid_inputs_propagate_producer_error_and_never_return_none() -> None:
    """Erros de formato/validação devem propagar ProducerError, nunca virar None silencioso."""
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; const c: S; pred P(S); } claim H: P(c); claim G: forall x:S. P(x);"
    )

    # Byte inválido não canon
    with pytest.raises(ProducerError):
        model_fallback.refute(b"\xff\xff\xff", v_bytes)
    with pytest.raises(ProducerError):
        model_fallback.refute(o_bytes, b"\xff\xff\xff")

    # Bytes com lixo residual
    with pytest.raises(ProducerError):
        model_fallback.refute(o_bytes + b"\x00", v_bytes)
    with pytest.raises(ProducerError):
        model_fallback.refute(o_bytes, v_bytes + b"\x00")

    # Wire com hash inválido de vocabulário
    wire = canon.decode(o_bytes)
    assert isinstance(wire, list)
    wire[4] = b"\x00" * 32
    with pytest.raises(ProducerError):
        model_fallback.refute(canon.encode(wire), v_bytes)


def test_limits_type_validation_even_when_primary_succeeds() -> None:
    """limits deve ser validado antes de chamar o primário, mesmo se Z3 encontraria modelo."""
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; const c: S; pred P(S); } claim H: P(c); claim G: forall x:S. P(x);"
    )

    # limits inválido (não é model_finder.Limits)
    with pytest.raises(ProducerError):
        model_fallback.refute(o_bytes, v_bytes, limits=cast(Any, "not_limits"))

    with pytest.raises(ProducerError):
        model_fallback.refute(o_bytes, v_bytes, limits=cast(Any, 42))

    # z3_limits inválido
    with pytest.raises(ProducerError):
        model_fallback.refute(o_bytes, v_bytes, z3_limits=cast(Any, "not_z3_limits"))


def test_explicit_z3_limits_decoupling() -> None:
    """z3_limits explícito permite configurar limites diferentes para o produtor primário."""
    o_bytes, v_bytes = document_case(
        "vocab v1 { sort S; const c: S; pred P(S); } claim H: P(c); claim G: forall x:S. P(x);"
    )
    # Z3 com max_domain=1 não encontra modelo e retorna None
    # Finder com max_domain=2 encontra o contra-modelo
    result = model_fallback.refute(
        o_bytes,
        v_bytes,
        limits=model_finder.Limits(max_domain=2, timeout_ms=5000),
        z3_limits=z3_models.Limits(max_domain=1, timeout_ms=5000),
    )
    assert result is not None
    assert check([("refute", case(v_bytes, o_bytes, result))]) == ["Accepted"]
