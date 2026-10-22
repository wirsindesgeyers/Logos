"""Portfólio, cache, isolamento e estados inconclusivos observáveis."""

from __future__ import annotations

import asyncio
import hashlib
import json
import os
import signal
import subprocess
from pathlib import Path
from typing import Any

import pytest

from logos_client.protocol import CasRef, CheckResult, WitnessResult
from logos_provers.farm import Candidate, ProverFarm, Sandbox
from logos_provers.sat_cases import document_case

CASE = document_case("test/false", "vocab v1 {} claim G:false;", sat=True)
HO = hashlib.sha256(b"logos/obligation/v1\0" + CASE.obligation).digest()


class Workers:
    def __init__(self) -> None:
        self.calls: list[str] = []
        self.cancelled: list[str] = []
        self.responses: dict[str, Candidate | None] = {
            "refute": None,
            "finder": Candidate("model", b"bad"),
            "sat": Candidate("model", b"good"),
            "witness": Candidate("model", b"witness"),
        }
        self.delays = {"refute": 0.0, "finder": 0.01, "sat": 0.02, "witness": 0.03}

    async def produce(self, mode: str, o: bytes, v: bytes, timeout_ms: int) -> Candidate | None:
        self.calls.append(mode)
        try:
            await asyncio.sleep(self.delays[mode])
            return self.responses[mode]
        except asyncio.CancelledError:
            self.cancelled.append(mode)
            raise


class Upload:
    def put(self, payload: bytes, domain: str) -> Any:
        return CasRef(hash=payload.ljust(32, b"\0"))


class Checker:
    def __init__(self) -> None:
        self.calls: list[bytes] = []
        self.reject = False
        self.unknown = False

    async def refute(self, message: Any) -> CheckResult:
        self.calls.append(message.model.hash.rstrip(b"\0"))
        return CheckResult(
            state=(
                "unknown"
                if self.unknown
                else (
                    "rejected_pending"
                    if message.model.hash.rstrip(b"\0") == b"bad" or self.reject
                    else "refuted"
                )
            ),
            event=bytes([len(self.calls)]) * 32,
        )

    async def witness(self, message: Any) -> WitnessResult:
        return WitnessResult(outcome="accepted", event=bytes(32))


def make_farm(workers: Workers, checker: Checker, **options: Any) -> ProverFarm:
    # Structural test doubles keep all accepted/rejected outcomes visible.
    return ProverFarm(checker, Upload(), workers, **options)  # type: ignore[arg-type]


def test_first_usable_candidate_and_cache_rechecks() -> None:
    async def run() -> None:
        workers, checker = Workers(), Checker()
        farm = make_farm(workers, checker)
        first = await farm.run(CASE.obligation, HO, CASE.vocabulary)
        assert first.result is not None and first.result.state == "refuted"
        assert checker.calls == [b"bad", b"good"]
        assert first.witness is not None and not first.cache_hit
        workers.responses = dict.fromkeys(workers.responses)
        second = await farm.run(CASE.obligation, HO, CASE.vocabulary)
        assert second.cache_hit and checker.calls[-1] == b"good" and len(checker.calls) == 3
        assert workers.calls.count("witness") == 2
        checker.reject = True
        third = await farm.run(CASE.obligation, HO, CASE.vocabulary)
        assert third.result is None and HO not in farm._cache
        assert any("checker rejected_pending" in message for message in third.diagnostics)

    asyncio.run(run())


def test_unknown_no_candidate_and_deadline_never_become_verdicts() -> None:
    async def run() -> None:
        workers, checker = Workers(), Checker()
        workers.responses = dict.fromkeys(workers.responses)
        farm = make_farm(workers, checker)
        result = await farm.run(CASE.obligation, HO, CASE.vocabulary)
        assert result.result is None and not checker.calls and len(result.diagnostics) == 4
        workers.delays = dict.fromkeys(workers.delays, 10.0)
        farm = make_farm(workers, checker, timeout=0.02)
        result = await farm.run(CASE.obligation, HO, CASE.vocabulary)
        assert result.result is None and len(workers.cancelled) == 4
        assert any("deadline" in item for item in result.diagnostics)

    asyncio.run(run())


def test_input_hash_and_vocabulary_are_checked_before_workers() -> None:
    async def run() -> None:
        workers = Workers()
        farm = make_farm(workers, Checker())
        with pytest.raises(ValueError, match="hash"):
            await farm.run(CASE.obligation, bytes(32), CASE.vocabulary)
        with pytest.raises(ValueError):
            await farm.run(CASE.obligation, HO, b"invalid")
        assert not workers.calls

    asyncio.run(run())


def test_cancellation_reaps_all_routes() -> None:
    async def run() -> None:
        workers = Workers()
        workers.delays = dict.fromkeys(workers.delays, 10.0)
        farm = make_farm(workers, Checker())
        task = asyncio.create_task(farm.run(CASE.obligation, HO, CASE.vocabulary))
        await asyncio.sleep(0.02)
        task.cancel()
        with pytest.raises(asyncio.CancelledError):
            await task
        assert len(workers.cancelled) == 4

    asyncio.run(run())


@pytest.fixture(scope="session")
def sandbox() -> Sandbox:
    root = Path(__file__).resolve().parents[2]
    subprocess.run(
        ["python3", "tools/build_cadical.py"],
        cwd=root,
        check=True,
        capture_output=True,
        timeout=240,
    )
    subprocess.run(
        ["cargo", "build", "-p", "logos-audit", "--example", "prover_bridge"],
        cwd=root,
        check=True,
        capture_output=True,
        timeout=240,
    )
    return Sandbox(root / "target/debug/examples/prover_bridge", root / "target/cadical/cadical")


def test_real_sandbox_has_no_host_files_environment_or_network(
    sandbox: Sandbox,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    secret = tmp_path / "credential-sentinel"
    secret.write_text("must-not-be-visible")
    monkeypatch.setenv("LOGOS_SECRET_SENTINEL", "must-not-be-inherited")
    code = (
        "secret_path = "
        + repr(str(secret))
        + "\n"
        + """
import json, os, pathlib, socket
secret = pathlib.Path(secret_path)
assert not secret.exists()
assert not pathlib.Path('/home/kauan').exists()
assert 'LOGOS_SECRET_SENTINEL' not in os.environ
assert pathlib.Path('/tools/encoder').is_file()
assert pathlib.Path('/tools/solver').is_file()
try:
    pathlib.Path('/app/logos_provers/forbidden-write').write_text('x')
except OSError:
    pass
else:
    raise AssertionError('runtime mount is writable')
s = socket.socket()
s.settimeout(0.05)
try:
    s.connect(('192.0.2.1', 443))
except OSError:
    pass
else:
    raise AssertionError('network is reachable')
print(json.dumps({'isolated': True}))
"""
    )
    command = [*sandbox.command()[:-2], "-c", code]
    response = subprocess.run(command, capture_output=True, check=True, timeout=10)
    assert json.loads(response.stdout) == {"isolated": True}
    assert not response.stderr


def test_real_workers(sandbox: Sandbox) -> None:
    async def run() -> None:
        refute, sat = await asyncio.gather(
            sandbox.produce("refute", CASE.obligation, CASE.vocabulary, 5000),
            sandbox.produce("sat", CASE.obligation, CASE.vocabulary, 5000),
        )
        assert refute is not None and refute.kind == "model"
        assert sat is not None and sat.kind == "model"
        true_case = document_case("test/true", "vocab v1 {} claim G:true;", sat=False)
        proof = await sandbox.produce("sat", true_case.obligation, true_case.vocabulary, 5000)
        assert proof is not None and proof.kind == "proof"

    asyncio.run(run())


def test_real_timeout_and_cancellation_leave_no_worker(
    sandbox: Sandbox,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    # Capture the real Process, not a PID file the timeout can interrupt mid-write.
    monkeypatch.setattr(
        sandbox, "command", lambda: ["/usr/bin/python3", "-c", "import time;time.sleep(10)"]
    )
    original_create = asyncio.create_subprocess_exec

    async def run() -> None:
        processes: list[asyncio.subprocess.Process] = []
        spawned = asyncio.Event()

        async def capture(*args: str, **kwargs: Any) -> asyncio.subprocess.Process:
            process = await original_create(*args, **kwargs)
            processes.append(process)
            spawned.set()
            return process

        monkeypatch.setattr(asyncio, "create_subprocess_exec", capture)

        def assert_reaped(process: asyncio.subprocess.Process) -> None:
            # A naturally finishing helper cannot satisfy the cleanup assertion.
            assert process.returncode == -signal.SIGKILL
            with pytest.raises(ProcessLookupError):
                os.kill(process.pid, 0)

        try:
            with pytest.raises(TimeoutError):
                await sandbox.produce("refute", CASE.obligation, CASE.vocabulary, 100)
            assert len(processes) == 1
            assert_reaped(processes[0])
            spawned.clear()
            task = asyncio.create_task(
                sandbox.produce("refute", CASE.obligation, CASE.vocabulary, 5000)
            )
            try:
                # Cancel only once the subprocess exists and belongs to produce().
                await asyncio.wait_for(spawned.wait(), 5)
                assert len(processes) == 2
                task.cancel()
                with pytest.raises(asyncio.CancelledError):
                    await task
                assert_reaped(processes[1])
            finally:
                task.cancel()
                await asyncio.gather(task, return_exceptions=True)
        finally:
            # Failure cleanup is after the assertions, so it cannot hide a leak.
            for process in processes:
                if process.returncode is None:
                    process.kill()
                await process.wait()

    asyncio.run(run())


def test_checker_unknown_is_not_a_verdict_and_cache_can_be_disabled() -> None:
    async def run() -> None:
        workers, checker = Workers(), Checker()
        checker.unknown = True
        farm = make_farm(workers, checker, cache_size=0)
        result = await farm.run(CASE.obligation, HO, CASE.vocabulary)
        assert result.result is None and not farm._cache
        assert any("checker unknown" in item for item in result.diagnostics)

    asyncio.run(run())


def test_worker_malformed_output_and_stream_limit(
    sandbox: Sandbox,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import logos_provers.farm as farm_module

    async def run() -> None:
        monkeypatch.setattr(
            sandbox, "command", lambda: ["/usr/bin/python3", "-c", "print('bad JSON')"]
        )
        with pytest.raises(ValueError):
            await sandbox.produce("refute", CASE.obligation, CASE.vocabulary, 5000)
        monkeypatch.setattr(farm_module, "MAX_OUTPUT", 1024)
        monkeypatch.setattr(
            sandbox,
            "command",
            lambda: [
                "/usr/bin/python3",
                "-c",
                "import sys;sys.stdout.write('x'*1025)",
            ],
        )
        with pytest.raises(ValueError, match="output limit"):
            await sandbox.produce("refute", CASE.obligation, CASE.vocabulary, 5000)

    asyncio.run(run())
