"""Recomputação independente do corte ATMS (ADR0052) a partir do stream de eventos.

Não usa o índice do kernel: recalcula do zero, por ponto fixo, quais claims
perderam todas as rotas de sustentação, e confere os registros Invalidated
gravados pelo kernel como caminhos `[h(Retracted raiz), premissa morta]`.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from logos_notebook.ledger_view import LedgerView


def dead_set(view: LedgerView, killed: set[bytes]) -> set[bytes]:
    """Claims mortos quando `killed` (alvos retratados) e checkers revogados caem.

    Nó = h(O). Um nó vive se alguma rota não morta diretamente tem todas as
    premissas `used` vivas; premissa viva = assumption não morta ou rota de nó
    vivo. Maior ponto fixo, recomputado do zero.
    """
    # 5.10: revogação só suspende; morrem rotas retratadas, rejeitadas no
    # recheck ou emitidas por checker já revogado.
    direct = set(killed) | view.born_dead
    alive = {route.obligation: True for route in view.routes.values()}

    def premise_alive(premise: bytes) -> bool:
        route = view.routes.get(premise)
        if route is not None:
            return alive[route.obligation]
        return premise in view.assumptions and premise not in direct

    changed = True
    while changed:
        changed = False
        for node in alive:
            if not alive[node]:
                continue
            supported = any(
                route.obligation == node
                and event not in direct
                and all(premise_alive(p) for p in route.used)
                for event, route in view.routes.items()
            )
            if not supported:
                alive[node] = False
                changed = True
    dead = {event for event in view.assumptions if event in direct}
    dead |= {
        event
        for event, route in view.routes.items()
        if event in direct or not alive[route.obligation]
    }
    return dead


@dataclass
class Step:
    retracted: bytes  # evento Retracted ou Rechecked rejeitado
    target: bytes
    reason: str
    newly_dead: list[bytes]
    survivors: list[bytes]  # rotas com premissa morta, mas nó vivo por outra rota


@dataclass
class CutAudit:
    steps: list[Step] = field(default_factory=list)
    dead: set[bytes] = field(default_factory=set)
    problems: list[str] = field(default_factory=list)

    @property
    def ok(self) -> bool:
        return not self.problems


def audit(view: LedgerView, kernel_dead: set[bytes]) -> CutAudit:
    """Compara o corte recalculado com o que o kernel responde e com os registros."""
    result = CutAudit()
    killed: set[bytes] = set()
    previous: set[bytes] = set()
    roots = {**view.recheck_rejected, **view.retracted}
    targets = sorted(roots.items(), key=lambda item: view.order.index(item[1]))
    for target, retraction in targets:
        killed.add(target)
        now = dead_set(view, killed)
        survivors = [
            event
            for event, route in view.routes.items()
            if event not in now and any(p in now for p in route.used)
        ]
        result.steps.append(
            Step(
                retraction,
                target,
                view.retraction_reason.get(target, "recheck rejeitado (5.10)"),
                [event for event in view.order if event in now - previous],
                [event for event in view.order if event in survivors],
            )
        )
        previous = now
    result.dead = dead_set(view, killed)
    claims = set(view.assumptions) | set(view.routes)
    if result.dead != kernel_dead & claims:
        missing = sorted(e.hex()[:12] for e in result.dead - kernel_dead)
        extra = sorted(e.hex()[:12] for e in (kernel_dead & claims) - result.dead)
        result.problems.append(f"kernel/recomputation disagree: missing={missing} extra={extra}")

    def root_of(premise: bytes) -> bytes | None:
        if premise in roots:
            return roots[premise]
        causes = view.invalidated.get(premise)
        return None if not causes else causes[0]

    for event, route in view.routes.items():
        record = view.invalidated.get(event)
        direct = event in roots or event in view.born_dead
        if event not in result.dead or direct:
            if record is not None and not direct:
                result.problems.append(f"Invalidated for a live route {event.hex()[:12]}")
            continue
        if record is None:
            result.problems.append(f"dead route without Invalidated {event.hex()[:12]}")
            continue
        if len(record) != 2:
            result.problems.append(f"causes is not [root, premise] for {event.hex()[:12]}")
            continue
        root, premise = record
        if premise not in route.used or premise not in result.dead:
            result.problems.append(f"cause is not a dead used premise for {event.hex()[:12]}")
        elif view.tags.get(root) not in (8, 15) or root_of(premise) != root:
            result.problems.append(f"cause path does not reach its root for {event.hex()[:12]}")
    for claim in view.invalidated:
        if claim not in view.routes:
            result.problems.append(f"Invalidated for a non-route {claim.hex()[:12]}")
    return result
