"""Operações do caderno contra o kernel real; só o kernel produz eventos e claims.

Mesmos componentes da 5R.1 (cliente 10.1, farm 10.2, staging): PROPOSE,
CERTIFY, REFUTE e ASSUME são mensagens normais do protocolo. Retratação não
passa por aqui: é operação de operador (`logos ledger retract`, ADR0052).
"""

from __future__ import annotations

from dataclasses import dataclass

from logos_client import air, canon
from logos_client.client import KernelClient, KernelError
from logos_client.protocol import (
    Assume,
    Certify,
    CheckResult,
    ProposeResult,
    Refute,
)
from logos_client.protocol import (
    Propose as ProposeMessage,
)
from logos_client.staging import StagingUploader
from logos_mcp.service import frames, snapshot
from logos_provers import model_context
from logos_provers.farm import FarmResult, ProverFarm, Sandbox

KIND_NAMES = (
    "fact",
    "policy",
    "verified",
    "assumption",
    "pending",
    "refuted",
    "unknown",
    "rejected",
    "invalidated",
    "proposal",
    "suspended",
)


@dataclass(frozen=True)
class Card:
    """Status consultado agora no kernel; não é Seal nem atestado."""

    event: bytes
    kind: str
    retracted: bool
    open_assumptions: tuple[bytes, ...]
    observed_seq: int

    @property
    def citable(self) -> bool:
        return self.kind in ("fact", "policy", "verified", "assumption") and not self.retracted

    @property
    def label(self) -> str:
        if self.retracted:
            return "retracted"
        if self.kind == "verified" and self.open_assumptions:
            return "verified_under_assumptions"
        return self.kind


class Lab:
    def __init__(
        self,
        client: KernelClient,
        uploader: StagingUploader,
        sandbox: Sandbox,
        vocabulary: bytes,
        *,
        timeout: float = 30,
    ) -> None:
        self.client, self.uploader, self.sandbox = client, uploader, sandbox
        self.vocabulary = vocabulary
        self.vocabulary_hash = model_context._validate_vocabulary(vocabulary).h_v
        self.vocabulary_value = canon.decode(vocabulary)
        self.timeout = timeout
        self.farm = ProverFarm(client, uploader, sandbox, timeout=timeout)
        self._pending: dict[tuple[bytes, tuple[bytes, ...]], ProposeResult] = {}

    def formula(self, text: str) -> canon.Value:
        """AIR-text → AIR no vocabulário vigente (sort-check fica com o kernel)."""
        return air.parse_formula(text, self.vocabulary_value)

    async def assume(self, formula: canon.Value, scope: str) -> bytes:
        result = await self.client.assume(
            Assume(vocabulary=self.vocabulary_hash, formula=canon.encode(formula), scope=scope)
        )
        return result.event

    async def propose(
        self, goal: canon.Value, cites: tuple[bytes, ...], text: str
    ) -> ProposeResult:
        return await self.client.propose(
            ProposeMessage(
                text_pt=text,
                goal=canon.encode(goal),
                vocabulary=self.vocabulary_hash,
                cites=cites,
            )
        )

    async def check(
        self, goal: canon.Value, cites: tuple[bytes, ...], text: str
    ) -> tuple[ProposeResult, FarmResult]:
        proposal = await self.propose(goal, cites, text)
        result = await self.farm.run(
            proposal.obligation_bytes, proposal.obligation, self.vocabulary
        )
        return proposal, result

    async def prove(
        self,
        goal: canon.Value,
        cites: tuple[bytes, ...],
        text: str,
        *,
        hide: tuple[bytes, ...] = (),
    ) -> tuple[ProposeResult, CheckResult | None]:
        """Rota LRAT cujo solver não vê `hide`; o kernel recalcula `used`.

        Repropor o mesmo objetivo com as mesmas citações dá a mesma h(O) e uma
        nova instância Pending: é assim que nasce uma rota independente.
        """
        key = (canon.encode(goal), cites)
        # Uma instância Pending ainda não certificada é reaproveitada: tentativas
        # sem prova (ex.: encolhimento do MUS) não deixam Pending órfãos.
        proposal = self._pending.get(key)
        fresh = proposal is None
        if proposal is None:
            proposal = await self.propose(goal, cites, text)
        candidate = await self.sandbox.produce(
            "sat",
            proposal.obligation_bytes,
            self.vocabulary,
            max(1, int(self.timeout * 1000)),
            hide=hide,
        )
        if candidate is None or candidate.kind != "proof":
            self._pending[key] = proposal
            return proposal, None
        if not fresh:
            del self._pending[key]
        result = await self.client.certify(
            Certify(
                obligation=proposal.obligation,
                format="lrat-rup/v1",
                certificate=self.uploader.put(candidate.payload, "certificate"),
            )
        )
        return proposal, result

    def implies(self, first: canon.Value, second: canon.Value) -> canon.Value:
        return self.formula(f"({air.print_formula(first)}) -> ({air.print_formula(second)})")

    async def refute(self, obligation: bytes, model: bytes) -> CheckResult:
        return await self.client.refute(
            Refute(obligation=obligation, model=self.uploader.put(model, "model"))
        )

    async def refute_with(
        self, goal: canon.Value, cites: tuple[bytes, ...], text: str, model: bytes
    ) -> tuple[ProposeResult, CheckResult]:
        """REFUTE com um modelo dado (ex.: linha de tabela); o kernel o recheca."""
        proposal = await self.propose(goal, cites, text)
        return proposal, await self.refute(proposal.obligation, model)

    async def card(self, event: bytes) -> Card:
        head = await self.client.head()
        status = await self.client.status(event)
        if status is None:
            raise LookupError(f"event {event.hex()} is not in the ledger")
        try:
            opened = await self.client.open_assumptions(event)
        except KernelError:
            opened = None  # claim sem fecho consultável (ex.: invalidado)
        return Card(
            event,
            KIND_NAMES[status.kind],
            status.retracted,
            () if opened is None else tuple(item.event for item in opened.assumptions),
            -1 if head is None else head.seq,
        )

    async def snapshot(self) -> tuple[bytes, list[bytes]]:
        wire, _, _ = await snapshot(self.client)
        return wire, frames(wire)
