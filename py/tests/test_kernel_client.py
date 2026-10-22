"""Cliente não confiável: vetores independentes e peers Unix adversários."""

from __future__ import annotations

import asyncio
import hashlib
import json
import os
import stat
from collections.abc import AsyncIterator, Awaitable, Callable
from contextlib import aclosing, asynccontextmanager
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest
from pydantic import ValidationError

from logos_client import canon
from logos_client.client import KernelClient, KernelError, SubscriptionInterrupted
from logos_client.protocol import (
    MAX_RESPONSE,
    CasRef,
    Certify,
    CheckResult,
    EventRecord,
    Propose,
    ProposeResult,
    ProtocolError,
    QueryHead,
    Refute,
    Request,
    StagedRef,
    Subscribe,
    SubscriptionEnd,
    Witness,
    WitnessResult,
    array,
    decode_request,
    encode_request,
    parse_event,
    parse_result,
)
from logos_client.staging import StagingUploader

ROOT = Path(__file__).resolve().parents[2]
HASH = bytes(32)
TRUE = canon.encode([1, 3, "FOL-ms-eq/v1", [[0]]])


def test_all_independent_request_vectors_and_strict_model_revalidation() -> None:
    vectors = json.loads((ROOT / "spec/vectors/kernel-protocol/vectors.json").read_text())[
        "vectors"
    ]
    seen = set()
    for vector in vectors:
        wire = bytes.fromhex(vector["wire_hex"])
        if vector["valid"]:
            request = decode_request(wire)
            seen.add(request.message.kind)
            assert encode_request(request) == bytes.fromhex(vector["frame_hex"])
        else:
            with pytest.raises((ValueError, canon.CanonError, UnicodeError)):
                decode_request(wire)
    assert seen == {
        "propose",
        "certify",
        "refute",
        "witness",
        "assume",
        "head",
        "status",
        "open",
        "subscribe",
    }
    for value in [True, -1, 2**64, "1"]:
        with pytest.raises(ValidationError):
            Request.model_validate({"id": value, "message": QueryHead()})
    request = Request(id=0, message=QueryHead())
    with pytest.raises(ValidationError):
        encode_request(request.model_copy(update={"id": True}))
    with pytest.raises(ValidationError):
        Propose(text_pt="ok", goal=TRUE, vocabulary=HASH, undocumented=True)  # type: ignore[call-arg]
    with pytest.raises(ValidationError):
        Subscribe(from_seq=0, max_events=True)
    with pytest.raises(ValidationError):
        StagedRef(hash=HASH, name="../" + "a" * 61)
    with pytest.raises(ValidationError):
        Propose(text_pt="é" * 32769, goal=TRUE, vocabulary=HASH)
    tainted = Propose(text_pt="cafe\u0301", goal=TRUE, vocabulary=HASH)
    assert decode_request(encode_request(Request(id=0, message=tainted))[4:]).message == tainted


def test_independent_result_vectors_are_not_interchangeable_or_credentials() -> None:
    proof = CasRef(hash=HASH)
    for name in ["check", "witness"]:
        data = json.loads(
            (ROOT / f"spec/vectors/kernel-protocol/{name}-result-v1.json").read_text()
        )
        for vector in data["vectors"]:
            value = canon.decode(bytes.fromhex(vector["value_hex"]))
            tag = vector["tag"]
            message = (
                Witness(obligation=HASH, model=proof)
                if name == "witness"
                else (
                    Refute(obligation=HASH, model=proof)
                    if tag == 1
                    else Certify(obligation=HASH, format="lrat-rup/v1", certificate=proof)
                )
            )
            result = parse_result(message, value)
            assert isinstance(result, WitnessResult if name == "witness" else CheckResult)
            assert result.event.hex() == vector["event_hash_hex"]
    vectors = json.loads(
        (ROOT / "spec/vectors/kernel-protocol/propose-result-v1.json").read_text()
    )["vectors"]
    for vector in vectors:
        request = decode_request(bytes.fromhex(vector["request_frame_hex"])[4:])
        value = array(canon.decode(bytes.fromhex(vector["response_hex"])), 4)[3]
        result = parse_result(request.message, value)
        assert isinstance(result, ProposeResult)
        assert result.obligation_bytes.hex() == vector["obligation_hex"]
    with pytest.raises(ProtocolError):
        parse_result(Certify(obligation=HASH, format="x", certificate=proof), [1, 1, HASH])
    with pytest.raises(ProtocolError):
        parse_result(Refute(obligation=HASH, model=proof), [1, 0, HASH])
    with pytest.raises(ValidationError):
        ProposeResult(obligation=HASH, obligation_bytes=b"\x80")


def test_staging_domains_modes_symlinks_collisions_and_cleanup(tmp_path: Path) -> None:
    folder = tmp_path / "staging"
    folder.mkdir(mode=0o700)
    uploader = StagingUploader(folder, max_bytes=10)
    ref = uploader.put(b"hello", "model")
    path = folder / ref.name
    assert path.read_bytes() == b"hello"
    assert stat.S_IMODE(path.stat().st_mode) == 0o600 and path.stat().st_nlink == 1
    assert ref.hash == hashlib.sha256(b"logos/model/v1\0hello").digest()
    cert = uploader.put(b"hello", "certificate")
    assert cert.hash != ref.hash
    with pytest.raises(ValueError):
        uploader.put(bytes(11), "model")
    link = tmp_path / "link"
    link.symlink_to(folder, target_is_directory=True)
    with pytest.raises(OSError):
        StagingUploader(link).put(b"x", "model")
    folder.chmod(0o770)
    with pytest.raises(PermissionError):
        uploader.put(b"x", "model")
    folder.chmod(0o700)
    # Collision in either name must never unlink or overwrite someone else's entry.
    for collision in [ref.name, ".upload-" + ref.name]:
        existing = folder / collision
        if not existing.exists():
            existing.write_bytes(b"preserve")
        original = existing.read_bytes()
        with (
            patch("logos_client.staging.secrets.token_hex", return_value=ref.name),
            pytest.raises(FileExistsError),
        ):
            uploader.put(b"x", "model")
        assert existing.read_bytes() == original
    private = folder / (".upload-" + ref.name)
    private.unlink()
    before = set(folder.iterdir())
    with (
        patch("logos_client.staging.os.write", return_value=0),
        pytest.raises(OSError, match="short"),
    ):
        uploader.put(b"x", "model")
    assert set(folder.iterdir()) == before
    assert os.geteuid() == path.stat().st_uid


type Peer = Callable[[asyncio.StreamReader, asyncio.StreamWriter], Awaitable[None]]


@asynccontextmanager
async def peer(path: Path, callback: Peer) -> AsyncIterator[None]:
    tasks: set[asyncio.Task[None]] = set()

    async def serve(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
        task = asyncio.current_task()
        assert task is not None
        tasks.add(task)
        try:
            await callback(reader, writer)
        finally:
            writer.close()
            await writer.wait_closed()
            tasks.remove(task)

    server = await asyncio.start_unix_server(serve, path=path)
    async with server:
        try:
            yield
        finally:
            for task in tasks:
                task.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)


async def incoming(reader: asyncio.StreamReader) -> Request:
    prefix = await reader.readexactly(4)
    return decode_request(await reader.readexactly(int.from_bytes(prefix, "big")))


def frame(request_id: int, value: canon.Value, *, success: int = 1) -> bytes:
    wire = canon.encode([1, request_id, success, value])
    return len(wire).to_bytes(4, "big") + wire


@pytest.mark.parametrize(
    "failure",
    [
        "id",
        "bool_id",
        "version",
        "truncated",
        "zero",
        "large",
        "error",
        "bad_status",
        "noncanonical",
    ],
)
def test_peer_response_failures_are_explicit(tmp_path: Path, failure: str) -> None:
    async def run() -> None:
        async def reply(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
            request = await incoming(reader)
            data = frame(request.id, [])
            if failure == "id":
                data = frame(request.id + 1, [])
            elif failure == "bool_id":
                raw = canon.encode([1, False, 1, []])
                data = len(raw).to_bytes(4, "big") + raw
            elif failure == "version":
                raw = canon.encode([2, request.id, 1, []])
                data = len(raw).to_bytes(4, "big") + raw
            elif failure == "truncated":
                data = data[:-1]
            elif failure == "zero":
                data = bytes(4)
            elif failure == "large":
                data = (MAX_RESPONSE + 1).to_bytes(4, "big")
            elif failure == "error":
                data = frame(request.id, 3, success=0)
            elif failure == "bad_status":
                data = frame(request.id, [False, 0, HASH])
            elif failure == "noncanonical":
                raw = b"\x84\x18\x01" + canon.encode(request.id) + b"\x01\x80"
                data = len(raw).to_bytes(4, "big") + raw
            writer.write(data)
            await writer.drain()

        async with peer(tmp_path / "s", reply):
            error = (
                KernelError
                if failure == "error"
                else (asyncio.IncompleteReadError if failure == "truncated" else ProtocolError)
            )
            with pytest.raises(error):
                await KernelClient(tmp_path / "s").head()

    asyncio.run(run())


def test_fragmented_response_and_cancellation_have_no_hidden_retry(tmp_path: Path) -> None:
    async def run() -> None:
        calls = 0
        received = asyncio.Event()
        closed = asyncio.Event()

        async def reply(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
            nonlocal calls
            request = await incoming(reader)
            calls += 1
            if calls == 1:
                for byte in frame(request.id, [7, 0, HASH]):
                    writer.write(bytes([byte]))
                    await writer.drain()
                    await asyncio.sleep(0)
            else:
                received.set()
                assert await reader.read() == b""
                closed.set()

        async with peer(tmp_path / "s", reply):
            client = KernelClient(tmp_path / "s")
            head = await client.head()
            assert head is not None and head.seq == 7
            task = asyncio.create_task(client.head())
            await received.wait()
            task.cancel()
            with pytest.raises(asyncio.CancelledError):
                await task
            await asyncio.wait_for(closed.wait(), 1)
            assert calls == 2

    asyncio.run(run())


def event(seq: int, previous: bytes | None) -> canon.Value:
    return [
        1,
        0,
        seq,
        0,
        [] if previous is None else [previous],
        [1, 0, 1, HASH, HASH],
        HASH,
        canon.CanonMap([("alg", "ed25519"), ("sig", bytes(64))]),
    ]


def test_subscription_partial_frame_reports_only_complete_cursor_and_resumes(
    tmp_path: Path,
) -> None:
    async def run() -> None:
        first = event(0, None)
        h0 = parse_event(first).hash
        second = event(1, h0)

        async def reply(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
            request = await incoming(reader)
            assert isinstance(request.message, Subscribe)
            if request.message.from_seq == 0:
                writer.write(frame(request.id, [0, first]) + frame(request.id, [0, second])[:-2])
            else:
                assert request.message.from_seq == 1
                writer.write(frame(request.id, [0, second]) + frame(request.id, [1, 2]))
            await writer.drain()

        async with peer(tmp_path / "s", reply):
            client = KernelClient(tmp_path / "s")
            seen: list[Any] = []
            with pytest.raises(SubscriptionInterrupted) as failure:
                async for item in client.subscribe(0):
                    seen.append(item)
            assert len(seen) == 1 and failure.value.next_seq == 1
            assert failure.value.previous_hash == h0
            resumed = [item async for item in client.subscribe(1, previous_hash=h0)]
            assert isinstance(resumed[0], EventRecord) and resumed[0].seq == 1
            assert resumed[1] == SubscriptionEnd(next_seq=2)

    asyncio.run(run())


def test_subscription_timeout_does_not_cancel_consumer_between_yields(tmp_path: Path) -> None:
    async def run() -> None:
        async def reply(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
            request = await incoming(reader)
            writer.write(frame(request.id, [0, event(0, None)]))
            await writer.drain()
            await reader.read()

        async with (
            peer(tmp_path / "s", reply),
            aclosing(KernelClient(tmp_path / "s", timeout=0.05).subscribe(0)) as stream,
        ):
            item = await anext(stream)
            assert isinstance(item, EventRecord)
            await asyncio.sleep(0.08)
            with pytest.raises(SubscriptionInterrupted) as failure:
                await anext(stream)
            assert failure.value.next_seq == 1

    asyncio.run(run())


@pytest.mark.parametrize("failure", ["sequence", "link", "cursor", "count"])
def test_subscription_rejects_skips_bad_links_terminal_and_excess(
    tmp_path: Path, failure: str
) -> None:
    async def run() -> None:
        first = event(0, None)
        h0 = parse_event(first).hash

        async def reply(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
            request = await incoming(reader)
            frames = frame(request.id, [0, first])
            if failure == "cursor":
                frames += frame(request.id, [1, 99])
            else:
                frames += frame(
                    request.id,
                    [
                        0,
                        event(2 if failure == "sequence" else 1, HASH if failure == "link" else h0),
                    ],
                )
            writer.write(frames)
            await writer.drain()

        async with peer(tmp_path / "s", reply):
            with pytest.raises(ProtocolError):
                async for _ in KernelClient(tmp_path / "s").subscribe(
                    0, max_events=1 if failure == "count" else 2
                ):
                    pass

    asyncio.run(run())


def test_rpc_absolute_timeout_closes_once_without_retry(tmp_path: Path) -> None:
    async def run() -> None:
        calls = 0
        closed = asyncio.Event()

        async def reply(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
            nonlocal calls
            await incoming(reader)
            calls += 1
            writer.write(b"\x00")
            await writer.drain()
            assert await reader.read() == b""
            closed.set()

        async with peer(tmp_path / "s", reply):
            with pytest.raises(TimeoutError):
                await KernelClient(tmp_path / "s", timeout=0.05).head()
            await asyncio.wait_for(closed.wait(), 1)
            assert calls == 1

    asyncio.run(run())


def test_staging_partial_writes_wrong_uid_and_ambiguous_directory_sync(tmp_path: Path) -> None:
    tmp_path.chmod(0o700)
    uploader = StagingUploader(tmp_path)
    original_write = os.write

    def short(fd: int, data: bytes | memoryview[int]) -> int:
        return original_write(fd, data[:1])

    with patch("logos_client.staging.os.write", side_effect=short):
        ref = uploader.put(b"abc", "certificate")
    assert (tmp_path / ref.name).read_bytes() == b"abc"
    uid = os.geteuid()
    with (
        patch("logos_client.staging.os.geteuid", return_value=uid + 1),
        pytest.raises(PermissionError),
    ):
        uploader.put(b"x", "model")
    before = set(tmp_path.iterdir())
    original_sync = os.fsync
    calls = 0

    def fail_directory(fd: int) -> None:
        nonlocal calls
        calls += 1
        if calls == 2:
            raise OSError("directory sync failed")
        original_sync(fd)

    with (
        patch("logos_client.staging.os.fsync", side_effect=fail_directory),
        pytest.raises(OSError) as error,
    ):
        uploader.put(b"ambiguous", "model")
    extra = set(tmp_path.iterdir()) - before
    assert len(extra) == 1
    assert next(iter(extra)).read_bytes() == b"ambiguous"
    assert "no automatic retry" in str(error.value.__notes__)


def test_already_refuted_detail_vector_is_parsed_and_malformed_detail_rejected() -> None:
    """5.6/S4: erro com detalhe [5, h_O, h(Refuted), h(M)] vira AlreadyRefuted."""
    from logos_client.client import AlreadyRefuted, _response
    from logos_client.protocol import ProtocolError

    root = Path(__file__).resolve().parents[2]
    vectors = json.loads(
        (root / "spec/vectors/kernel-protocol/already-refuted-v1.json").read_text()
    )

    async def parse(payload: bytes, request_id: int) -> canon.Value:
        reader = asyncio.StreamReader()
        reader.feed_data(len(payload).to_bytes(4, "big") + payload)
        reader.feed_eof()
        return await _response(reader, request_id)

    for v in vectors["vectors"]:
        with pytest.raises(AlreadyRefuted) as caught:
            asyncio.run(parse(bytes.fromhex(v["response_hex"]), v["request_id"]))
        assert caught.value.code == 5
        assert caught.value.obligation.hex() == v["obligation_hex"]
        assert caught.value.refuted.hex() == v["refuted_hex"]
        assert caught.value.model.hex() == v["model_hex"]
    bad_code = canon.encode([1, 7, 0, [4, b"\0" * 32, b"\0" * 32, b"\0" * 32]])
    with pytest.raises(ProtocolError):
        asyncio.run(parse(bad_code, 7))
    short = canon.encode([1, 7, 0, [5, b"\0" * 31, b"\0" * 32, b"\0" * 32]])
    with pytest.raises(ProtocolError):
        asyncio.run(parse(short, 7))


def test_assume_and_open_result_vectors_parse_strictly() -> None:
    """5.7: assume-result/v1 e open-result/v1 a partir de vetores independentes."""
    from logos_client.protocol import (
        Assume,
        AssumeResult,
        OpenResult,
        ProtocolError,
        QueryOpen,
        parse_result,
    )

    root = Path(__file__).resolve().parents[2]
    data = json.loads(
        (root / "spec/vectors/kernel-protocol/assume-open-result-v1.json").read_text()
    )
    assume = Assume(
        vocabulary=b"\0" * 32,
        formula=bytes.fromhex("8401036c464f4c2d6d732d65712f7631818100"),
        scope="session:x",
    )
    query = QueryOpen(reference=b"\0" * 32)
    for v in data["vectors"]:
        value = canon.decode(bytes.fromhex(v["value_hex"]))
        if v["kind"] == "assume":
            result = parse_result(assume, value)
            assert isinstance(result, AssumeResult)
            assert result.event.hex() == v["events"][0]
            continue
        result = parse_result(query, value)
        if v["events"] is None:
            assert result is None
            continue
        assert isinstance(result, OpenResult)
        assert [a.event.hex() for a in result.assumptions] == v["events"]
        if "scopes" in v:
            assert [a.scope for a in result.assumptions] == v["scopes"]
    unordered = canon.encode([1, [[b"\x02" * 32, "s"], [b"\x01" * 32, "s"]]])
    with pytest.raises(ProtocolError):
        parse_result(query, canon.decode(unordered))
