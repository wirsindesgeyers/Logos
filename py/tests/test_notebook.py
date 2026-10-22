"""5R.3: bundle notebook/v1, auditoria ATMS independente e rótulos do relatório."""

from __future__ import annotations

import hashlib
import json
import random
from pathlib import Path
from typing import Any

import pytest

from logos_client import air, canon
from logos_notebook import atms, bundle, ledger_view, pilot, report, science
from logos_notebook.bundle import (
    Artifact,
    Attachment,
    ClaimRef,
    Kind,
    Link,
    Nature,
    Notebook,
    NotebookError,
    Project,
    Provenance,
    Relation,
    Role,
    StatusHint,
)
from logos_notebook.lab import Card

ROOT = Path(__file__).resolve().parents[2]
VOCAB = air.parse_document(pilot.VOCABULARY).vocabulary
V_BYTES = canon.encode(VOCAB)
H_V = hashlib.sha256(b"logos/vocab/v1\0" + V_BYTES).digest()


def formula(text: str) -> canon.Value:
    return air.parse_formula(text, VOCAB)


def sample() -> Notebook:
    prov = Provenance("fonte", "pilot.py#X", "método", "ator", 1)
    notebook = Notebook(Project(b"\x01" * 16, "Projeto", "session:teste", H_V))
    notebook.artifacts += [
        Artifact(
            b"\x02" * 16,
            Kind.HYPOTHESIS,
            Nature.HYPOTHESIS,
            "H",
            "Esgoto eleva nutrientes.",
            prov,
            formula=formula("Esgoto() -> Nutrientes()"),
            claim_refs=[ClaimRef(Role.ASSUMPTION, b"\x0a" * 32)],
            limitations=["hipótese"],
        ),
        Artifact(
            b"\x03" * 16,
            Kind.CONCLUSION,
            Nature.DERIVATION_REQUEST,
            "C",
            "",
            prov,
            formula=formula("Nutrientes()"),
            claim_refs=[ClaimRef(Role.CHECK, b"\x0b" * 32, b"\x0c" * 32)],
            status_hint=StatusHint(7, "verified_under_assumptions"),
        ),
        Artifact(
            b"\x04" * 16,
            Kind.DATA_TABLE,
            Nature.EMPIRICAL_EVIDENCE,
            "T",
            "tabela",
            prov,
            attachment=Attachment("text/csv", b"linha,Esgoto\nP1,1\n"),
        ),
    ]
    notebook.links |= {
        Link(b"\x03" * 16, Relation.DERIVED_FROM, b"\x02" * 16),
        Link(b"\x02" * 16, Relation.PREDICTS, b"\x03" * 16),
    }
    return notebook


def raw(notebook: Notebook) -> Any:
    return canon.decode(notebook.encode())


def test_round_trip_digest_and_relation_order_are_canonical() -> None:
    data = sample().encode()
    again = bundle.decode(data)
    assert again.encode() == data
    assert bundle.digest(data) == hashlib.sha256(b"logos/notebook/v1\0" + data).digest()
    links: Any = raw(again)[4]
    assert links == sorted(links)
    assert again.get(b"\x03" * 16).status_hint == StatusHint(7, "verified_under_assumptions")


def mutate(edit: Any) -> bytes:
    value = raw(sample())
    edit(value)
    # canon.encode recusa texto não NFC: o marcador vira "cafe" + U+0301 nos bytes.
    return canon.encode(value).replace(b"cafeXX", "cafe\u0301".encode())


BAD: list[tuple[str, Any]] = [
    ("version", lambda v: v.__setitem__(0, 2)),
    ("tag", lambda v: v.__setitem__(1, "logos/other")),
    ("extra field", lambda v: v.append(0)),
    ("scope", lambda v: v[2].__setitem__(2, "global")),
    ("project id", lambda v: v[2].__setitem__(0, b"\x01" * 15)),
    ("unknown kind", lambda v: v[3][0].__setitem__(1, 99)),
    (
        "evidence as theorem",
        lambda v: (
            v[3][0].__setitem__(1, int(Kind.EVIDENCE)),
            v[3][0][8].append([1, b"\x0d" * 32, b"\x0e" * 32]),
        ),
    ),
    ("hypothesis as check", lambda v: v[3][0][8].__setitem__(0, [1, b"\x0a" * 32, b"\x0c" * 32])),
    ("check without h(O)", lambda v: v[3][1][8].__setitem__(0, [1, b"\x0b" * 32, None])),
    ("assumption with h(O)", lambda v: v[3][0][8].__setitem__(0, [0, b"\x0a" * 32, b"\x01" * 32])),
    ("conclusion without formula", lambda v: v[3][1].__setitem__(6, None)),
    ("bad AIR", lambda v: v[3][0].__setitem__(6, [1, 2, 3])),
    ("non-NFC text", lambda v: v[3][0].__setitem__(5, "cafeXX")),
    ("empty title", lambda v: v[3][0].__setitem__(4, "")),
    ("attachment on hypothesis", lambda v: v[3][0].__setitem__(10, ["text/csv", b""])),
    ("duplicate id", lambda v: v[3][1].__setitem__(0, b"\x02" * 16)),
    ("dangling relation", lambda v: v[4][0].__setitem__(2, b"\x09" * 16)),
    ("self relation", lambda v: v[4].__setitem__(0, [b"\x02" * 16, 0, b"\x02" * 16])),
    ("unsorted relations", lambda v: v[4].reverse()),
    ("duplicate relation", lambda v: v[4].append(v[4][-1])),
    ("unknown relation", lambda v: v[4][0].__setitem__(1, 77)),
    ("negative time", lambda v: v[3][0][7].__setitem__(4, -1)),
    ("status hint shape", lambda v: v[3][1].__setitem__(11, [7])),
]


@pytest.mark.parametrize("name,edit", BAD, ids=[name for name, _ in BAD])
def test_decoder_refuses_out_of_format_bundles(name: str, edit: Any) -> None:
    with pytest.raises(NotebookError):
        bundle.decode(mutate(edit))


def test_encoder_refuses_what_decoder_refuses() -> None:
    notebook = sample()
    notebook.artifacts[0].kind = Kind.EVIDENCE
    notebook.artifacts[0].claim_refs.append(ClaimRef(Role.CHECK, b"\x0d" * 32, b"\x0e" * 32))
    with pytest.raises(NotebookError, match="cannot carry"):
        notebook.encode()


def test_random_corruption_raises_only_notebook_error() -> None:
    data = sample().encode()
    rng = random.Random(53)
    accepted = 0
    for _ in range(3000):
        corrupt = bytearray(data)
        for _ in range(rng.randint(1, 4)):
            corrupt[rng.randrange(len(corrupt))] = rng.randrange(256)
        try:
            decoded = bundle.decode(bytes(corrupt))
        except NotebookError:
            continue
        accepted += 1
        assert decoded.encode() == bytes(corrupt), "accepted bytes must be canonical"
    assert accepted < 3000


def test_vectors_match_generator() -> None:
    path = ROOT / "spec/vectors/notebook/notebook.json"
    vectors = json.loads(path.read_text())
    for case in vectors["valid"]:
        data = bytes.fromhex(case["bundle"])
        assert bundle.digest(data).hex() == case["digest"]
        assert bundle.decode(data).encode() == data
    for case in vectors["invalid"]:
        with pytest.raises(NotebookError):
            bundle.decode(bytes.fromhex(case["bundle"]))


# --- ATMS independente -----------------------------------------------------


def h(name: str) -> bytes:
    return hashlib.sha256(name.encode()).digest()


def view_with(routes: dict[str, tuple[str, list[str]]], assumptions: list[str]) -> Any:
    view = ledger_view.LedgerView()
    for name in assumptions:
        view.order.append(h(name))
        view.tags[h(name)] = ledger_view.ASSUMPTION
        view.assumptions[h(name)] = (b"\0" * 32, "session:t")
    for name, (node, used) in routes.items():
        view.order.append(h(name))
        view.tags[h(name)] = ledger_view.VERIFIED
        view.routes[h(name)] = ledger_view.Route(
            h(name), h(node), b"\1" * 32, tuple(h(u) for u in used), tuple(h(u) for u in used)
        )
    return view


def retract(view: Any, target: str, causes: dict[str, tuple[str, str]]) -> None:
    root = h("R:" + target)
    view.order.append(root)
    view.tags[root] = ledger_view.RETRACTED
    view.retracted[h(target)] = root
    for claim, (cause_root, premise) in causes.items():
        view.invalidated[h(claim)] = (h("R:" + cause_root), h(premise))


CAUSES = {"PA": ("E1", "E1"), "PB": ("E3", "E3"), "C1": ("E1", "PA"), "C2": ("E3", "E3")}
ROUTES = {
    "PA": ("P", ["E1", "E2"]),
    "PB": ("P", ["E3"]),
    "C1": ("C", ["PA", "H"]),
    "C2": ("D", ["E3"]),
}


def test_independent_route_keeps_node_and_dependents() -> None:
    view = view_with(ROUTES, ["E1", "E2", "E3", "H"])
    assert atms.dead_set(view, {h("E1")}) == {h("E1")}
    assert atms.dead_set(view, {h("E1"), h("E3")}) == {
        h(n) for n in ("E1", "E3", "PA", "PB", "C1", "C2")
    }
    retract(view, "E1", {})
    retract(
        view, "E3", {"PA": ("E1", "E1"), "PB": ("E3", "E3"), "C1": ("E1", "PA"), "C2": ("E3", "E3")}
    )
    dead = {h(n) for n in ("E1", "E3", "PA", "PB", "C1", "C2")}
    result = atms.audit(view, dead)
    assert result.ok, result.problems
    assert [len(step.newly_dead) for step in result.steps] == [1, 5]
    assert result.steps[0].survivors == [h("PA")]


@pytest.mark.parametrize(
    "tamper",
    [
        "missing record",
        "record for live",
        "wrong premise",
        "wrong root",
        "kernel disagrees",
        "not a path",
    ],
)
def test_cut_audit_detects_wrong_records_and_statuses(tamper: str) -> None:
    view = view_with(ROUTES, ["E1", "E2", "E3", "H"])
    retract(view, "E1", {})
    retract(
        view, "E3", {"PA": ("E1", "E1"), "PB": ("E3", "E3"), "C1": ("E1", "PA"), "C2": ("E3", "E3")}
    )
    dead = {h(n) for n in ("E1", "E3", "PA", "PB", "C1", "C2")}
    if tamper == "missing record":
        del view.invalidated[h("C1")]
    elif tamper == "record for live":
        view.invalidated[h("H")] = (h("R:E1"), h("E1"))
    elif tamper == "wrong premise":
        view.invalidated[h("C1")] = (h("R:E1"), h("H"))
    elif tamper == "wrong root":
        view.invalidated[h("C2")] = (h("R:E1"), h("E3"))
    elif tamper == "kernel disagrees":
        dead.discard(h("C2"))
    else:
        view.invalidated[h("C1")] = (h("R:E1"),)
    assert not atms.audit(view, dead).ok


# --- relatório e tabelas -----------------------------------------------------


def card(event: bytes, kind: str, *, opened: tuple[bytes, ...] = ()) -> Card:
    return Card(event, kind, False, opened, 9)


def test_report_never_presents_evidence_or_hypothesis_as_theorem() -> None:
    notebook = sample()
    cards = {
        b"\x0a" * 32: card(b"\x0a" * 32, "assumption"),
        b"\x0b" * 32: card(b"\x0b" * 32, "verified", opened=(b"\x0a" * 32,)),
    }
    rows = report.artifact_rows(notebook, cards)
    assert report.theorem_violations(rows) == []
    by_kind = {row["kind"]: row for row in rows}
    assert by_kind["conclusion"]["presented_as_provable"]
    assert by_kind["conclusion"]["refs"][0]["current"] == "verified_under_assumptions"
    assert not by_kind["hypothesis"]["presented_as_provable"]
    text = report.markdown(notebook, rows, [])
    assert "provável sob Γ" in text and "status atual" in text
    # status_hint fica rotulado como histórico, nunca como status atual.
    assert by_kind["conclusion"]["status_hint_historical"]["seq"] == 7
    # Uma hipótese com referência CHECK verificada seria violação.
    rows[0]["refs"].append({"role": "check", "current": "verified"})
    rows[0]["presented_as_provable"] = True
    assert report.theorem_violations(rows) == ["H", "H"]


def test_table_parsing_is_strict_and_rows_become_models() -> None:
    predicates = science.nullary_predicates(V_BYTES)
    rows = science.parse_table(pilot.TABLE, predicates)
    assert len(rows) == 8 and rows[3] == {"Esgoto", "Coliformes"}
    for bad in (
        b"ponto,Esgoto\nP1,1\n",
        b"linha,Esgoto\nP1,2\n",
        b"linha,Esgoto,Esgoto\nP1,1,1\n",
        b"linha,Inexistente\nP1,1\n",
        b"linha,Esgoto\nP1,1\nP1,0\n",
        b"linha,Esgoto\n",
        b"linha,Esgoto\nP1\n",
    ):
        with pytest.raises(ValueError):
            science.parse_table(bad, predicates)
    model = science.row_model(V_BYTES, rows[3])
    assert science.true_predicates(model, V_BYTES) == {"Esgoto", "Coliformes"}
    assert not science.holds(formula("Esgoto() -> Odor()"), model, V_BYTES)
    assert science.holds(formula("Esgoto() -> Coliformes()"), model, V_BYTES)
    with pytest.raises(ValueError):
        science.row_model(V_BYTES, {"Inexistente"})


def test_revocation_only_suspends_and_rejected_recheck_is_a_cut_root() -> None:
    """5.10: revogação não mata; Rechecked rejeitado mata e é raiz dos registros."""
    view = view_with(ROUTES, ["E1", "E2", "E3", "H"])
    view.revoked.add(b"\1" * 32)  # checker de todas as rotas
    assert atms.dead_set(view, set()) == set()
    root = h("RC:PB")
    view.order.append(root)
    view.tags[root] = ledger_view.RECHECKED
    view.recheck_rejected[h("PB")] = root
    dead = {h("PB")}
    result = atms.audit(view, dead)
    assert result.ok, result.problems
    assert [s.target for s in result.steps] == [h("PB")]
    assert result.steps[0].survivors == []
