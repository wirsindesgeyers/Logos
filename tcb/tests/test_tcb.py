"""Testes da ferramenta de manifesto/contagem da TCB (Etapa 0.2)."""

from __future__ import annotations

import copy
import subprocess
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path
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
    assert "`sha2`" in md


@contextmanager
def _measured_workspace(
    root: Path,
    packages: dict[str, tuple[int, str]],
    roots: dict[str, str],
    *,
    proc_macros: frozenset[str] = frozenset(),
) -> Iterator[Path]:
    """Grafo Cargo local real; restaura inclusive o cache após cada medição."""
    root.mkdir()
    (root / "Cargo.toml").write_text(
        '[workspace]\nmembers = ["crates/*"]\nresolver = "2"\n', encoding="utf-8"
    )
    (root / "tcb").mkdir()
    manifest_text = "version = 1\n"
    for prop in tcb.PROPERTIES:
        manifest_text += f'\n[properties.{prop}]\ntitle = "{prop}"\n'
    for name, prop in roots.items():
        manifest_text += (
            f'\n[[crate]]\nname = "{name}"\nproperties = ["{prop}"]\n'
            'strict = false\nverified = "none"\nfiles = ["src/lib.rs"]\n'
        )
    (root / "tcb" / "manifest.toml").write_text(manifest_text, encoding="utf-8")
    for name, (loc, dependencies) in packages.items():
        base = root / "crates" / name
        (base / "src").mkdir(parents=True)
        cargo = f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2024"\n'
        source = "".join(f"pub fn f{i}() {{}}\n" for i in range(loc))
        if name in proc_macros:
            cargo += "\n[lib]\nproc-macro = true\n"
            source = (
                "extern crate proc_macro;\n#[proc_macro]\n"
                "pub fn identity(item: proc_macro::TokenStream)"
                " -> proc_macro::TokenStream { item }\n"
            )
        (base / "Cargo.toml").write_text(cargo + dependencies, encoding="utf-8")
        (base / "src" / "lib.rs").write_text(source, encoding="utf-8")
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"], cwd=root, check=True, capture_output=True
    )
    old_root, old_manifest, old_meta = tcb.ROOT, tcb.MANIFEST, tcb._METADATA
    try:
        tcb.set_root(root)
        yield root
    finally:
        tcb.set_root(old_root)
        tcb.MANIFEST, tcb._METADATA = old_manifest, old_meta


def test_workspace_dependency_in_another_property_is_counted(tmp_path: Path) -> None:
    packages = {
        "logos-canon": (3, '\n[dependencies]\nlogos-cli = { path = "../logos-cli" }\n'),
        "logos-cli": (1, ""),
    }
    roots = {"logos-canon": "correctness", "logos-cli": "fidelity"}
    with _measured_workspace(tmp_path / "workspace", packages, roots):
        report = tcb.build_report(tcb.load_manifest())
    corr, fid = report["properties"]["correctness"], report["properties"]["fidelity"]
    assert (corr["loc_own"], corr["loc_deps"], corr["loc_total"]) == (3, 1, 4)
    assert (fid["loc_own"], fid["loc_deps"], fid["loc_total"]) == (1, 0, 1)
    assert [(d["name"], d["kind"], d["loc"]) for d in report["dependencies"]] == [
        ("logos-cli", "runtime", 1)
    ]


def test_shared_roots_are_not_duplicated_and_dev_is_excluded(tmp_path: Path) -> None:
    packages = {
        "a": (
            3,
            '\n[dependencies]\nb = { path = "../b" }\n'
            '\n[build-dependencies]\nbuild = { path = "../build" }\n'
            '\n[dev-dependencies]\ndev = { path = "../dev" }\n',
        ),
        "b": (2, ""),
        "build": (1, ""),
        "dev": (10, ""),
    }
    with _measured_workspace(
        tmp_path / "workspace", packages, {"a": "correctness", "b": "correctness"}
    ):
        report = tcb.build_report(tcb.load_manifest())
    corr = report["properties"]["correctness"]
    assert (corr["loc_own"], corr["loc_deps"], corr["loc_build_deps"], corr["loc_total"]) == (
        5,
        0,
        1,
        5,
    )
    assert [(d["name"], d["kind"]) for d in report["dependencies"]] == [("build", "build")]


def test_dependency_usage_is_classified_per_property(tmp_path: Path) -> None:
    packages = {
        "a": (1, '\n[dependencies]\nshared = { path = "../shared" }\n'),
        "b": (1, '\n[build-dependencies]\nshared = { path = "../shared" }\n'),
        "shared": (10, ""),
    }
    with _measured_workspace(
        tmp_path / "workspace", packages, {"a": "correctness", "b": "fidelity"}
    ):
        report = tcb.build_report(tcb.load_manifest())
    corr, fid = report["properties"]["correctness"], report["properties"]["fidelity"]
    assert (corr["loc_deps"], corr["loc_build_deps"], corr["loc_total"]) == (10, 0, 11)
    assert (fid["loc_deps"], fid["loc_build_deps"], fid["loc_total"]) == (0, 10, 1)


def test_proc_macro_is_build_only_even_on_normal_dependency_edge(tmp_path: Path) -> None:
    packages = {
        "a": (1, '\n[dependencies]\nmacros = { path = "../macros" }\n'),
        "macros": (3, ""),
    }
    with _measured_workspace(
        tmp_path / "workspace", packages, {"a": "correctness"}, proc_macros=frozenset({"macros"})
    ):
        report = tcb.build_report(tcb.load_manifest())
    corr = report["properties"]["correctness"]
    assert (corr["loc_deps"], corr["loc_build_deps"], corr["loc_total"]) == (0, 3, 1)
