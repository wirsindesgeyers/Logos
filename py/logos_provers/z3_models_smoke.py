"""Smoke não autoritativo: AIR → fixture O → SMT-LIB/Z3 → checker Rust real.

Execute em py/: uv run python -m logos_provers.z3_models_smoke.
O é fixture de integração, não obrigação emitida por um monitor/ledger real.
"""

from __future__ import annotations

import hashlib
import subprocess
from pathlib import Path

from logos_client import air, canon
from logos_provers.z3_models import Limits, refute, witness

ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
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
    expected = hashlib.sha256(b"logos/obligation/v1\x00" + obligation).digest()
    limits = Limits(max_domain=2, timeout_ms=5000)
    countermodel = refute(obligation, vocabulary, limits=limits)
    if countermodel is None:
        raise RuntimeError("Z3 failed to produce the known two-element countermodel")
    model = canon.decode(countermodel)
    if not isinstance(model, list) or model[2] != [2]:
        raise RuntimeError("countermodel must distinguish c from an individual outside P")
    consistent = witness(obligation, vocabulary, limits=limits)
    if consistent is None:
        raise RuntimeError("Z3 failed to produce the one-element consistency witness")
    if refute(obligation, vocabulary, limits=Limits(max_domain=1, timeout_ms=5000)) is not None:
        raise RuntimeError("one individual cannot refute forall x:S. P(x) under P(c)")

    wrong_context: list[canon.Value] = list(model)
    wrong_context[1] = b"\x00" * 32
    cases = [
        ("refute", expected, countermodel),
        ("witness", expected, consistent),
        ("refute", expected, consistent),
        ("refute", b"\x00" * 32, countermodel),
        ("refute", expected, canon.encode(wrong_context)),
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
    expected_outcomes = ["Accepted", "Accepted", "Rejected", "Rejected", "Rejected"]
    if outcomes != expected_outcomes:
        raise RuntimeError(f"real checker outcomes: {outcomes!r}; stderr: {result.stderr}")
    h_m = hashlib.sha256(b"logos/model/v1\x00" + countermodel).hexdigest()
    print(f"Z3_MODELS_SMOKE_OK domain=[2] h(M)={h_m} checker={','.join(outcomes)}")


if __name__ == "__main__":
    main()
