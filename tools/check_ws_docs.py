#!/usr/bin/env python3
"""Coerência documental da trilha WS (Apêndice G) e dos documentos que ela toca.

Confere links e âncoras locais (slug do GitHub), existência de todo ID WS.n,
W1..W9, TW1..TW13 e AW1..AW3 citado, campos obrigatórios das etapas WS sem caixa
marcada, grafo de dependências WS acíclico, nenhuma etapa 5R/10/MEM dependendo
de WS e numeração sequencial de ADRs. Saída: WS_DOCS_OK.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FILES = [
    "ROADMAP.md",
    "README.md",
    "STATUS.md",
    "docs/visao-e-andamento.md",
    "docs/spec/LOGOS-workspace-and-web-ui.md",
    "docs/threat-model.md",
    "docs/limits.md",
    "docs/guarantees.md",
    "docs/adr/0047-produto-unico-com-clientes-e-workspaces.md",
    "docs/adr/0048-servidor-web-local-nao-confiavel.md",
    "docs/adr/0049-artefatos-visualizacoes-e-documentos.md",
    "docs/plans/fase-ws.md",
]
FIELDS = [
    "**Depende de (recebe pronto).**",
    "**Fora do escopo.**",
    "**Entregáveis.**",
    "**Critério de conclusão.**",
]


def slug(heading: str) -> str:
    text = re.sub(r"[^\w\- ]", "", heading.replace("`", "").strip().lower())
    return text.replace(" ", "-")


def anchors(path: Path) -> set[str]:
    found: set[str] = set()
    seen: dict[str, int] = {}
    in_code = False
    text = path.read_text()
    for line in text.splitlines():
        if line.startswith("```"):
            in_code = not in_code
            continue
        match = None if in_code else re.match(r"^(#{1,6}) (.+)$", line)
        if match:
            base = slug(match[2])
            count = seen.get(base, 0)
            seen[base] = count + 1
            found.add(base if count == 0 else f"{base}-{count}")
    return found | set(re.findall(r'<a id="([^"]+)"', text))


def check_links(errors: list[str]) -> None:
    for name in FILES:
        path = ROOT / name
        for link in re.findall(r"(?<!!)\[[^\]\n]*\]\(([^\s)]+)\)", path.read_text()):
            if "://" in link or link.startswith("mailto:"):
                continue
            target, _, fragment = link.partition("#")
            resolved = (path.parent / target) if target else path
            if not resolved.exists():
                errors.append(f"{name}: link quebrado {link}")
            elif fragment and resolved.suffix == ".md" and fragment not in anchors(resolved):
                errors.append(f"{name}: âncora inexistente {link}")


def check_stages(errors: list[str]) -> set[str]:
    roadmap = (ROOT / "ROADMAP.md").read_text()
    appendix = roadmap.split("## Apêndice G", 1)[1].split("*Fim do roadmap.", 1)[0]
    deps: dict[str, set[str]] = {}
    for block in re.split(r"^### Etapa ", appendix, flags=re.M)[1:]:
        stage = block.split()[0]
        if not stage.startswith("WS."):
            continue
        for field in FIELDS:
            if field not in block:
                errors.append(f"{stage}: falta {field}")
        criteria = block.split("**Critério de conclusão.**", 1)[-1]
        if "- [ ]" not in criteria and "- [x]" not in criteria:
            errors.append(f"{stage}: sem critério")
        needed = block.split(FIELDS[0], 1)[-1].split(FIELDS[1], 1)[0]
        deps[stage] = {f"WS.{d}" for d in re.findall(r"WS\.(\d)", needed)} - {stage}
    state: dict[str, int] = {}

    def visit(node: str, path: list[str]) -> None:
        state[node] = 1
        for dep in deps.get(node, set()):
            if dep not in deps:
                errors.append(f"{node}: depende de etapa inexistente {dep}")
            elif state.get(dep) == 1:
                errors.append(f"ciclo: {[*path, node, dep]}")
            elif not state.get(dep):
                visit(dep, [*path, node])
        state[node] = 2

    for node in deps:
        if not state.get(node):
            visit(node, [])
    pattern = r"^### Etapa ((?:5R|10|MEM)\.\w+)(.*?)(?=^### |^## )"
    for match in re.finditer(pattern, roadmap, re.M | re.S):
        needed = match[2].split(FIELDS[0], 1)
        if len(needed) == 2 and re.search(r"\bWS\.\d", needed[1].split(FIELDS[1], 1)[0]):
            errors.append(f"{match[1]} depende de WS")
    return set(deps)


def check_ids(errors: list[str], stages: set[str]) -> None:
    text = "\n".join((ROOT / name).read_text() for name in FILES)
    spec = (ROOT / "docs/spec/LOGOS-workspace-and-web-ui.md").read_text()
    threats = (ROOT / "docs/threat-model.md").read_text()
    roadmap = (ROOT / "ROADMAP.md").read_text()
    for number in set(re.findall(r"\bW(\d+)\b", text)):
        if not re.search(rf"\*\*W{number}\.\*\*", spec):
            errors.append(f"W{number} sem definição")
    for number in set(re.findall(r"\bTW(\d+)\b", text)):
        row = rf"^\| TW{number} \|"
        if not re.search(row, threats, re.M) or not re.search(row, roadmap, re.M):
            errors.append(f"TW{number} sem linha")
    for number in set(re.findall(r"\bAW(\d+)\b", text)):
        if not re.search(rf"^\| AW{number} \|", threats, re.M):
            errors.append(f"AW{number} sem linha")
    for number in set(re.findall(r"\bWS\.(\d+)\b", text)):
        if f"WS.{number}" not in stages:
            errors.append(f"WS.{number} citado sem etapa")
    for section in set(re.findall(r"WS§(\d+)", text)):
        if not re.search(rf"^## {section}\. ", spec, re.M):
            errors.append(f"WS§{section} inexistente")
    for sub in set(re.findall(r"WS§(\d+\.\d+)", text)):
        if not re.search(rf"^### {re.escape(sub)} ", spec, re.M):
            errors.append(f"WS§{sub} inexistente")
    numbers = sorted(int(p.name[:4]) for p in (ROOT / "docs/adr").glob("[0-9][0-9][0-9][0-9]-*.md"))
    if numbers != list(range(1, numbers[-1] + 1)):
        errors.append(f"ADRs não sequenciais: {numbers}")


def main() -> None:
    errors: list[str] = []
    check_links(errors)
    stages = check_stages(errors)
    check_ids(errors, stages)
    if errors:
        print("\n".join(errors))
        sys.exit(1)
    print(f"WS_DOCS_OK stages={len(stages)} files={len(FILES)}")


if __name__ == "__main__":
    main()
