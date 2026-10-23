#!/usr/bin/env python3
"""Plano do nível `affected` de ci/verify.sh (ADR0057): smokes das áreas tocadas.

Lê os caminhos alterados desde `--base` (commits, árvore de trabalho e arquivos novos)
e imprime, separados por espaço, os nomes de smokes/scripts de ci/verify.sh a rodar,
ou `all`. Conservador: caminho sem regra conhecida, base inválida ou mudança de
toolchain/dependências seleciona tudo. Um crate alterado seleciona os smokes de todo
crate que depende dele (fecho transitivo do `cargo metadata`).
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Smoke -> crates Rust e pacotes Python que ele exercita diretamente.
SMOKES: dict[str, tuple[tuple[str, ...], tuple[str, ...]]] = {
    "core-smoke": (("logos-core",), ()),
    "air-smoke": (("logos-air",), ()),
    "vocabulary-smoke": (("logos-air",), ()),
    "sort-check-smoke": (("logos-air",), ()),
    "air-text-smoke": (("logos-air-text",), ()),
    "review-sheet": (("logos-seal",), ()),
    "obligation-smoke": (("logos-obligation",), ()),
    "protocol-smoke": (("logos-protocol",), ()),
    "model-smoke": (("logos-model",), ()),
    "eval-smoke": (("logos-model",), ()),
    "checker-smoke": (("logos-ledger",), ()),
    "events-smoke": (("logos-ledger",), ()),
    "fragment-smoke": (("logos-ground",), ()),
    "ground-smoke": (("logos-ground",), ()),
    "cnf-smoke": (("logos-ground",), ()),
    "lrat-smoke": (("logos-lrat",), ()),
    "z3-models-smoke": ((), ("logos_provers",)),
    "model-finder-smoke": ((), ("logos_provers",)),
    "model-explain-smoke": ((), ("logos_provers",)),
    "check-cnf": (("logos-ground",), ("logos_client",)),
    "check-audit": (("logos-audit", "logos-ledger"), ()),
    "check-sat": (("logos-audit",), ("logos_provers",)),
    "check-farm": (("logos-kernel", "logos-audit"), ("logos_provers",)),
    "check-mcp-research": (("logos-kernel", "logos-audit"), ("logos_mcp",)),
    "check-notebook": (("logos-kernel", "logos-audit", "logos-cli"), ("logos_notebook",)),
    "check-render-templates": (("logos-kernel", "logos-audit"), ("logos_provers",)),
    "check-ledger-crash": (("logos-ledger",), ()),
    "check-ledger-index": (("logos-ledger",), ()),
}
# Pacote Python -> pacotes que dependem dele (importam).
PY_DEPENDENTS = {
    "logos_client": {"logos_provers", "logos_mcp", "logos_notebook"},
    "logos_provers": {"logos_mcp", "logos_notebook"},
    "logos_mcp": {"logos_notebook"},
}
# Caminhos que não exigem smokes (o nível rápido já cobre vetores, docs e lints).
NEUTRAL_PREFIXES = (
    "docs/",
    "spec/",
    "README.md",
    "ROADMAP.md",
    "STATUS.md",
    "AGENTS.md",
    "CLAUDE.md",
    "py/tests/",
    "tcb/",
    "fuzz/",
    ".github/",
    "LICENSE",
)
NEUTRAL_TOOLS_PREFIX = ("tools/gen_", "tools/check_ws_docs.py", "tools/verify_plan.py")


def git(*args: str) -> list[str]:
    output = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout
    return [line for line in output.splitlines() if line]


def changed(base: str) -> list[str]:
    merge = git("merge-base", base, "HEAD")[0]
    paths = set(git("diff", "--name-only", merge, "HEAD"))
    paths |= set(git("diff", "--name-only", "HEAD"))
    paths |= set(git("ls-files", "--others", "--exclude-standard"))
    return sorted(paths)


def reverse_closure() -> dict[str, set[str]]:
    """crate -> crates do workspace que dependem dele (inclusive ele mesmo)."""
    meta = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--no-deps"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )
    members = {p["name"] for p in meta["packages"]}
    deps = {
        p["name"]: {d["name"] for d in p["dependencies"] if d["name"] in members}
        for p in meta["packages"]
    }
    users: dict[str, set[str]] = {name: {name} for name in members}
    changed_any = True
    while changed_any:
        changed_any = False
        for crate, needs in deps.items():
            for dependency in needs:
                for target in list(users):
                    if dependency in users[target] and crate not in users[target]:
                        users[target].add(crate)
                        changed_any = True
    return users


def plan(paths: list[str]) -> str:
    crates: set[str] = set()
    packages: set[str] = set()
    named: set[str] = set()
    for path in paths:
        parts = path.split("/")
        if path.startswith(NEUTRAL_PREFIXES) or path.startswith(NEUTRAL_TOOLS_PREFIX):
            continue
        if parts[0] == "crates" and len(parts) > 2:
            crates.add(parts[1])
        elif parts[0] == "py" and len(parts) > 2 and parts[1].startswith("logos_"):
            packages.add(parts[1])
        elif parts[0] == "ci" and path.startswith("ci/check-") and path.endswith(".sh"):
            named.add(Path(path).stem)
        else:
            # Cargo.lock, toolchain, deny.toml, ci/verify.sh, tools/build_*, pyproject...
            return "all"
    users = reverse_closure()
    affected_crates = set().union(*(users.get(c, {c}) for c in crates)) if crates else set()
    affected_py = set(packages)
    for package in packages:
        affected_py |= PY_DEPENDENTS.get(package, set())
    for smoke, (rust, python) in SMOKES.items():
        if affected_crates & set(rust) or affected_py & set(python):
            named.add(smoke)
    return " ".join(sorted(named & set(SMOKES)))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default="origin/main")
    args = parser.parse_args()
    try:
        paths = changed(args.base)
    except subprocess.CalledProcessError, IndexError:
        print(f"verify_plan: base {args.base!r} inválida; selecionando tudo", file=sys.stderr)
        print("all")
        return
    print(plan(paths))


if __name__ == "__main__":
    main()
