#!/usr/bin/env python3
"""Manifesto e contagem da TCB (roadmap, Etapa 0.2).

Subcomandos:
  check                 confere tcb/manifest.toml contra o workspace (falha na CI se divergir)
  report                mede a TCB: JSON (--json) e tabela Markdown (--markdown)
  diff BASE HEAD        compara dois relatórios JSON em Markdown (comentário de PR)
  strict-crates         imprime os crates com `strict = true`, um por linha

Contagem: linhas de *código* Rust (tokei: sem comentários, docs ou linhas em branco) dos
arquivos listados no manifesto, mais o `src/` e o `build.rs` das dependências transitivas
(normais e de build) dos crates da TCB. Testes dentro de `src/` (`#[cfg(test)]`) contam:
é uma cota superior da TCB.

Só usa a biblioteca padrão; depende de `cargo` e `tokei` no PATH para `report`.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "tcb" / "manifest.toml"
FORMAT_VERSION = 1
PROPERTIES = ("correctness", "fidelity", "progress")
VERIFIED = ("none", "kani", "verus", "cakeml")

Json = dict[str, Any]


class ManifestError(Exception):
    """Manifesto inválido ou divergente do repositório."""


def set_root(root: Path) -> None:
    """Aponta a ferramenta para outra árvore do repositório (ex.: checkout da base do PR)."""
    global ROOT, MANIFEST, _METADATA
    ROOT = root.resolve()
    MANIFEST = ROOT / "tcb" / "manifest.toml"
    _METADATA = None


def load_manifest(path: Path | None = None) -> Json:
    with (path or MANIFEST).open("rb") as f:
        return tomllib.load(f)


def crate_dir(name: str) -> Path:
    return ROOT / "crates" / name


def actual_sources(name: str) -> set[str]:
    """Caminhos .rs de src/ do crate, relativos ao diretório do crate."""
    base = crate_dir(name)
    return {str(p.relative_to(base)) for p in (base / "src").rglob("*.rs")}


def validate(manifest: Json) -> list[str]:
    """Devolve a lista de problemas do manifesto (vazia se está tudo certo)."""
    problems: list[str] = []
    if manifest.get("version") != FORMAT_VERSION:
        problems.append(f"version deve ser {FORMAT_VERSION}")
    props = set(manifest.get("properties", {}))
    for p in PROPERTIES:
        if p not in props:
            problems.append(f"propriedade ausente: {p}")

    seen: set[str] = set()
    for entry in manifest.get("crate", []):
        name = entry.get("name", "<sem nome>")
        if name in seen:
            problems.append(f"crate duplicado: {name}")
        seen.add(name)
        if not crate_dir(name).is_dir():
            problems.append(f"{name}: diretório crates/{name} não existe")
            continue
        for prop in entry.get("properties", []):
            if prop not in props:
                problems.append(f"{name}: propriedade desconhecida {prop!r}")
        if not entry.get("properties"):
            problems.append(f"{name}: precisa de ao menos uma propriedade")
        if entry.get("verified") not in VERIFIED:
            problems.append(f"{name}: verified deve ser um de {VERIFIED}")
        listed = set(entry.get("files", []))
        actual = actual_sources(name)
        for f in sorted(actual - listed):
            problems.append(
                f"{name}: arquivo fora do manifesto: {f} (adicione em tcb/manifest.toml)"
            )
        for f in sorted(listed - actual):
            problems.append(f"{name}: manifesto lista arquivo inexistente: {f}")

    for ext in manifest.get("external", []):
        name = ext.get("name", "<sem nome>")
        if not re.fullmatch(r"[0-9a-f]{64}", str(ext.get("sha256", ""))):
            problems.append(f"external {name}: sha256 deve ter 64 dígitos hex minúsculos")
        if ext.get("verified") not in VERIFIED:
            problems.append(f"external {name}: verified deve ser um de {VERIFIED}")
        for prop in ext.get("properties", []):
            if prop not in props:
                problems.append(f"external {name}: propriedade desconhecida {prop!r}")
    return problems


def run(cmd: list[str]) -> str:
    try:
        return subprocess.run(cmd, cwd=ROOT, check=True, capture_output=True, text=True).stdout
    except FileNotFoundError as e:
        raise SystemExit(f"erro: {cmd[0]} não encontrado no PATH") from e
    except subprocess.CalledProcessError as e:
        raise SystemExit(f"erro: {' '.join(cmd)} falhou:\n{e.stderr}") from e


def tokei_rust_loc(paths: list[Path]) -> dict[str, int]:
    """Linhas de código Rust por arquivo (caminho relativo à raiz, ou absoluto se fora dela)."""
    if not paths:
        return {}
    data = json.loads(run(["tokei", "--types", "Rust", "-o", "json", *map(str, paths)]))
    out: dict[str, int] = {}
    for report in data.get("Rust", {}).get("reports", []):
        out[str(Path(report["name"]).resolve())] = int(report["stats"]["code"])
    return out


_METADATA: Json | None = None


def cargo_metadata() -> Json:
    global _METADATA
    if _METADATA is None:
        _METADATA = json.loads(run(["cargo", "metadata", "--format-version", "1", "--locked"]))
    return _METADATA


def transitive_dependencies(tcb_crates: set[str]) -> list[Json]:
    """Dependências externas (fora do workspace) alcançáveis a partir de `tcb_crates`."""
    meta = cargo_metadata()
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    workspace = set(meta["workspace_members"])
    stack = [pid for pid in workspace if packages[pid]["name"] in tcb_crates]
    seen: set[str] = set()
    while stack:
        pid = stack.pop()
        if pid in seen:
            continue
        seen.add(pid)
        for dep in nodes[pid]["deps"]:
            kinds = {k["kind"] for k in dep["dep_kinds"]}
            if kinds <= {"dev"}:
                continue  # só dev-dependency: não entra na TCB
            stack.append(dep["pkg"])
    deps: list[Json] = []
    for pid in sorted(seen - workspace, key=lambda i: packages[i]["name"]):
        pkg = packages[pid]
        base = Path(pkg["manifest_path"]).parent
        deps.append(
            {
                "name": pkg["name"],
                "version": pkg["version"],
                "paths": [p for p in (base / "src", base / "build.rs") if p.exists()],
            }
        )
    return deps


def git(*args: str) -> str:
    try:
        return run(["git", *args]).strip()
    except SystemExit:
        return ""


def build_report(manifest: Json) -> Json:
    crates = manifest.get("crate", [])
    files = [crate_dir(c["name"]) / f for c in crates for f in c["files"]]
    loc = tokei_rust_loc(files)

    crate_rows: list[Json] = []
    for c in crates:
        own = sum(loc.get(str((crate_dir(c["name"]) / f).resolve()), 0) for f in c["files"])
        crate_rows.append(
            {
                "name": c["name"],
                "properties": c["properties"],
                "verified": c["verified"],
                "files": len(c["files"]),
                "loc": own,
            }
        )

    # Cada dependência pertence às propriedades dos crates da TCB que (transitivamente) a usam.
    dep_props: dict[str, set[str]] = {}
    dep_info: dict[str, Json] = {}
    for prop in PROPERTIES:
        member_names = {c["name"] for c in crates if prop in c["properties"]}
        for dep in transitive_dependencies(member_names) if member_names else []:
            key = f"{dep['name']} {dep['version']}"
            dep_props.setdefault(key, set()).add(prop)
            dep_info[key] = dep
    dep_rows: list[Json] = []
    for key in sorted(dep_info):
        dep = dep_info[key]
        dep_loc = sum(tokei_rust_loc(dep["paths"]).values())
        dep_rows.append(
            {
                "name": dep["name"],
                "version": dep["version"],
                "properties": sorted(dep_props[key]),
                "loc": dep_loc,
            }
        )

    per_property: Json = {}
    for prop in PROPERTIES:
        members = [r for r in crate_rows if prop in r["properties"]]
        own = sum(r["loc"] for r in members)
        verified = sum(r["loc"] for r in members if r["verified"] != "none")
        deps = sum(d["loc"] for d in dep_rows if prop in d["properties"])
        total = own + deps
        per_property[prop] = {
            "title": manifest["properties"][prop]["title"],
            "crates": [r["name"] for r in members],
            "loc_own": own,
            "loc_deps": deps,
            "loc_total": total,
            "loc_verified": verified,
            "verified_ratio": round(verified / total, 4) if total else 0.0,
        }

    return {
        "format": FORMAT_VERSION,
        "commit": git("rev-parse", "HEAD"),
        "dirty": bool(git("status", "--porcelain")),
        "unit": "linhas de código Rust (tokei), cota superior (inclui #[cfg(test)] em src/)",
        "properties": per_property,
        "crates": crate_rows,
        "dependencies": dep_rows,
        "external": manifest.get("external", []),
    }


def render_markdown(report: Json) -> str:
    lines = [
        "## TCB",
        "",
        f"Commit `{report['commit'][:12] or 'desconhecido'}`"
        + (" (árvore modificada)" if report["dirty"] else "")
        + f" · {report['unit']}",
        "",
        "| Propriedade | Crates | LOC próprio | LOC deps | Total | Verificado |",
        "| --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for p in report["properties"].values():
        lines.append(
            f"| {p['title']} | {len(p['crates'])} | {p['loc_own']} | {p['loc_deps']} "
            f"| {p['loc_total']} | {p['loc_verified']} ({p['verified_ratio']:.0%}) |"
        )
    lines += [
        "",
        "| Crate | Propriedades | Arquivos | LOC | Verificado |",
        "| --- | --- | ---: | ---: | --- |",
    ]
    for c in report["crates"]:
        lines.append(
            f"| `{c['name']}` | {', '.join(c['properties'])} | {c['files']} | {c['loc']} "
            f"| {c['verified']} |"
        )
    if report["dependencies"]:
        lines += ["", "| Dependência | Versão | Propriedades | LOC |", "| --- | --- | --- | ---: |"]
        for d in report["dependencies"]:
            props = ", ".join(d["properties"])
            lines.append(f"| `{d['name']}` | {d['version']} | {props} | {d['loc']} |")
    else:
        lines += ["", "Nenhuma dependência externa na TCB."]
    for e in report["external"]:
        lines.append(
            f"- binário externo `{e['name']}` sha256 `{e['sha256'][:12]}…` ({e['verified']})"
        )
    return "\n".join(lines) + "\n"


def signed(n: int) -> str:
    return f"+{n}" if n > 0 else str(n)


def render_diff(base: Json, head: Json) -> str:
    lines = [
        "## Diferença da TCB em relação a `main`",
        "",
        "| Propriedade | Base | Este PR | Δ |",
        "| --- | ---: | ---: | ---: |",
    ]
    for prop, hp in head["properties"].items():
        bp = base["properties"].get(prop, {"loc_total": 0})
        delta = hp["loc_total"] - bp["loc_total"]
        lines.append(f"| {hp['title']} | {bp['loc_total']} | {hp['loc_total']} | {signed(delta)} |")

    base_crates = {c["name"]: c for c in base["crates"]}
    changed = []
    for c in head["crates"]:
        before = base_crates.get(c["name"], {"loc": 0})["loc"]
        if before != c["loc"] or c["name"] not in base_crates:
            changed.append((c["name"], before, c["loc"]))
    removed = [n for n in base_crates if n not in {c["name"] for c in head["crates"]}]
    if changed or removed:
        lines += ["", "| Crate | Base | Este PR | Δ |", "| --- | ---: | ---: | ---: |"]
        lines += [f"| `{n}` | {b} | {h} | {signed(h - b)} |" for n, b, h in changed]
        lines += [
            f"| `{n}` (removido da TCB) | {base_crates[n]['loc']} | 0 "
            f"| {signed(-base_crates[n]['loc'])} |"
            for n in removed
        ]
    base_deps = {d["name"]: d for d in base["dependencies"]}
    head_deps = {d["name"]: d for d in head["dependencies"]}
    new_deps = sorted(set(head_deps) - set(base_deps))
    if new_deps:
        lines += [
            "",
            "**Novas dependências na TCB** (exigem ADR): " + ", ".join(f"`{n}`" for n in new_deps),
        ]
    if not (changed or removed or new_deps):
        lines += ["", "Sem mudança na TCB."]
    return "\n".join(lines) + "\n"


def cmd_check(_: argparse.Namespace) -> int:
    problems = validate(load_manifest())
    for p in problems:
        print(f"ERRO: {p}", file=sys.stderr)
    if not problems:
        print("ok: tcb/manifest.toml consistente com o workspace")
    return 1 if problems else 0


def cmd_report(args: argparse.Namespace) -> int:
    manifest = load_manifest()
    problems = validate(manifest)
    if problems:
        for p in problems:
            print(f"ERRO: {p}", file=sys.stderr)
        return 1
    report = build_report(manifest)
    text = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    if args.json:
        Path(args.json).parent.mkdir(parents=True, exist_ok=True)
        Path(args.json).write_text(text)
    if args.save:
        if not report["commit"]:
            print("erro: --save exige um repositório git", file=sys.stderr)
            return 1
        dest = ROOT / "tcb" / "history" / f"{report['commit']}.json"
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(text)
        print(f"salvo em {dest.relative_to(ROOT)}", file=sys.stderr)
    md = render_markdown(report)
    if args.markdown:
        Path(args.markdown).write_text(md)
    sys.stdout.write(md)
    return 0


def cmd_diff(args: argparse.Namespace) -> int:
    base = json.loads(Path(args.base).read_text())
    head = json.loads(Path(args.head).read_text())
    sys.stdout.write(render_diff(base, head))
    return 0


def cmd_strict(_: argparse.Namespace) -> int:
    for c in load_manifest().get("crate", []):
        if c.get("strict"):
            print(c["name"])
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawTextHelpFormatter
    )
    parser.add_argument("--root", type=Path, help="raiz do repositório a medir (padrão: este)")
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check").set_defaults(fn=cmd_check)
    rep = sub.add_parser("report")
    rep.add_argument("--json", help="grava o relatório JSON neste caminho")
    rep.add_argument("--markdown", help="grava a tabela Markdown neste caminho")
    rep.add_argument("--save", action="store_true", help="grava em tcb/history/<commit>.json")
    rep.set_defaults(fn=cmd_report)
    d = sub.add_parser("diff")
    d.add_argument("base")
    d.add_argument("head")
    d.set_defaults(fn=cmd_diff)
    sub.add_parser("strict-crates").set_defaults(fn=cmd_strict)
    args = parser.parse_args(argv)
    if args.root:
        set_root(args.root)
    return int(args.fn(args))


if __name__ == "__main__":
    sys.exit(main())
