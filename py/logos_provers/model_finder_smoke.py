"""Smoke não autoritativo da Etapa 2.5: AIR → fixture O → finder/fallback → checker Rust real.

Execute em py/: uv run python -m logos_provers.model_finder_smoke.
O é fixture de integração, não obrigação emitida por um monitor/ledger real.
"""

from __future__ import annotations

import hashlib
import subprocess
from pathlib import Path
from typing import Any
from unittest.mock import patch

import z3  # type: ignore[import-untyped]

from logos_client import air, canon
from logos_provers import model_fallback, model_finder

ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    # 1. Fixture AIR: Γ = P(c), goal = forall x:S. P(x)
    document = air.parse_document(
        "vocab v1 { sort S; const c: S; pred P(S); } claim H: P(c); claim G: forall x:S. P(x);"
    )
    vocabulary = canon.encode(document.vocabulary)
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + vocabulary).digest()
    obligation = canon.encode(
        [
            1,
            0,
            air.LOGIC_ID,
            1,
            h_v,
            [[b"\x21" * 32, document.claims[0].formula]],
            document.claims[1].formula,
        ]
    )
    expected_h_o = hashlib.sha256(b"logos/obligation/v1\x00" + obligation).digest()

    # 2. Finder maxdomain2 produz Mdomain2
    limits_domain2 = model_finder.Limits(max_domain=2, timeout_ms=5000)
    finder_domain2 = model_finder.refute(obligation, vocabulary, limits=limits_domain2)
    if finder_domain2.status != model_finder.SearchStatus.FOUND or finder_domain2.model is None:
        raise RuntimeError(
            f"finder failed to produce domain-2 countermodel: status={finder_domain2.status}"
        )
    decoded_model = canon.decode(finder_domain2.model)
    if not isinstance(decoded_model, list) or decoded_model[2] != [2]:
        raise RuntimeError(f"finder countermodel domain must be [2], got {decoded_model!r}")

    # 3. maxdomain1 completo no-model
    finder_domain1 = model_finder.refute(
        obligation, vocabulary, limits=model_finder.Limits(max_domain=1, timeout_ms=5000)
    )
    if (
        finder_domain1.status != model_finder.SearchStatus.NO_MODEL_WITHIN_BOUND
        or finder_domain1.model is not None
    ):
        raise RuntimeError(
            "domain 1 should complete with NO_MODEL_WITHIN_BOUND and no model bytes, "
            f"got {finder_domain1}"
        )

    # 4. maxsteps1 EXHAUSTED sem bytes
    finder_exhausted = model_finder.refute(
        obligation,
        vocabulary,
        limits=model_finder.Limits(max_domain=2, max_steps=1, timeout_ms=5000),
    )
    if (
        finder_exhausted.status != model_finder.SearchStatus.EXHAUSTED
        or finder_exhausted.model is not None
    ):
        raise RuntimeError(
            f"max_steps=1 should return EXHAUSTED without model bytes, got {finder_exhausted}"
        )

    # 5. Fallback exercised após unknown NATIVO REAL de Z3
    orig_check = z3.Solver.check
    observed_unknown: str | None = None

    def wrapped_check(self: Any, *args: Any, **kwargs: Any) -> Any:
        nonlocal observed_unknown
        self.set("rlimit", 1)
        actual_outcome = orig_check(self, *args, **kwargs)
        if actual_outcome != z3.unknown:
            raise AssertionError(f"expected real native z3.unknown, got {actual_outcome}")
        observed_unknown = str(self.reason_unknown())
        return actual_outcome

    with patch.object(z3.Solver, "check", wrapped_check):
        fallback_model = model_fallback.refute(obligation, vocabulary, limits=limits_domain2)

    if observed_unknown is None:
        raise RuntimeError("native Z3 wrapper was not invoked or did not observe unknown")
    if fallback_model is None:
        raise RuntimeError("fallback failed to produce countermodel after native Z3 unknown")

    # 6. Checker Rust real: finder aceito, fallback original O/hO Accepted, wrong hO Rejected
    wrong_h_o = b"\x00" * 32
    cases = [
        ("refute", expected_h_o, finder_domain2.model),
        ("refute", expected_h_o, fallback_model),
        ("refute", wrong_h_o, fallback_model),
    ]
    rows = [
        f"{mode} {vocabulary.hex()} {obligation.hex()} {h_o.hex()} {candidate.hex()}"
        for mode, h_o, candidate in cases
    ]
    result = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "logos-model", "--example", "checker_diff", "--"],
        input="\n".join(rows) + "\n",
        text=True,
        capture_output=True,
        cwd=ROOT,
        timeout=180,
        check=True,
    )
    outcomes = result.stdout.splitlines()
    expected_outcomes = ["Accepted", "Accepted", "Rejected"]
    if outcomes != expected_outcomes:
        raise RuntimeError(f"real checker outcomes: {outcomes!r}; stderr: {result.stderr}")

    print(
        f"MODEL_FINDER_SMOKE_OK domain=[2] unknown={observed_unknown} checker={','.join(outcomes)}"
    )


if __name__ == "__main__":
    main()
