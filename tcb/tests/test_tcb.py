"""Testes da ferramenta de manifesto/contagem da TCB (Etapa 0.2)."""

from __future__ import annotations

import copy
from typing import Any

import pytest
import tcb

Json = dict[str, Any]


@pytest.fixture
def manifest() -> Json:
    return tcb.load_manifest()


def test_real_manifest_is_consistent(manifest: Json) -> None:
    assert tcb.validate(manifest) == []


def test_unlisted_source_file_is_reported(manifest: Json) -> None:
    broken = copy.deepcopy(manifest)
    broken["crate"][0]["files"] = []  # esquece de listar src/lib.rs
    problems = tcb.validate(broken)
    assert any("fora do manifesto" in p and "src/lib.rs" in p for p in problems)


def test_listed_but_missing_file_is_reported(manifest: Json) -> None:
    broken = copy.deepcopy(manifest)
    broken["crate"][0]["files"].append("src/nao_existe.rs")
    assert any("inexistente" in p for p in tcb.validate(broken))


def test_invalid_verified_and_property(manifest: Json) -> None:
    broken = copy.deepcopy(manifest)
    broken["crate"][0]["verified"] = "coq"
    broken["crate"][0]["properties"] = ["velocidade"]
    problems = tcb.validate(broken)
    assert any("verified" in p for p in problems)
    assert any("desconhecida" in p for p in problems)


def test_external_requires_sha256(manifest: Json) -> None:
    broken = copy.deepcopy(manifest)
    broken["external"] = [{"name": "cake_lpr", "sha256": "xyz", "verified": "cakeml"}]
    assert any("sha256" in p for p in tcb.validate(broken))
    broken["external"][0]["sha256"] = "0" * 64
    assert tcb.validate(broken) == []


def _report(total: int, loc: int, deps: list[str]) -> Json:
    return {
        "properties": {"correctness": {"title": "Correção", "loc_total": total}},
        "crates": [{"name": "logos-canon", "loc": loc}],
        "dependencies": [{"name": d} for d in deps],
    }


def test_diff_shows_delta_and_new_dependencies() -> None:
    md = tcb.render_diff(_report(10, 10, []), _report(25, 25, ["sha2"]))
    assert "+15" in md
    assert "`logos-canon`" in md
    assert "`sha2`" in md and "ADR" in md


def test_diff_without_changes() -> None:
    md = tcb.render_diff(_report(10, 10, []), _report(10, 10, []))
    assert "Sem mudança na TCB" in md
