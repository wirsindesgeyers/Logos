"""Etapa 5R.3 ponta a ponta: caderno piloto no kernel real, retratação e auditoria.

1. Vinte hipóteses e seis evidências do piloto com proveniência completa;
   hipóteses e evidências com fórmula entram como assumptions (ASSUME 5.7).
2. Previsão com duas rotas LRAT independentes (mesma h(O), `used` disjuntos),
   conclusões dependentes, experimento crucial, conflito mínimo e leis contra tabela.
3. O operador para o kernel e retrata duas evidências (`logos ledger retract`);
   o kernel reinicia, reconcilia e o caderno reconsulta tudo.
4. Auditor independente autentica/recheca o stream; o corte ATMS é recalculado
   do zero em Python e comparado com o kernel e com os registros Invalidated.
"""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import os
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any

from logos_client import air, canon
from logos_client.client import KernelClient
from logos_client.staging import StagingUploader
from logos_mcp.auditor import AuditConfig, Auditor
from logos_notebook import atms, ledger_view, pilot, report, science
from logos_notebook.bundle import (
    Artifact,
    Attachment,
    ClaimRef,
    Kind,
    Link,
    Nature,
    Notebook,
    Project,
    Provenance,
    Relation,
    Role,
    StatusHint,
    decode,
    new_id,
)
from logos_notebook.lab import Card, Lab
from logos_provers.farm import Sandbox

ROOT = Path(__file__).resolve().parents[2]


class Kernel:
    """Processo do kernel; o operador o para antes de retratar (writer.lock)."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.process: subprocess.Popen[bytes] | None = None
        self.log = (root / "kernel.log").open("ab")

    def start(self) -> None:
        socket = self.root / "s"
        socket.unlink(missing_ok=True)
        self.process = subprocess.Popen(
            [
                str(ROOT / "target/release/logos-kernel"),
                str(socket),
                str(self.root / "ledger"),
                str(self.root / "operator.key"),
                str(self.root / "staging"),
                str(self.root / "private"),
                str(os.geteuid()),
            ],
            stdout=self.log,
            stderr=self.log,
        )
        until = time.monotonic() + 120
        process = self.process
        while not socket.exists():
            if process.poll() is not None or time.monotonic() > until:
                raise RuntimeError((self.root / "kernel.log").read_text())
            time.sleep(0.02)

    def stop(self) -> None:
        if self.process is None:
            return
        self.process.terminate()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=10)
        self.process = None


def retract(root: Path, target: bytes, reason: str) -> dict[str, Any]:
    output = subprocess.run(
        [
            str(ROOT / "target/debug/logos"),
            "ledger",
            "retract",
            str(root / "ledger"),
            "--key",
            str(root / "operator.key"),
            "--target",
            target.hex(),
            "--reason",
            reason,
        ],
        capture_output=True,
        check=True,
        timeout=60,
    )
    return dict(json.loads(output.stdout))


def provenance(key: str, method: str) -> Provenance:
    return Provenance(
        pilot.SOURCE, f"logos_notebook/pilot.py#{key}", method, pilot.ACTOR, pilot.RECORDED_AT
    )


class Scenario:
    def __init__(self, root: Path, public: dict[str, str]) -> None:
        self.root = root
        self.vocabulary = bytes.fromhex(public["vocabulary_bytes"])
        self.client = KernelClient(root / "s", timeout=60)
        self.lab = Lab(
            self.client,
            StagingUploader(root / "staging"),
            Sandbox(ROOT / "target/debug/examples/prover_bridge", ROOT / "target/cadical/cadical"),
            self.vocabulary,
            timeout=30,
        )
        self.notebook = Notebook(
            Project(
                new_id("projeto", "piloto-igarape"),
                "Piloto 5R.3 — qualidade da água de um igarapé (fixture)",
                pilot.SCOPE,
                bytes.fromhex(public["vocabulary"]),
            )
        )
        self.events: dict[str, bytes] = {}
        self.ids: dict[str, bytes] = {}
        self.sections: list[tuple[str, list[str]]] = []
        self.evidence: dict[str, Any] = {}

    def add(self, key: str, artifact: Artifact) -> Artifact:
        self.ids[key] = artifact.id
        self.notebook.artifacts.append(artifact)
        return artifact

    def link(self, source: str, relation: Relation, target: str) -> None:
        self.notebook.links.add(Link(self.ids[source], relation, self.ids[target]))

    async def declare(self) -> None:
        for item, kind in [(h, Kind.HYPOTHESIS) for h in pilot.HYPOTHESES] + [
            (e, Kind.EVIDENCE) for e in pilot.EVIDENCE
        ]:
            nature = Nature.HYPOTHESIS if kind == Kind.HYPOTHESIS else Nature.EMPIRICAL_EVIDENCE
            if item.key in ("E4", "E6"):
                nature = Nature.SOURCE_SAYS
            formula = None if item.formula is None else self.lab.formula(item.formula)
            artifact = self.add(
                item.key,
                Artifact(
                    new_id(self.notebook.project.id, item.key),
                    kind,
                    nature,
                    f"{item.key} {item.title}",
                    item.statement,
                    provenance(item.key, item.method),
                    formula=formula,
                    limitations=[
                        "fixture sintética: sem valor empírico",
                        "assumption de sessão: premissa aberta, nunca teorema"
                        if formula is not None
                        else "nota de proveniência não verificada (sem fórmula)",
                    ],
                ),
            )
            if formula is not None:
                event = await self.lab.assume(formula, pilot.SCOPE)
                self.events[item.key] = event
                artifact.claim_refs.append(ClaimRef(Role.ASSUMPTION, event))

    async def derive(
        self,
        key: str,
        kind: Kind,
        title: str,
        goal: str,
        cites: list[str],
        *,
        hidden_sets: tuple[tuple[str, ...], ...] = ((),),
    ) -> list[bytes]:
        """Uma rota por conjunto escondido; todas sobre a mesma h(O)."""
        formula = self.lab.formula(goal)
        cited = tuple(self.events[name] for name in cites)
        routes: list[bytes] = []
        artifact = self.add(
            key,
            Artifact(
                new_id(self.notebook.project.id, key),
                kind,
                Nature.DERIVATION_REQUEST,
                f"{key} {title}",
                f"Derivação pedida ao kernel a partir de {', '.join(cites)}.",
                provenance(key, "PROPOSE + produtor SAT isolado + CERTIFY"),
                formula=formula,
                limitations=["provável só sob as assumptions abertas listadas"],
            ),
        )
        obligation: bytes | None = None
        for hidden in hidden_sets:
            proposal, result = await self.lab.prove(
                formula,
                cited,
                f"{key}: {title}",
                hide=tuple(self.events[name] for name in hidden),
            )
            if result is None or result.state != "verified":
                raise RuntimeError(f"{key}: route hiding {hidden} did not verify: {result}")
            if obligation is not None and proposal.obligation != obligation:
                raise RuntimeError("independent routes must share h(O)")
            obligation = proposal.obligation
            routes.append(result.event)
            artifact.claim_refs.append(ClaimRef(Role.CHECK, result.event, proposal.obligation))
        for name in cites:
            self.link(key, Relation.DERIVED_FROM, name)
        self.events[key] = routes[0]
        if len(routes) > 1:
            self.events[key + "B"] = routes[1]
        return routes

    async def used(self) -> dict[str, list[str]]:
        _, events = await self.lab.snapshot()
        view = ledger_view.read(events)
        names = {event: name for name, event in self.events.items()}
        return {
            name: sorted(names.get(p, p.hex()[:12]) for p in view.routes[event].used)
            for name, event in self.events.items()
            if event in view.routes
        }

    async def build(self) -> None:
        await self.declare()
        await self.derive(
            "P1",
            Kind.PREDICTION,
            "Oxigênio baixo previsto por dois mecanismos",
            "OxigenioBaixo()",
            ["E1", "E2", "H01", "H02", "H03", "E3", "E4", "H05", "H06"],
            hidden_sets=(("E3", "E4", "H05", "H06"), ("E1", "E2", "H01", "H02", "H03")),
        )
        for name in ("H01", "H02", "H03", "H05", "H06"):
            self.link(name, Relation.PREDICTS, "P1")
        self.link("E5", Relation.TESTS, "P1")
        await self.derive(
            "C1", Kind.CONCLUSION, "Mortandade prevista", "Mortandade()", ["P1", "H04"]
        )
        await self.derive(
            "C2", Kind.CONCLUSION, "Coliformes previstos", "Coliformes()", ["E1", "H07"]
        )
        # E3 é citada mas escondida do solver: citação não é dependência normativa.
        await self.derive(
            "C3",
            Kind.CONCLUSION,
            "Odor previsto",
            "Odor()",
            ["E1", "H09", "E3"],
            hidden_sets=(("E3",),),
        )
        await self.derive(
            "C4", Kind.CONCLUSION, "Turbidez prevista", "Turbidez()", ["E3", "E4", "H05"]
        )
        await self.derive(
            "C5", Kind.CONCLUSION, "Assoreamento previsto", "Assoreamento()", ["E4", "H08"]
        )
        self.evidence["used"] = await self.used()
        expected_used = {
            "P1": ["E1", "E2", "H01", "H02", "H03"],
            "P1B": ["E3", "E4", "H05", "H06"],
            "C1": ["H04", "P1"],
            "C2": ["E1", "H07"],
            "C3": ["E1", "H09"],
            "C4": ["E3", "E4", "H05"],
            "C5": ["E4", "H08"],
        }
        for name, used in expected_used.items():
            if self.evidence["used"][name] != used:
                raise RuntimeError(f"{name}: used {self.evidence['used'][name]} != {used}")
        await self.crucial()
        await self.conflict()
        await self.laws()

    async def crucial(self) -> None:
        h12, h13 = self.notebook.get(self.ids["H12"]), self.notebook.get(self.ids["H13"])
        result = await science.crucial_experiment(
            self.lab,
            (self.events["E5"],),
            h12.formula,
            h13.formula,
            pilot.OBSERVABLE,
            h1_event=self.events["H12"],
            h2_event=self.events["H13"],
        )
        self.link("H12", Relation.ALTERNATIVE_TO, "H13")
        certified = [row for row in result.predictions if row["certified"]]
        if {row["observable"] for row in certified} != {"Algas", "Turbidez"}:
            raise RuntimeError(f"crucial experiment: {result.predictions}")
        if any(s.event is None for s in result.separations):
            raise RuntimeError("crucial experiment needs both separating models")
        for row in certified:
            for side, hypothesis in (("h1", "H12"), ("h2", "H13")):
                key = f"X-{row['observable']}-{hypothesis}"
                value = row["under_h1"] if side == "h1" else not row["under_h1"]
                event = bytes.fromhex(row[side + "_event"])
                obligation = ledger_view.read((await self.lab.snapshot())[1]).routes[event]
                self.add(
                    key,
                    Artifact(
                        new_id(self.notebook.project.id, key),
                        Kind.PREDICTION,
                        Nature.DERIVATION_REQUEST,
                        f"Previsão discriminante: {row['observable']} sob {hypothesis}",
                        f"Com E5, {hypothesis} prevê {row['observable']} = "
                        f"{'sim' if value else 'não'}.",
                        provenance(key, "experimento crucial (WS§11.1 a)"),
                        formula=self.lab.formula(
                            f"{row['observable']}()" if value else f"not {row['observable']}()"
                        ),
                        claim_refs=[ClaimRef(Role.CHECK, event, obligation.obligation)],
                        limitations=["condicional a E5 e à hipótese; não decide entre elas"],
                    ),
                )
                self.link(hypothesis, Relation.PREDICTS, key)
        self.evidence["crucial"] = {
            "separations": [
                {
                    "direction": s.direction,
                    "refuted_event": None if s.event is None else s.event.hex(),
                    "observable": s.observable,
                    "diagnostics": s.diagnostics,
                }
                for s in result.separations
            ],
            "discriminators": result.discriminators,
            "predictions": result.predictions,
        }
        body = ["Pergunta: o oxigênio baixo (E5) vem da floração (H12) ou de sedimentos (H13)?", ""]
        for s in result.separations:
            body.append(f"### Cenário {s.direction} (Refuted aceito pelo kernel)")
            body.append("")
            assert s.observable is not None and s.explanation is not None
            shown = ", ".join(f"{k}={'sim' if v else 'não'}" for k, v in s.observable.items())
            body += [f"Observáveis no modelo: {shown}", "", "```text", s.explanation, "```", ""]
        body.append("Previsões discriminantes certificadas (Verified sob E5 e a hipótese):")
        for row in certified:
            side = "sim" if row["under_h1"] else "não"
            body.append(f"- {row['observable']}: H12 prevê {side}; H13 prevê o oposto.")
        noise = [r["observable"] for r in result.predictions if not r["certified"]]
        body.append(f"- Diferenças sem previsão certificada (arbitrárias nos modelos): {noise}")
        self.sections.append(("Experimento crucial (WS§11.1 a)", body))

    async def conflict(self) -> None:
        names = ["E5", "H12", "H13", "E2", "H01", "H07"]
        gamma = tuple(self.events[name] for name in names)
        result = await science.minimal_conflict(self.lab, gamma)
        if result is None:
            raise RuntimeError("expected an inconsistent set")
        by_event = {self.events[name]: name for name in names}
        core = sorted(by_event[event] for event in result.core)
        if core != ["E5", "H12", "H13"] or result.inconclusive:
            raise RuntimeError(f"unexpected core {core} / {result.inconclusive}")
        key = "K1"
        unsat_o = ledger_view.read((await self.lab.snapshot())[1]).routes[result.unsat_route]
        self.add(
            key,
            Artifact(
                new_id(self.notebook.project.id, key),
                Kind.CONCLUSION,
                Nature.DERIVATION_REQUEST,
                "K1 Conflito mínimo entre E5, H12 e H13",
                "E5, H12 e H13 juntas são inconsistentes; tirar qualquer uma resolve.",
                provenance(key, "MUS checado (WS§11.1 b): LRAT + modelos de S - {p}"),
                formula=self.lab.formula("false"),
                claim_refs=[ClaimRef(Role.CHECK, result.unsat_route, unsat_o.obligation)],
                limitations=["inconsistência relativa às fórmulas, não aos fenômenos"],
            ),
        )
        for name in core:
            self.link(key, Relation.CONFLICTS_WITH, name)
        self.evidence["conflict"] = {
            "gamma": names,
            "core": core,
            "unsat_route": result.unsat_route.hex(),
            "necessity": {by_event[p]: e.hex() for p, e in result.necessity.items()},
            "attempts": result.attempts,
        }
        self.sections.append(
            (
                "Conflito mínimo (WS§11.1 b)",
                [
                    f"Γ = {names}. O kernel aceitou Γ ⊢ ⊥ com used = {core} "
                    f"(rota `{result.unsat_route.hex()[:16]}`).",
                    "Cada subconjunto sem uma premissa tem modelo aceito (Refuted de S - {p} ⊢ ⊥):",
                    *[
                        f"- sem {by_event[p]}: `{e.hex()[:16]}`"
                        for p, e in result.necessity.items()
                    ],
                    f"Tentativas de encolhimento (premissas escondidas): {len(result.attempts)}.",
                ],
            )
        )

    async def laws(self) -> None:
        predicates = science.nullary_predicates(self.vocabulary)
        rows = science.parse_table(pilot.TABLE, predicates)
        table = self.add(
            "T1",
            Artifact(
                new_id(self.notebook.project.id, "T1"),
                Kind.DATA_TABLE,
                Nature.EMPIRICAL_EVIDENCE,
                "T1 Campanha de campo (8 pontos)",
                "Tabela de presença/ausência por ponto de coleta.",
                provenance("T1", "planilha de campo (fictícia)"),
                attachment=Attachment("text/csv", pilot.TABLE),
                limitations=["tabela fictícia; cada linha vira um model/v1"],
            ),
        )
        laws = [(name, self.lab.formula(text)) for name, text in pilot.LAWS]
        verdicts = await science.test_laws(self.lab, rows, laws, pilot.LAWS_SCOPE)
        _, events = await self.lab.snapshot()
        view = ledger_view.read(events)
        summary: dict[str, Any] = {}
        law_names = {v.assumption: v.name for v in verdicts if v.assumption is not None}
        body = ["Linhas da T1 viram model/v1; REFUTE certifica a linha que viola a lei."]
        for verdict, (name, formula) in zip(verdicts, laws, strict=True):
            refs: list[ClaimRef] = []
            if verdict.refutation is not None:
                refs.append(
                    ClaimRef(Role.CHECK, verdict.refutation, view.refuted[verdict.refutation])
                )
            if verdict.assumption is not None:
                refs.append(ClaimRef(Role.ASSUMPTION, verdict.assumption))
            if verdict.redundant_route is not None:
                route = view.routes[verdict.redundant_route]
                refs.append(ClaimRef(Role.CHECK, verdict.redundant_route, route.obligation))
            if verdict.independence is not None:
                refs.append(
                    ClaimRef(Role.CHECK, verdict.independence, view.refuted[verdict.independence])
                )
            self.add(
                name,
                Artifact(
                    new_id(self.notebook.project.id, name),
                    Kind.CANDIDATE_LAW,
                    Nature.MODEL_SUGGESTS,
                    f"{name} Lei candidata",
                    "Lei geral candidata sobre os pontos de coleta.",
                    provenance(name, "lei candidata proposta para teste (fictícia)"),
                    formula=formula,
                    claim_refs=refs,
                    limitations=[
                        "não refutada pela tabela ≠ verdadeira; redundância é relativa às demais"
                    ],
                ),
            )
            self.notebook.links.add(Link(self.ids[name], Relation.OBSERVED_IN, table.id))
            status = (
                f"refutada pela linha P{verdict.refuting_row + 1}"
                if verdict.refuting_row is not None
                else "redundante"
                if verdict.redundant_route is not None
                else "independente"
                if verdict.independence is not None
                else "inconclusiva"
            )
            summary[name] = {
                "status": status,
                "other_rows_local": [f"P{i + 1}" for i in verdict.other_rows],
                "redundant_used": sorted(law_names[u] for u in verdict.redundant_used),
                "diagnostics": verdict.diagnostics,
            }
            body.append(f"- {name} `{report.compact(formula)}`: {status}")
        expected = {
            "L1": "independente",
            "L2": "refutada pela linha P4",
            "L3": "independente",
            "L4": "redundante",
            "L5": "refutada pela linha P6",
            "L6": "independente",
            "L7": "redundante",
        }
        if {name: row["status"] for name, row in summary.items()} != expected:
            raise RuntimeError(f"laws: {summary}")
        self.evidence["laws"] = summary
        self.sections.append(("Leis candidatas contra tabela (WS§11.1 c)", body))

    async def cards(self) -> dict[bytes, Card]:
        _, events = await self.lab.snapshot()
        view = ledger_view.read(events)
        claims = list(view.assumptions) + list(view.routes) + list(view.refuted)
        return {event: await self.lab.card(event) for event in claims}


def kernel_dead(cards: dict[bytes, Card]) -> set[bytes]:
    return {event for event, card in cards.items() if card.retracted or card.kind == "invalidated"}


async def main_async(root: Path, public: dict[str, str], kernel: Kernel) -> dict[str, Any]:
    scenario = Scenario(root, public)
    await scenario.build()
    names = {event: name for name, event in scenario.events.items()}

    def named(events: set[bytes] | list[bytes]) -> list[str]:
        return sorted(names.get(e, e.hex()[:12]) for e in events)

    before = await scenario.cards()
    if kernel_dead(before):
        raise RuntimeError("nothing should be dead before retraction")
    # Salvar e reabrir: status_hint é histórico, IDs e referências preservados.
    for artifact in scenario.notebook.artifacts:
        if artifact.claim_refs:
            card = before.get(artifact.claim_refs[0].event)
            if card is not None:
                artifact.status_hint = StatusHint(card.observed_seq, card.label)
    saved = scenario.notebook.encode()
    reopened = decode(saved)
    if reopened.encode() != saved or len(reopened.artifacts) != len(scenario.notebook.artifacts):
        raise RuntimeError("bundle round trip")
    (root / "notebook.bundle").write_bytes(saved)

    steps: list[dict[str, Any]] = []
    expected_steps = [
        ("E3", "pluviômetro descalibrado (fixture)", ["C4", "E3"]),
        (
            "E1",
            "tubulação desativada antes da coleta (fixture)",
            ["C1", "C2", "C3", "E1", "P1", "P1B"],
        ),
    ]
    cards = before
    for target, reason, expected in expected_steps:
        kernel.stop()
        record = retract(root, scenario.events[target], reason)
        kernel.start()
        cards = await scenario.cards()
        dead = kernel_dead(cards)
        newly = named(dead - {e for step in steps for e in step["dead_events"]})
        if newly != expected:
            raise RuntimeError(f"retracting {target}: {newly} != {expected}")
        steps.append(
            {
                "target": target,
                "reason": reason,
                "cli": record,
                "newly_dead": newly,
                "dead_events": sorted(dead),
                "invalidated_records": len(record["invalidated"]),
            }
        )
    # Rota independente: depois da 1ª retratação, P1B (premissa morta) seguia citável.
    wire, events = await scenario.lab.snapshot()
    view = ledger_view.read(events)
    cut = atms.audit(view, kernel_dead(cards))
    if not cut.ok:
        raise RuntimeError(f"cut audit: {cut.problems}")
    survivors = [named(s.survivors) for s in cut.steps]
    if survivors[0] != ["P1B"]:
        raise RuntimeError(f"independent route survivors: {survivors}")
    for step, computed in zip(steps, cut.steps, strict=True):
        if named(computed.newly_dead) != step["newly_dead"]:
            raise RuntimeError("cut steps disagree with kernel")
    auditor = Auditor(
        AuditConfig(
            ROOT / "target/release/logos-audit",
            root / "ledger/cas",
            public["public_key"],
            ROOT / "target/cake-lpr/cake_lpr",
            (
                hashlib.sha256(
                    b"logos/checker/v1\0" + (ROOT / "target/release/logos-kernel").read_bytes()
                ).hexdigest(),
            ),
        )
    )
    source = root / "events.logos"
    source.write_bytes(wire)
    audit = await auditor.verify(source)
    if audit["events"] != len(events):
        raise RuntimeError("auditor did not cover the snapshot")

    rows = report.artifact_rows(scenario.notebook, cards)
    violations = report.theorem_violations(rows)
    hypotheses = [row for row in rows if row["kind"] == "hypothesis"]
    if len(hypotheses) != 20 or not all(row["provenance_complete"] for row in rows):
        raise RuntimeError("20 hypotheses with complete provenance required")
    if violations:
        raise RuntimeError(f"presented as theorem: {violations}")
    cut_body = [
        "Recalculado do zero sobre o stream autenticado (ADR0052) e comparado com o kernel:",
        "",
    ]
    for computed in cut.steps:
        target = names[computed.target]
        cut_body.append(f"- Retratação de {target} ({computed.reason}):")
        cut_body.append(f"  - perderam todas as rotas: {named(computed.newly_dead)}")
        cut_body.append(f"  - sobrevivem por rota independente: {named(computed.survivors)}")
    cut_body.append("")
    cut_body.append("Dependências normativas `used` versus citações:")
    for name in ("P1", "P1B", "C1", "C2", "C3", "C4", "C5"):
        route = view.routes[scenario.events[name]]
        cut_body.append(f"- {name}: cita {named(list(route.cited))}; usa {named(list(route.used))}")
    cut_body.append("")
    cut_body.append(
        f"Auditor independente: {audit['events']} eventos autenticados, "
        f"{audit['lrat_rust_rechecked']} LRAT rechecados em Rust e "
        f"{audit['lrat_cake_rechecked']} em cake_lpr, "
        f"{audit['refutations_rechecked']} refutações rechecadas."
    )
    scenario.sections.insert(0, ("Corte de retratação (5.9) e auditoria", cut_body))
    text = report.markdown(scenario.notebook, rows, scenario.sections)
    return {
        "hypotheses": len(hypotheses),
        "artifacts": len(rows),
        "relations": len(scenario.notebook.links),
        "provenance_complete": True,
        "presented_as_theorem": violations,
        "used": scenario.evidence["used"],
        "retractions": [{k: v for k, v in s.items() if k != "dead_events"} for s in steps],
        "cut_audit": {
            "steps": [
                {
                    "target": names[s.target],
                    "newly_dead": named(s.newly_dead),
                    "survivors": named(s.survivors),
                }
                for s in cut.steps
            ],
            "problems": cut.problems,
        },
        "crucial": scenario.evidence["crucial"],
        "conflict": scenario.evidence["conflict"],
        "laws": scenario.evidence["laws"],
        "audit": audit,
        "bundle_sha256": hashlib.sha256(saved).hexdigest(),
        "bundle_bytes": len(saved),
        "rows": rows,
        "markdown": text,
    }


def run(report_path: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="logos-notebook-") as directory:
        root = Path(directory)
        vocabulary = root / "public-vocabulary"
        vocabulary.write_bytes(canon.encode(air.parse_document(pilot.VOCABULARY).vocabulary))
        public = json.loads(
            subprocess.run(
                [str(ROOT / "target/debug/examples/client_fixture"), str(root), str(vocabulary)],
                capture_output=True,
                check=True,
                timeout=30,
            ).stdout
        )
        for name in ("staging", "private"):
            (root / name).mkdir(mode=0o700)
        kernel = Kernel(root)
        kernel.start()
        try:
            evidence = asyncio.run(main_async(root, public, kernel))
        finally:
            kernel.stop()
        report_path.parent.mkdir(parents=True, exist_ok=True)
        markdown = evidence.pop("markdown")
        report_path.with_suffix(".md").write_text(markdown)
        report_path.write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + "\n")
        bundle = (root / "notebook.bundle").read_bytes()
        report_path.with_suffix(".bundle").write_bytes(bundle)
        print(
            f"NOTEBOOK_SMOKE_OK hypotheses={evidence['hypotheses']} "
            f"artifacts={evidence['artifacts']} retractions=2 "
            f"cut={[s['newly_dead'] for s in evidence['cut_audit']['steps']]} "
            f"audit_events={evidence['audit']['events']}"
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, default=ROOT / "target/notebook-report.json")
    run(parser.parse_args().report)


if __name__ == "__main__":
    main()
