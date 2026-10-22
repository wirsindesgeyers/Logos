"""Operações formais de auxílio à ciência (WS§11.1) sobre os checkers existentes.

Tudo aqui é produtor não confiável: escolhe obrigações e candidatos, mas cada
conclusão apresentada é um evento do kernel (Verified/Refuted). Ausência de
modelo ou de prova é inconclusiva, nunca validade nem inconsistência (L1).
"""

from __future__ import annotations

import csv
import io
from dataclasses import dataclass, field
from typing import Any

from logos_client import canon
from logos_client.client import KernelError
from logos_client.protocol import ProposeResult
from logos_notebook import ledger_view
from logos_notebook.lab import Lab
from logos_provers import model_context, model_explain
from logos_provers import model_finder as finder

# --- modelos proposicionais -------------------------------------------------


def nullary_predicates(vocabulary: bytes) -> list[str]:
    vocab = model_context._validate_vocabulary(vocabulary)
    if vocab.sorts:
        raise ValueError("table models need a vocabulary with nullary predicates only")
    names: list[str] = []
    for symbol in vocab.symbols:
        if symbol.kind != 2 or symbol.arity:
            raise ValueError("table models need a vocabulary with nullary predicates only")
        names.append(symbol.name)
    return names


def row_model(vocabulary: bytes, true: set[str]) -> bytes:
    """model/v1 de uma linha: predicados verdadeiros nela; os demais falsos."""
    vocab = model_context._validate_vocabulary(vocabulary)
    names = nullary_predicates(vocabulary)
    unknown = true - set(names)
    if unknown:
        raise ValueError(f"unknown predicates: {sorted(unknown)}")
    preds: list[canon.Value] = [
        [symbol.index, [[]] if symbol.name in true else []]
        for symbol in vocab.symbols
        if symbol.grounding_kind != 0
    ]
    return canon.encode([1, vocab.h_v, [], [], [], preds])


def true_predicates(model: bytes, vocabulary: bytes) -> set[str]:
    vocab = model_context._validate_vocabulary(vocabulary)
    interpreter = model_explain._interpreter(model, vocab)
    return {
        symbol.name
        for symbol in vocab.symbols
        if symbol.kind == 2 and not symbol.arity and interpreter.tables[symbol.index][0]
    }


def holds(formula: canon.Value, model: bytes, vocabulary: bytes) -> bool:
    """Avaliação local não confiável (2.2 em Python), só para escolher candidatos."""
    vocab = model_context._validate_vocabulary(vocabulary)
    interpreter = model_explain._interpreter(model, vocab)
    ast = model_context._parse_formula_envelope(formula)
    model_context._sort_check_formula(ast, [], vocab)
    try:
        return interpreter.formula(ast, [])
    except finder._Exhausted as error:
        raise ValueError("evaluation budget exhausted") from error


def parse_table(data: bytes, predicates: list[str]) -> list[set[str]]:
    """CSV estrito: cabeçalho `linha,<predicados>`, células `1`/`0`; nada é inferido."""
    try:
        text = data.decode("utf-8")
    except UnicodeError as error:
        raise ValueError("table must be UTF-8") from error
    rows = list(csv.reader(io.StringIO(text, newline="")))
    if not rows or rows[0][:1] != ["linha"]:
        raise ValueError("table header must start with 'linha'")
    header = rows[0][1:]
    if len(set(header)) != len(header) or not set(header) <= set(predicates):
        raise ValueError("table columns must be distinct vocabulary predicates")
    result: list[set[str]] = []
    names: set[str] = set()
    for row in rows[1:]:
        if len(row) != len(header) + 1 or row[0] in names or not row[0]:
            raise ValueError("table row arity/name")
        names.add(row[0])
        if any(cell not in ("0", "1") for cell in row[1:]):
            raise ValueError("table cells must be 0 or 1")
        result.append({name for name, cell in zip(header, row[1:], strict=True) if cell == "1"})
    if not result:
        raise ValueError("table has no rows")
    return result


# --- (a) experimento crucial -----------------------------------------------


@dataclass
class Separation:
    direction: str  # "H1 e não H2" ou "H2 e não H1"
    obligation: bytes
    event: bytes | None  # Refuted aceito pelo kernel
    observable: dict[str, bool] | None
    explanation: str | None
    diagnostics: list[str]


@dataclass
class CrucialExperiment:
    separations: list[Separation]
    discriminators: list[str]  # observáveis com valores diferentes nos dois cenários
    predictions: list[dict[str, Any]] = field(default_factory=list)


async def countermodel(
    lab: Lab, goal: canon.Value, cites: tuple[bytes, ...], text: str
) -> tuple[ProposeResult, bytes | None, bytes | None, list[str]]:
    """PROPOSE + produtores de modelo isolados + REFUTE; devolve (O, evento, M)."""
    proposal = await lab.propose(goal, cites, text)
    diagnostics: list[str] = []
    for mode in ("refute", "finder"):
        try:
            candidate = await lab.sandbox.produce(
                mode,
                proposal.obligation_bytes,
                lab.vocabulary,
                max(1, int(lab.timeout * 1000)),
            )
        except (ValueError, RuntimeError, OSError, TimeoutError) as error:
            diagnostics.append(f"{mode}: {error} (inconclusive)")
            continue
        if candidate is None or candidate.kind != "model":
            diagnostics.append(f"{mode}: no model within bound (inconclusive)")
            continue
        result = await lab.refute(proposal.obligation, candidate.payload)
        if result.state == "refuted":
            return proposal, result.event, candidate.payload, diagnostics
        diagnostics.append(f"{mode}: checker {result.state}")
    return proposal, None, None, diagnostics


async def crucial_experiment(
    lab: Lab,
    gamma: tuple[bytes, ...],
    h1: canon.Value,
    h2: canon.Value,
    observable: list[str],
    *,
    h1_event: bytes | None = None,
    h2_event: bytes | None = None,
) -> CrucialExperiment:
    """Modelos de Γ + {H1, não H2} e Γ + {não H1, H2} projetados nos observáveis.

    Com H1/H2 declaradas como assumptions, tenta ainda certificar previsões
    discriminantes: Γ,H1 ⊢ L e Γ,H2 ⊢ não L para um literal observável L.
    """
    separations: list[Separation] = []
    projections: list[set[str]] = []
    for direction, first, second in (("H1 e não H2", h1, h2), ("H2 e não H1", h2, h1)):
        goal = lab.implies(first, second)
        proposal, event, model, diagnostics = await countermodel(
            lab, goal, gamma, f"experimento crucial: cenário {direction}"
        )
        obligation = proposal.obligation
        if model is None:
            separations.append(Separation(direction, obligation, None, None, None, diagnostics))
            continue
        true = true_predicates(model, lab.vocabulary)
        projections.append(true)
        separations.append(
            Separation(
                direction,
                obligation,
                event,
                {name: name in true for name in observable},
                model_explain.explain_model(model, lab.vocabulary, proposal.obligation_bytes),
                diagnostics,
            )
        )
    discriminators = (
        [name for name in observable if (name in projections[0]) != (name in projections[1])]
        if len(projections) == 2
        else []
    )
    result = CrucialExperiment(separations, discriminators)
    if h1_event is None or h2_event is None or len(projections) != 2:
        return result
    for name in discriminators:
        atom = lab.formula(f"{name}()")
        negated = lab.formula(f"not {name}()")
        lit1, lit2 = (atom, negated) if name in projections[0] else (negated, atom)
        row: dict[str, Any] = {"observable": name, "under_h1": name in projections[0]}
        for key, hypothesis, literal in (("h1", h1_event, lit1), ("h2", h2_event, lit2)):
            _, farm = await lab.check(
                literal, (*gamma, hypothesis), f"previsão discriminante {name} sob {key}"
            )
            row[key] = None if farm.result is None else farm.result.state
            row[key + "_event"] = None if farm.result is None else farm.result.event.hex()
        row["certified"] = row["h1"] == "verified" and row["h2"] == "verified"
        result.predictions.append(row)
    return result


# --- (b) conflito mínimo checado ---------------------------------------------


@dataclass
class MinimalConflict:
    core: list[bytes]
    unsat_route: bytes  # Verified de Γ ⊢ ⊥ com used == core
    attempts: list[dict[str, Any]]
    necessity: dict[bytes, bytes]  # premissa -> Refuted de (core \ {p}) ⊢ ⊥
    inconclusive: list[bytes]


async def used_of(lab: Lab, event: bytes) -> tuple[bytes, ...]:
    _, events = await lab.snapshot()
    route = ledger_view.read(events).routes.get(event)
    if route is None:
        raise LookupError("event is not a Verified route")
    return route.used


async def minimal_conflict(lab: Lab, gamma: tuple[bytes, ...]) -> MinimalConflict | None:
    """Núcleo S ⊆ Γ: Γ ⊢ ⊥ por LRAT (used = S) e cada S - {p} consistente por modelo.

    O encolhimento usa o produtor SAT com premissas escondidas sobre a mesma
    O; o kernel recalcula `used` a cada rota. Minimalidade só é afirmada para
    premissas com Refuted aceito; as demais ficam inconclusivas.
    """
    false = lab.formula("false")
    _, first = await lab.prove(false, gamma, "conflito: Γ ⊢ ⊥")
    if first is None or first.state != "verified":
        return None
    route = first.event
    core = list(await used_of(lab, route))
    attempts: list[dict[str, Any]] = [{"hide": [], "route": route.hex(), "used": len(core)}]
    changed = True
    while changed:
        changed = False
        for premise in list(core):
            hide = tuple(sorted((set(gamma) - set(core)) | {premise}))
            _, attempt = await lab.prove(false, gamma, "conflito: encolhimento", hide=hide)
            entry: dict[str, Any] = {"hide": [h.hex() for h in hide], "route": None}
            if attempt is not None and attempt.state == "verified":
                route = attempt.event
                core = list(await used_of(lab, route))
                entry.update(route=route.hex(), used=len(core))
                attempts.append(entry)
                changed = True
                break
            attempts.append(entry)
    necessity: dict[bytes, bytes] = {}
    inconclusive: list[bytes] = []
    for premise in core:
        rest = tuple(p for p in core if p != premise)
        _, event, _, _ = await countermodel(lab, false, rest, "conflito: S sem uma premissa")
        if event is None:
            inconclusive.append(premise)
        else:
            necessity[premise] = event
    return MinimalConflict(core, route, attempts, necessity, inconclusive)


# --- (c) leis candidatas contra tabelas ----------------------------------------


@dataclass
class LawVerdict:
    name: str
    refuting_row: int | None = None
    refutation: bytes | None = None  # Refuted (linha como contramodelo)
    other_rows: list[int] = field(default_factory=list)  # avaliação local, não certificada
    assumption: bytes | None = None
    redundant_route: bytes | None = None  # Verified: segue das demais leis mantidas
    redundant_used: list[bytes] = field(default_factory=list)
    independence: bytes | None = None  # Refuted: as demais não a implicam
    diagnostics: list[str] = field(default_factory=list)


async def test_laws(
    lab: Lab,
    rows: list[set[str]],
    laws: list[tuple[str, canon.Value]],
    scope: str,
) -> list[LawVerdict]:
    verdicts: list[LawVerdict] = []
    for name, formula in laws:
        verdict = LawVerdict(name)
        violations = [
            index
            for index, row in enumerate(rows)
            if not holds(formula, row_model(lab.vocabulary, row), lab.vocabulary)
        ]
        if violations:
            first = violations[0]
            _, result = await lab.refute_with(
                formula,
                (),
                f"lei {name} contra a linha {first + 1} da tabela",
                row_model(lab.vocabulary, rows[first]),
            )
            if result.state == "refuted":
                verdict.refuting_row, verdict.refutation = first, result.event
                verdict.other_rows = violations[1:]
            else:
                verdict.diagnostics.append(f"row refutation: checker {result.state}")
        verdicts.append(verdict)
    survivors = [(v, f) for v, (_, f) in zip(verdicts, laws, strict=True) if v.refutation is None]
    for verdict, formula in survivors:
        verdict.assumption = await lab.assume(formula, scope)
    kept = [v for v, _ in survivors]
    for verdict, formula in survivors:
        others = tuple(v.assumption for v in kept if v is not verdict and v.assumption)
        try:
            _, farm = await lab.check(formula, others, f"redundância da lei {verdict.name}")
        except KernelError as error:
            verdict.diagnostics.append(f"redundancy: {error}")
            continue
        if farm.result is None:
            verdict.diagnostics.append("redundancy inconclusive")
        elif farm.result.state == "verified":
            verdict.redundant_route = farm.result.event
            verdict.redundant_used = list(await used_of(lab, farm.result.event))
            kept.remove(verdict)
        elif farm.result.state == "refuted":
            verdict.independence = farm.result.event
        else:
            verdict.diagnostics.append(f"redundancy: {farm.result.state}")
    return verdicts
