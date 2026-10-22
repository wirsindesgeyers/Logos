"""Etapa 5.9: cascata ATMS do ledger real contra referência ingênua em Python.

Gera um DAG de 10 000 claims (assumptions e rotas Verified, várias rotas por
h(O)), retrata alvos vivos um a um e compara, a cada passo, o conjunto
derrubado e as causas gravadas com uma recomputação do zero por ponto fixo.
A referência não usa nada incremental: recalcula todos os nós a cada passo.
"""

from __future__ import annotations

import json
import random
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CLAIMS = 10_000
RETRACTIONS = 25


def generate_dag(seed: int) -> tuple[list[dict[str, object]], dict[int, tuple[int, ...]]]:
    """Claims em ordem; rotas do mesmo nó compartilham `cites` (logo h(O))."""
    rng = random.Random(seed)
    claims: list[dict[str, object]] = []
    node_of: dict[int, tuple[int, ...]] = {}
    nodes: list[tuple[int, ...]] = []
    for index in range(CLAIMS):
        if index < 50 or rng.random() < 0.25:
            claims.append({"kind": "assumption"})
            continue
        if nodes and rng.random() < 0.3:
            cites = rng.choice(nodes)
        else:
            window = max(0, index - 200)
            cites = tuple(sorted(rng.sample(range(window, index), rng.randint(1, 3))))
            nodes.append(cites)
        used = sorted(rng.sample(cites, rng.randint(1, len(cites))))
        claims.append({"kind": "route", "cites": list(cites), "used": used})
        node_of[index] = cites
    return claims, node_of


def dead_set(
    claims: list[dict[str, object]],
    node_of: dict[int, tuple[int, ...]],
    retracted: set[int],
) -> set[int]:
    """Recomputação ingênua do zero: maior ponto fixo de nós vivos."""
    routes: dict[tuple[int, ...], list[int]] = {}
    for index, node in node_of.items():
        routes.setdefault(node, []).append(index)
    alive = dict.fromkeys(routes, True)

    def premise_alive(premise: int) -> bool:
        if premise in node_of:
            return alive[node_of[premise]]
        return premise not in retracted

    changed = True
    while changed:
        changed = False
        for node, members in routes.items():
            supported = any(
                route not in retracted and all(premise_alive(p) for p in claims[route]["used"])  # type: ignore[attr-defined]
                for route in members
            )
            if alive[node] and not supported:
                alive[node] = False
                changed = True
    dead = {i for i in range(len(claims)) if i not in node_of and i in retracted}
    dead |= {i for i, node in node_of.items() if i in retracted or not alive[node]}
    return dead


def test_retraction_cascade_matches_naive_reference_on_10k_dag(tmp_path: Path) -> None:
    claims, node_of = generate_dag(5_9)
    rng = random.Random(59)
    retract: list[int] = []
    retracted: set[int] = set()
    steps: list[set[int]] = []
    dead: set[int] = set()
    for _ in range(RETRACTIONS):
        target = rng.choice([i for i in range(CLAIMS) if i not in dead])
        retract.append(target)
        retracted.add(target)
        dead = dead_set(claims, node_of, retracted)
        steps.append(dead)
    source = tmp_path / "dag.json"
    output = tmp_path / "out.json"
    source.write_text(json.dumps({"claims": claims, "retract": retract}))
    subprocess.run(
        [
            "cargo",
            "run",
            "-q",
            "--release",
            "-p",
            "logos-kernel",
            "--example",
            "atms_fixture",
            "--",
            source,
            output,
        ],
        cwd=ROOT,
        check=True,
    )
    result = json.loads(output.read_text())
    hashes: list[str] = result["hashes"]

    final = {i for i, s in enumerate(result["status"]) if s == "invalidated"}
    assert final == steps[-1]
    assert len(final) > RETRACTIONS, "cascata precisa ir além dos alvos"
    survivors = [
        i
        for i in node_of
        if i not in final and any(p in final for p in claims[i]["used"])  # type: ignore[attr-defined]
    ]
    assert survivors, "precisa haver rota independente sobrevivendo a premissa derrubada"

    # Registros esperados, simulando a ordem real: um por rota derrubada que não é alvo,
    # com a menor premissa (por hash) cujo nó já estava morto nesse passo.
    members: dict[tuple[int, ...], list[int]] = {}
    for index, node in node_of.items():
        members.setdefault(node, []).append(index)

    def node_dead(premise: int, now: set[int]) -> bool:
        if premise in node_of:
            return all(r in now for r in members[node_of[premise]])
        return premise in now

    root_of: dict[int, int] = {}
    expected: list[tuple[int, int, int]] = []
    previous: set[int] = set()
    for step, (target, now) in enumerate(zip(retract, steps, strict=True)):
        root_of[target] = step
        for claim in sorted(now - previous):
            if claim not in node_of or claim in root_of:
                continue
            used: list[int] = claims[claim]["used"]  # type: ignore[assignment]
            premise = min((p for p in used if node_dead(p, now)), key=lambda p: hashes[p])
            root_of[claim] = root_of[premise]
            expected.append((claim, root_of[premise], premise))
        previous = now
    assert sorted(tuple(r) for r in result["records"]) == sorted(expected)
    print(
        f"ATMS_DIFFERENTIAL_OK claims={CLAIMS} routes={len(node_of)} nodes={len(members)} "
        f"retractions={RETRACTIONS} invalidated={len(final)} records={len(expected)} "
        f"independent_survivors={len(survivors)}"
    )
