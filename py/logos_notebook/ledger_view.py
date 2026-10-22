"""Leitura dos corpos de evento usados pelo caderno (spec/events.md §3); não autentica.

A autenticação do stream (assinaturas, cadeia, recheck de certificados) é do
auditor independente; este módulo só lê campos de um snapshot já capturado.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass, field

from logos_client import canon

PROPOSED, OBLIGATION, VERIFIED, REFUTED, UNKNOWN = 2, 3, 4, 5, 6
ASSUMPTION, RETRACTED, INVALIDATED, REVOKED, REJECTED = 7, 8, 9, 12, 13
RECHECKED = 15


class ViewError(ValueError):
    pass


@dataclass(frozen=True)
class Route:
    event: bytes
    obligation: bytes
    checker: bytes
    cited: tuple[bytes, ...]
    used: tuple[bytes, ...]


@dataclass
class LedgerView:
    """Eventos por ordem de seq, com os campos que a auditoria do corte usa."""

    order: list[bytes] = field(default_factory=list)
    tags: dict[bytes, int] = field(default_factory=dict)
    routes: dict[bytes, Route] = field(default_factory=dict)
    refuted: dict[bytes, bytes] = field(default_factory=dict)  # evento -> h(O)
    assumptions: dict[bytes, tuple[bytes, str]] = field(default_factory=dict)  # (h(V), scope)
    retracted: dict[bytes, bytes] = field(default_factory=dict)  # alvo -> evento Retracted
    retraction_reason: dict[bytes, str] = field(default_factory=dict)
    invalidated: dict[bytes, tuple[bytes, ...]] = field(default_factory=dict)  # claim -> causes
    revoked: set[bytes] = field(default_factory=set)
    # Verified de checker já revogado no momento do append nasce morta.
    born_dead: set[bytes] = field(default_factory=set)
    # 5.10: rota -> evento Rechecked rejeitado (mata a rota; raiz da cascata).
    recheck_rejected: dict[bytes, bytes] = field(default_factory=dict)


def _list(value: canon.Value, what: str) -> list[canon.Value]:
    if not isinstance(value, list):
        raise ViewError(what)
    return value


def _digest(value: canon.Value) -> bytes:
    if not isinstance(value, bytes) or len(value) != 32:
        raise ViewError("digest")
    return value


def _refs(value: canon.Value) -> tuple[bytes, ...]:
    return tuple(_digest(item) for item in _list(value, "refs"))


def read(events: list[bytes]) -> LedgerView:
    """`events` são bytes canônicos de envelopes event/v1 em ordem de seq."""
    view = LedgerView()
    for wire in events:
        envelope = _list(canon.decode(wire), "envelope")
        if len(envelope) != 8:
            raise ViewError("envelope arity")
        event = hashlib.sha256(b"logos/event/v1\0" + wire).digest()
        body = _list(envelope[5], "body")
        tag = body[1]
        if type(tag) is not int:
            raise ViewError("body tag")
        view.order.append(event)
        view.tags[event] = tag
        if tag == VERIFIED:
            check = _list(body[2], "check")
            if _digest(check[2]) in view.revoked:
                view.born_dead.add(event)
            view.routes[event] = Route(
                event,
                _digest(check[0]),
                _digest(check[2]),
                _refs(body[3]),
                _refs(body[4]),
            )
        elif tag == REFUTED:
            view.refuted[event] = _digest(_list(body[2], "check")[0])
        elif tag == ASSUMPTION:
            scope = body[4]
            if not isinstance(scope, str):
                raise ViewError("scope")
            view.assumptions[event] = (_digest(body[2]), scope)
        elif tag == RETRACTED:
            target = _digest(body[2])
            view.retracted.setdefault(target, event)
            reason = body[3]
            view.retraction_reason[target] = reason if isinstance(reason, str) else ""
        elif tag == INVALIDATED:
            view.invalidated[_digest(body[2])] = _refs(body[3])
        elif tag == REVOKED:
            view.revoked.add(_digest(body[2]))
        elif tag == RECHECKED:
            outcome = _list(body[3], "check")[4]
            # CheckOutcome core/v1: [1, 1, reason] é Rejected.
            if isinstance(outcome, list) and outcome[:2] == [1, 1]:
                view.recheck_rejected.setdefault(_digest(body[2]), event)
    return view
