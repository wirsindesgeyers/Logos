"""Etapa 2.6: 20 fixtures → checker Rust real → apresentação PT, sem autoridade.

Execute em py/: uv run python -m logos_provers.model_explain_smoke.
Fixtures O não autenticam eventos; h(O) real deve vir do monitor.
"""

from __future__ import annotations

import hashlib
import subprocess
from pathlib import Path

from logos_client import air, canon
from logos_provers.model_explain import explain_model
from logos_provers.model_explain_examples import corpus_20

ROOT = Path(__file__).resolve().parents[2]


def main() -> None:
    cases = corpus_20()
    rows = [
        f"refute {item.vocabulary.hex()} {item.obligation.hex()} "
        f"{hashlib.sha256(b'logos/obligation/v1\x00' + item.obligation).hexdigest()} "
        f"{item.model.hex()}"
        for item in cases
    ]
    result = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "logos-model", "--example", "checker_diff", "--"],
        input="\n".join(rows) + "\n",
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
        timeout=180,
    )
    if len(cases) != 20 or result.stdout.splitlines() != ["Accepted"] * 20:
        raise RuntimeError(f"20 refutations not accepted: {result.stdout!r}; {result.stderr}")
    explanations: list[str] = []
    for item in cases:
        text = explain_model(item.model, item.vocabulary, item.obligation)
        obligation = canon.decode(item.obligation)
        assert isinstance(obligation, list)
        goal = air.print_formula(obligation[6])
        h_m = hashlib.sha256(b"logos/model/v1\x00" + item.model).hexdigest()
        if f"Objetivo falsificado: {goal}" not in text or f"h(M): {h_m}" not in text:
            raise RuntimeError("explanation lost false goal or model identity")
        if "Premissa " in text and ": falsa — " in text:
            raise RuntimeError("accepted refutation displayed a false premise")
        explanations.append(text)
    print(explanations[0])
    print("MODEL_EXPLAIN_SMOKE_OK accepted=20/20 false_goals=20 hashes=20")


if __name__ == "__main__":
    main()
