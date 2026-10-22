"""Sete mensagens pelo cliente real e executável real; Python nunca carrega a chave."""

from __future__ import annotations

import asyncio
import json
import os
import subprocess
import time
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

import pytest

from logos_client import canon
from logos_client.client import KernelClient, KernelError
from logos_client.protocol import (
    Assume,
    Certify,
    EventRecord,
    Propose,
    Refute,
    SubscriptionEnd,
    Witness,
    array,
)
from logos_client.staging import StagingUploader

ROOT = Path(__file__).resolve().parents[2]
TRUE = canon.encode([1, 3, "FOL-ms-eq/v1", [[0]]])
FALSE = canon.encode([1, 3, "FOL-ms-eq/v1", [[1]]])


@pytest.fixture(scope="session")
def executables() -> tuple[Path, Path]:
    subprocess.run(
        [
            "cargo",
            "build",
            "-p",
            "logos-kernel",
            "--bin",
            "logos-kernel",
            "--example",
            "client_fixture",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
        timeout=240,
    )
    return ROOT / "target/debug/logos-kernel", ROOT / "target/debug/examples/client_fixture"


@dataclass(frozen=True)
class RunningKernel:
    root: Path
    public: dict[str, str]


@pytest.fixture
def kernel(tmp_path: Path, executables: tuple[Path, Path]) -> Iterator[RunningKernel]:
    tmp_path.chmod(0o700)
    binary, fixture = executables
    result = subprocess.run(
        [str(fixture), str(tmp_path)], capture_output=True, text=True, check=True, timeout=30
    )
    public: dict[str, str] = json.loads(result.stdout)
    for name in ["staging", "private"]:
        (tmp_path / name).mkdir(mode=0o700)
    log = (tmp_path / "kernel.log").open("wb")
    # Provisioning happens only in Rust. The path is opaque to Python; no key bytes are read.
    process = subprocess.Popen(
        [
            str(binary),
            str(tmp_path / "s"),
            str(tmp_path / "ledger"),
            str(tmp_path / "operator.key"),
            str(tmp_path / "staging"),
            str(tmp_path / "private"),
            str(os.geteuid()),
        ],
        stdout=log,
        stderr=subprocess.STDOUT,
    )
    try:
        until = time.monotonic() + 120
        while not (tmp_path / "s").exists():
            if process.poll() is not None or time.monotonic() > until:
                pytest.fail("kernel failed to start: " + (tmp_path / "kernel.log").read_text())
            time.sleep(0.02)
        yield RunningKernel(tmp_path, public)
    finally:
        if process.poll() is None:
            process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)
        log.close()


def test_all_seven_messages_against_real_kernel(kernel: RunningKernel) -> None:
    async def run() -> None:
        client = KernelClient(kernel.root / "s", timeout=30)
        uploader = StagingUploader(kernel.root / "staging")
        public = kernel.public
        vocab = bytes.fromhex(public["vocabulary"])
        model = bytes.fromhex(public["model"])
        head = await client.head()
        assert head is not None and head.seq == 2
        assert await client.status(bytes([9]) * 32) is None
        result = await client.propose(Propose(text_pt="cafe\u0301", goal=TRUE, vocabulary=vocab))
        assert await client.status(result.obligation) is None  # Status is h(Event), not h(O).
        head = await client.head()
        assert head is not None
        events = [item async for item in client.subscribe(0, max_events=head.seq + 1)]
        assert isinstance(events[-1], SubscriptionEnd)
        pending = events[-2]
        assert isinstance(pending, EventRecord)
        status = await client.status(pending.hash)
        assert status is not None and status.kind == 4
        witness = await client.witness(
            Witness(obligation=result.obligation, model=uploader.put(model, "model"))
        )
        assert witness.outcome == "accepted"
        rejected = await client.certify(
            Certify(
                obligation=result.obligation,
                format="lrat-rup/v1",
                certificate=uploader.put(b"invalid", "certificate"),
            )
        )
        assert rejected.state == "rejected_pending"
        verified = await client.certify(
            Certify(
                obligation=result.obligation,
                format="lrat-rup/v1",
                certificate=uploader.put(bytes.fromhex(public["proof"]), "certificate"),
            )
        )
        assert verified.state == "verified"
        status = await client.status(verified.event)
        assert status is not None and status.kind == 2
        late = await client.witness(
            Witness(obligation=result.obligation, model=uploader.put(model, "model"))
        )
        assert late.outcome == "accepted"
        counter = await client.propose(Propose(text_pt="falso", goal=FALSE, vocabulary=vocab))
        refuted = await client.refute(
            Refute(obligation=counter.obligation, model=uploader.put(model, "model"))
        )
        assert refuted.state == "refuted"
        status = await client.status(refuted.event)
        assert status is not None and status.kind == 5
        before = await client.head()
        # 5.7: escopo fora das formas normativas é recusado sem evento.
        with pytest.raises(KernelError) as error:
            await client.assume(Assume(vocabulary=vocab, formula=TRUE, scope=""))
        assert error.value.code == 0
        assert await client.head() == before
        assumed = await client.assume(
            Assume(vocabulary=vocab, formula=TRUE, scope="session:integracao")
        )
        assumption_status = await client.status(assumed.event)
        assert assumption_status is not None and assumption_status.kind == 3
        opened = await client.open_assumptions(assumed.event)
        assert opened is not None
        assert [(a.event, a.scope) for a in opened.assumptions] == [
            (assumed.event, "session:integracao")
        ]
        head = await client.head()
        concurrent = await asyncio.gather(*(client.head() for _ in range(4)))
        assert concurrent == [head] * 4
        assert head is not None
        all_events = [item async for item in client.subscribe(0, max_events=head.seq + 1)]
        assert all_events[-1] == SubscriptionEnd(next_seq=head.seq + 1)
        bodies = [
            array(array(canon.decode(item.canonical_bytes), 8)[5])
            for item in all_events
            if isinstance(item, EventRecord)
        ]
        assert any(body[0] == 3 and body[1] == 14 for body in bodies)
        assert any(body[1] == 4 for body in bodies) and any(body[1] == 5 for body in bodies)
        assert list((kernel.root / "staging").iterdir()) == []

    asyncio.run(run())
