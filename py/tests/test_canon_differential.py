"""Teste diferencial Rust x Python da codificação canônica (roadmap 0.3).

Duas direções, 100k casos cada (``LOGOS_DIFF_CASES`` ajusta):
  * Python gera objetos/mutações e julga; o Rust (`examples/diff check`) precisa concordar no
    veredito (aceite + renderização, ou o mesmo nome de erro) e reencodar os bytes aceitos
    de forma idêntica.
  * Rust gera e julga; o Python precisa concordar e reencodar de forma idêntica.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

from logos_client import canon
from tests import canon_gen

ROOT = Path(__file__).resolve().parents[2]
N = int(os.environ.get("LOGOS_DIFF_CASES", "100000"))
CARGO = ["cargo", "run", "--quiet", "-p", "logos-canon", "--example", "diff", "--"]


def cargo(*args: str, stdin: str | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [*CARGO, *args], cwd=ROOT, input=stdin, capture_output=True, text=True, check=False
    )


def test_python_cases_agree_in_rust() -> None:
    lines = []
    ok = 0
    for data in canon_gen.cases(N, seed=20261001):
        v = canon.verdict(data)
        if v.startswith("ok:"):
            ok += 1
            assert canon.encode(canon.decode(data)) == data  # Python: ponto fixo
        lines.append(f"{data.hex()}\t{v}")
    assert 0.2 * N < ok < 0.9 * N, f"mistura de casos pouco variada: {ok}/{N} aceitos"
    result = cargo("check", stdin="\n".join(lines) + "\n")
    assert result.returncode == 0, result.stderr + result.stdout


def test_rust_cases_agree_in_python() -> None:
    result = cargo("gen", str(N), "424242")
    assert result.returncode == 0, result.stderr
    total = ok = 0
    for line in result.stdout.splitlines():
        hex_, expected = line.split("\t")
        data = bytes.fromhex(hex_)
        assert canon.verdict(data) == expected, hex_
        if expected.startswith("ok:"):
            ok += 1
            assert canon.encode(canon.decode(data)) == data, hex_
        total += 1
    assert total == N
    assert 0.2 * N < ok < 0.9 * N


@pytest.mark.parametrize("seed", [1, 2, 3])
def test_generators_are_deterministic(seed: int) -> None:
    assert canon_gen.cases(50, seed) == canon_gen.cases(50, seed)
