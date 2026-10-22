"""Operações de pesquisa fora da TCB; somente o kernel produz eventos/claims."""

from __future__ import annotations

import asyncio
import dataclasses
import tempfile
from contextlib import aclosing
from pathlib import Path
from typing import Any

from pydantic import BaseModel

from logos_client.client import KernelClient
from logos_client.protocol import (
    Certify,
    EventRecord,
    Propose,
    Refute,
    SubscriptionEnd,
    Witness,
)
from logos_client.staging import StagingUploader
from logos_mcp.auditor import MAX_STREAM, AuditFailure, Auditor
from logos_mcp.schemas import CheckInput, ObligationInput, ProposeInput, QueryInput, canonical
from logos_provers import model_context
from logos_provers.farm import Candidate, ProverFarm, Sandbox


def json_value(value: Any) -> Any:
    if isinstance(value, bytes):
        return value.hex()
    if isinstance(value, BaseModel):
        return json_value(value.model_dump())
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return json_value(dataclasses.asdict(value))
    if isinstance(value, dict):
        return {key: json_value(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [json_value(item) for item in value]
    return value


async def snapshot(client: KernelClient) -> tuple[bytes, str | None, int]:
    """Prefixo imutável em audit-stream/v1 via QueryHead/Subscribe; nunca abre o SQLite vivo."""
    head = await client.head()
    target = 0 if head is None else head.seq + 1
    wire = bytearray(b"LOGOS-AUDIT/v1\n")
    cursor, previous = 0, None
    while cursor < target:
        end = min(target, cursor + 256)
        async with aclosing(
            client.subscribe(
                cursor,
                max_events=end - cursor,
                previous_hash=previous,
            )
        ) as stream:
            terminal = False
            async for event in stream:
                if isinstance(event, SubscriptionEnd):
                    if event.next_seq != end or cursor != end:
                        raise AuditFailure("snapshot incomplete")
                    terminal = True
                    continue
                if event.seq != cursor or cursor >= target:
                    raise AuditFailure("snapshot sequence mismatch")
                wire.extend(len(event.canonical_bytes).to_bytes(4, "big"))
                wire.extend(event.canonical_bytes)
                if len(wire) > MAX_STREAM:
                    raise AuditFailure("snapshot limit")
                cursor += 1
                previous = event.hash
            if not terminal:
                raise AuditFailure("snapshot has no terminal cursor")
    expected = None if head is None else head.hash
    if previous != expected:
        raise AuditFailure("snapshot head mismatch")
    return bytes(wire), None if expected is None else expected.hex(), target


def frames(wire: bytes) -> list[bytes]:
    """Separa os eventos de um audit-stream/v1 capturado por `snapshot`."""
    header = b"LOGOS-AUDIT/v1\n"
    if not wire.startswith(header):
        raise AuditFailure("audit stream header")
    events: list[bytes] = []
    cursor = len(header)
    while cursor < len(wire):
        if cursor + 4 > len(wire):
            raise AuditFailure("truncated frame length")
        size = int.from_bytes(wire[cursor : cursor + 4], "big")
        cursor += 4
        if cursor + size > len(wire):
            raise AuditFailure("truncated frame")
        events.append(wire[cursor : cursor + size])
        cursor += size
    return events


class ResearchService:
    def __init__(
        self,
        client: KernelClient,
        uploader: StagingUploader,
        sandbox: Sandbox,
        vocabulary: bytes,
        auditor: Auditor,
        *,
        timeout: float = 30,
    ) -> None:
        if len(vocabulary) > 1_048_576:
            raise ValueError("public vocabulary limit")
        self.vocabulary = vocabulary
        self.vocabulary_hash = model_context._validate_vocabulary(vocabulary).h_v
        self.client, self.uploader, self.sandbox = client, uploader, sandbox
        self.auditor, self.timeout = auditor, timeout
        self.farm = ProverFarm(client, uploader, sandbox, timeout=timeout)
        self.lock = asyncio.Lock()

    def _context(self, request: ObligationInput) -> tuple[bytes, bytes]:
        if bytes.fromhex(request.vocabulary_bytes) != self.vocabulary:
            raise ValueError("vocabulary is not the configured public vocabulary")
        return bytes.fromhex(request.obligation_bytes), bytes.fromhex(request.obligation)

    async def propose(self, request: ProposeInput) -> dict[str, object]:
        if bytes.fromhex(request.vocabulary) != self.vocabulary_hash:
            raise ValueError("vocabulary is not configured")
        async with self.lock:
            result = await self.client.propose(
                Propose(
                    text_pt=request.text_pt,
                    goal=canonical(request.goal),
                    vocabulary=self.vocabulary_hash,
                    cites=tuple(bytes.fromhex(reference) for reference in request.cites),
                )
            )
        return {
            "source": "kernel",
            "authenticated": False,
            "obligation": result.obligation.hex(),
            "obligation_bytes": result.obligation_bytes.hex(),
            "vocabulary_bytes": self.vocabulary.hex(),
        }

    async def check(self, request: CheckInput) -> dict[str, object]:
        obligation, digest = self._context(request)
        async with self.lock:
            if request.certificate is not None:
                # No format/checker selection supplied by the model.
                result = await self.client.certify(
                    Certify(
                        obligation=digest,
                        format="lrat-rup/v1",
                        certificate=self.uploader.put(
                            bytes.fromhex(request.certificate), "certificate"
                        ),
                    )
                )
                return {
                    "source": "kernel",
                    "authenticated": False,
                    "result": json_value(result),
                    "consistency": "not_requested_by_this_direct_certificate_call",
                }
            portfolio = await self.farm.run(obligation, digest, self.vocabulary)
        return {"source": "kernel", "authenticated": False, **json_value(portfolio)}

    async def countermodel(self, request: ObligationInput) -> dict[str, object]:
        obligation, digest = self._context(request)
        diagnostics: list[str] = []
        async with self.lock:
            tasks: set[asyncio.Task[Candidate | None]] = {
                asyncio.create_task(
                    self.sandbox.produce(
                        mode, obligation, self.vocabulary, int(self.timeout * 1000)
                    )
                )
                for mode in ("refute", "finder")
            }
            try:
                async with asyncio.timeout(self.timeout):
                    while tasks:
                        done, _ = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
                        for task in done:
                            tasks.remove(task)
                            try:
                                candidate = task.result()
                            except ValueError, RuntimeError, OSError, TimeoutError:
                                diagnostics.append("producer_error_or_timeout (inconclusive)")
                                continue
                            if candidate is None or candidate.kind != "model":
                                diagnostics.append("no_model_within_bound (inconclusive)")
                                continue
                            witness = await self.client.witness(
                                Witness(
                                    obligation=digest,
                                    model=self.uploader.put(candidate.payload, "model"),
                                )
                            )
                            result = await self.client.refute(
                                Refute(
                                    obligation=digest,
                                    model=self.uploader.put(candidate.payload, "model"),
                                )
                            )
                            if result.state == "refuted":
                                return {
                                    "source": "kernel",
                                    "authenticated": False,
                                    "result": json_value(result),
                                    "witness": json_value(witness),
                                    "model": candidate.payload.hex(),
                                    "diagnostics": diagnostics,
                                }
                            diagnostics.append("checker_" + result.state)
            finally:
                for task in tasks:
                    task.cancel()
                await asyncio.gather(*tasks, return_exceptions=True)
        return {
            "source": "kernel",
            "authenticated": False,
            "result": None,
            "diagnostics": diagnostics,
        }

    async def query(self, request: QueryInput) -> dict[str, object]:
        if request.reference is not None:
            result = await self.client.status(bytes.fromhex(request.reference))
            return {"source": "kernel", "authenticated": False, "status": json_value(result)}
        if request.from_seq is None:
            return {
                "source": "kernel",
                "authenticated": False,
                "head": json_value(await self.client.head()),
            }
        events: list[dict[str, object]] = []
        total = 0
        previous = (
            bytes.fromhex(request.previous_hash) if request.previous_hash is not None else None
        )
        async with aclosing(
            self.client.subscribe(
                request.from_seq,
                max_events=request.max_events,
                previous_hash=previous,
            )
        ) as stream:
            async for event in stream:
                if isinstance(event, SubscriptionEnd):
                    return {
                        "source": "kernel",
                        "authenticated": False,
                        "events": events,
                        "next_seq": event.next_seq,
                    }
                assert isinstance(event, EventRecord)
                total += len(event.canonical_bytes)
                if total > MAX_STREAM:
                    raise ValueError("query output limit; request fewer events")
                events.append(
                    {
                        "hash": event.hash.hex(),
                        "seq": event.seq,
                        "canonical_bytes": event.canonical_bytes.hex(),
                    }
                )
        raise ValueError("subscription has no explicit terminal cursor")

    async def _audit_or_export(self, *, export: bool) -> dict[str, object]:
        async with self.lock, asyncio.timeout(self.auditor.timeout):
            wire, head, count = await snapshot(self.client)
            with tempfile.TemporaryDirectory(prefix="logos-mcp-snapshot-") as directory:
                source = Path(directory) / "events.logos"
                source.write_bytes(wire)
                if export:
                    result = await self.auditor.export(source)
                    report = result["audit"]
                else:
                    report = await self.auditor.verify(source)
                    result = {"audit": report}
                if (
                    not isinstance(report, dict)
                    or report["head"] != head
                    or report["events"] != count
                ):
                    raise AuditFailure("auditor did not verify the captured snapshot")
        return {"source": "independent_auditor", "snapshot_head": head, **result}

    async def audit(self) -> dict[str, object]:
        return await self._audit_or_export(export=False)

    async def export(self) -> dict[str, object]:
        return await self._audit_or_export(export=True)
