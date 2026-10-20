"""Cliente async não confiável: uma conexão por operação, sem retry implícito."""

from __future__ import annotations

import asyncio
from collections.abc import AsyncGenerator
from pathlib import Path
from typing import cast

from pydantic import ValidationError

from logos_client import canon
from logos_client.protocol import (
    MAX_RESPONSE,
    Assume,
    Certify,
    CheckResult,
    EventRecord,
    Head,
    Message,
    Propose,
    ProposeResult,
    ProtocolError,
    QueryHead,
    QueryStatus,
    Refute,
    Request,
    Result,
    Status,
    Subscribe,
    SubscriptionEnd,
    Witness,
    WitnessResult,
    array,
    encode_request,
    parse_event,
    parse_result,
    uint,
)


class KernelError(Exception):
    """Erro remoto tipado; não é refutação nem indicação de que nada foi persistido."""

    def __init__(self, code: int, request_id: int) -> None:
        self.code, self.request_id = code, request_id
        names = ("Invalid", "Limit", "Unauthorized", "Unavailable", "Storage")
        super().__init__(f"kernel {names[code]} (request {request_id})")


class SubscriptionInterrupted(ConnectionError):
    """Cursor local de eventos completos; o caller decide se/quando reconectar."""

    def __init__(self, next_seq: int, previous_hash: bytes | None) -> None:
        self.next_seq, self.previous_hash = next_seq, previous_hash
        super().__init__(f"subscription interrupted; next complete-event cursor={next_seq}")


async def _response(reader: asyncio.StreamReader, request_id: int) -> canon.Value:
    prefix = await reader.readexactly(4)
    size = int.from_bytes(prefix, "big")
    if not 0 < size <= MAX_RESPONSE:
        raise ProtocolError("response frame size")
    payload = await reader.readexactly(size)
    try:
        v = array(canon.decode(payload), 4)
        if uint(v[0]) != 1 or uint(v[1]) != request_id:
            raise ProtocolError("response version/id mismatch")
        tag = uint(v[2], 1)
        if tag == 0:
            raise KernelError(uint(v[3], 4), request_id)
        return v[3]
    except canon.CanonError as error:
        raise ProtocolError("noncanonical response") from error


async def _close(writer: asyncio.StreamWriter) -> None:
    writer.close()
    try:
        async with asyncio.timeout(1):
            await writer.wait_closed()
    except ConnectionError, TimeoutError:
        # close() already releases transport ownership; do not mask an operation error.
        pass


class KernelClient:
    """Sem chave, ledger ou CAS privado; timeout inclui connect, write e read.

    Chamar no mesmo event loop. IDs correlacionam respostas, não dão idempotência.
    Timeout/cancelamento pode seguir um append confirmado: não repetir às cegas.
    """

    def __init__(self, socket: Path, *, timeout: float = 120) -> None:
        if type(timeout) not in (int, float) or not 0 < timeout <= 3600:
            raise ValueError("timeout must be in (0,3600]")
        self.socket, self.timeout, self._id = socket, timeout, 0

    def _request(self, message: Message) -> tuple[int, bytes]:
        request = Request(id=self._id, message=message)
        payload = encode_request(request)
        self._id += 1
        return request.id, payload

    async def request(self, message: Message) -> Result:
        if isinstance(message, Subscribe):
            raise ValueError("use subscribe() to consume the complete stream")
        request_id, payload = self._request(message)
        async with asyncio.timeout(self.timeout):
            reader, writer = await asyncio.open_unix_connection(self.socket)
            try:
                writer.write(payload)
                await writer.drain()
                value = await _response(reader, request_id)
                try:
                    return parse_result(message, value)
                except (canon.CanonError, ValidationError) as error:
                    raise ProtocolError("invalid result") from error
            finally:
                await _close(writer)

    async def propose(self, message: Propose) -> ProposeResult:
        return cast(ProposeResult, await self.request(message))

    async def certify(self, message: Certify) -> CheckResult:
        return cast(CheckResult, await self.request(message))

    async def refute(self, message: Refute) -> CheckResult:
        return cast(CheckResult, await self.request(message))

    async def witness(self, message: Witness) -> WitnessResult:
        return cast(WitnessResult, await self.request(message))

    async def assume(self, message: Assume) -> None:
        # The deployed kernel returns Unavailable; no success format exists yet.
        await self.request(message)
        raise ProtocolError("ASSUME success has no v1 contract")

    async def head(self) -> Head | None:
        return cast(Head | None, await self.request(QueryHead()))

    async def status(self, reference: bytes) -> Status | None:
        return cast(Status | None, await self.request(QueryStatus(reference=reference)))

    async def subscribe(
        self,
        from_seq: int,
        *,
        max_events: int = 256,
        previous_hash: bytes | None = None,
    ) -> AsyncGenerator[EventRecord | SubscriptionEnd]:
        message = Subscribe(from_seq=from_seq, max_events=max_events)
        if previous_hash is not None and (
            type(previous_hash) is not bytes or len(previous_hash) != 32
        ):
            raise ValueError("previous hash must be bytes32")
        request_id, payload = self._request(message)
        cursor, count, previous = from_seq, 0, previous_hash
        deadline = asyncio.get_running_loop().time() + self.timeout
        try:
            async with asyncio.timeout_at(deadline):
                reader, writer = await asyncio.open_unix_connection(self.socket)
            try:
                async with asyncio.timeout_at(deadline):
                    writer.write(payload)
                    await writer.drain()
                while True:
                    async with asyncio.timeout_at(deadline):
                        value = await _response(reader, request_id)
                    f = array(value, 2)
                    tag = uint(f[0], 1)
                    if tag == 1:
                        if uint(f[1]) != cursor:
                            raise ProtocolError("subscription terminal cursor mismatch")
                        yield SubscriptionEnd(next_seq=cursor)
                        return
                    event = parse_event(f[1])
                    if event.seq != cursor or count >= max_events:
                        raise ProtocolError("subscription sequence/count mismatch")
                    if (previous is not None and event.previous != previous) or (
                        cursor == 0 and event.previous is not None
                    ):
                        raise ProtocolError("subscription previous hash mismatch")
                    cursor += 1
                    count += 1
                    previous = event.hash
                    yield event
            finally:
                await _close(writer)
        except (asyncio.IncompleteReadError, ConnectionError, TimeoutError) as error:
            raise SubscriptionInterrupted(cursor, previous) from error
        except (canon.CanonError, ValidationError) as error:
            raise ProtocolError("invalid subscription result") from error
