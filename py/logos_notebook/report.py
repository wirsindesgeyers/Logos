"""Relatório do caderno: rótulos epistêmicos (WS§11.1) e status consultados agora.

O relatório nunca usa `status_hint` como status atual e nunca chama evidência
ou hipótese de teorema: só um evento Verified do kernel é "provável sob Γ", e
sempre com as assumptions abertas listadas.
"""

from __future__ import annotations

import re
from typing import Any

from logos_client import air
from logos_notebook.bundle import Kind, Nature, Notebook, Role
from logos_notebook.lab import Card

NATURE_PT = {
    Nature.HYPOTHESIS: "hipótese (proposição investigável, não conclusão)",
    Nature.EMPIRICAL_EVIDENCE: "evidência empírica (observação com proveniência; não é φ)",
    Nature.SOURCE_SAYS: "fonte afirma φ (atribuição; não é φ)",
    Nature.MODEL_SUGGESTS: "modelo sugere φ (saída de LLM/modelo; não é φ)",
    Nature.USER_SAVED: "salvo pelo usuário (não é φ)",
    Nature.DERIVATION_REQUEST: "pedido de derivação (status vem do kernel)",
}

LABEL_PT = {
    "assumption": "assumption declarada (premissa aberta; não verificada)",
    "verified": "provável sem premissas abertas (Verified)",
    "verified_under_assumptions": "provável sob Γ (Verified condicional às assumptions)",
    "refuted": "refutada por contramodelo (Refuted)",
    "invalidated": "invalidada por retratação (não citável)",
    "retracted": "retratada (não citável)",
    "pending": "pendente",
    "unknown": "inconclusiva (Unknown)",
    "rejected": "certificado rejeitado",
}


def compact(formula: Any) -> str:
    """AIR-text normativo com predicados nulários abreviados; só apresentação."""
    text = air.print_formula(formula)
    return re.sub(r'pred\(symbol\("([^"]+)", pred, \[\], bool\), \[\]\)', r"\1", text)


def artifact_rows(notebook: Notebook, cards: dict[bytes, Card]) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for item in notebook.artifacts:
        refs = []
        for ref in item.claim_refs:
            card = cards.get(ref.event)
            refs.append(
                {
                    "role": ref.role.name.lower(),
                    "event": ref.event.hex(),
                    "obligation": None if ref.obligation is None else ref.obligation.hex(),
                    "current": None if card is None else card.label,
                    "citable": None if card is None else card.citable,
                    "open_assumptions": []
                    if card is None
                    else [event.hex() for event in card.open_assumptions],
                    "observed_seq": None if card is None else card.observed_seq,
                }
            )
        theorem = any(
            ref["role"] == "check" and ref["current"] in ("verified", "verified_under_assumptions")
            for ref in refs
        )
        rows.append(
            {
                "id": item.id.hex(),
                "kind": item.kind.name.lower(),
                "title": item.title,
                "statement_pt": item.statement_pt,
                "nature": NATURE_PT[item.nature],
                "formula": None if item.formula is None else compact(item.formula),
                "provenance": {
                    "source": item.provenance.source,
                    "locator": item.provenance.locator,
                    "method": item.provenance.method,
                    "actor": item.provenance.actor,
                    "recorded_at": item.provenance.recorded_at,
                },
                "provenance_complete": item.provenance.complete(),
                "limitations": item.limitations,
                "refs": refs,
                # Somente um Verified do kernel sustenta "provável sob Γ".
                "presented_as_provable": theorem,
                "status_hint_historical": None
                if item.status_hint is None
                else {"seq": item.status_hint.seq, "label": item.status_hint.label},
            }
        )
    return rows


def _status(row: dict[str, Any]) -> str:
    if not row["refs"]:
        return "nota sem claim (proveniência apenas; não verificada)"
    parts = []
    for ref in row["refs"]:
        text = LABEL_PT.get(ref["current"], str(ref["current"]))
        if ref["open_assumptions"] and ref["current"] == "verified_under_assumptions":
            text += f" — {len(ref['open_assumptions'])} assumptions abertas"
        parts.append(f"{ref['role']}: {text}")
    return "; ".join(parts)


def markdown(
    notebook: Notebook,
    rows: list[dict[str, Any]],
    sections: list[tuple[str, list[str]]],
) -> str:
    lines = [
        f"# Caderno: {notebook.project.title}",
        "",
        "Relatório não autoritativo. Status consultados no kernel no momento da",
        "geração; `status_hint` do bundle é histórico. Hipóteses e evidências",
        "nunca são teoremas: só um Verified do kernel é provável sob Γ.",
        "",
        f"- Escopo das assumptions: `{notebook.project.scope}`",
        f"- h(V): `{notebook.project.vocabulary.hex()}`",
        "",
    ]
    for kind in Kind:
        selected = [row for row in rows if row["kind"] == kind.name.lower()]
        if not selected:
            continue
        lines += [f"## {kind.name.title()} ({len(selected)})", ""]
        for row in selected:
            lines.append(f"- **{row['title']}** — {row['statement_pt']}")
            lines.append(f"  - natureza: {row['nature']}")
            if row["formula"] is not None:
                lines.append(f"  - AIR: `{row['formula']}`")
            source = row["provenance"]
            lines.append(
                f"  - proveniência: {source['source']}; {source['locator']}; "
                f"{source['method']}; {source['actor']}"
            )
            lines.append(f"  - status atual: {_status(row)}")
        lines.append("")
    for title, body in sections:
        lines += [f"## {title}", "", *body, ""]
    return "\n".join(lines)


def theorem_violations(rows: list[dict[str, Any]]) -> list[str]:
    """Itens que não podem aparecer como prováveis: hipótese, evidência, assumption."""
    forbidden = {"question", "hypothesis", "evidence", "assumption", "data_table"}
    problems = [
        row["title"] for row in rows if row["kind"] in forbidden and row["presented_as_provable"]
    ]
    problems += [
        row["title"]
        for row in rows
        if row["kind"] in forbidden
        and any(ref["role"] == Role.CHECK.name.lower() for ref in row["refs"])
    ]
    return problems
