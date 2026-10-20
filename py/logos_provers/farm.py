"""Portfólio não confiável; cada candidato exige um checker do kernel."""

from __future__ import annotations

import asyncio
import hashlib
import importlib.util
import json
import os
import signal
import sys
from collections import OrderedDict
from contextlib import suppress
from dataclasses import dataclass
from pathlib import Path

from logos_client.client import KernelClient, KernelError
from logos_client.protocol import Certify, CheckResult, Refute, Witness, WitnessResult
from logos_client.staging import StagingUploader
from logos_provers import model_context

MAX_INPUT = 1_048_576
MAX_OUTPUT = 33_554_432
MODES = ("refute", "finder", "sat", "witness")


@dataclass(frozen=True)
class Candidate:
    kind: str
    payload: bytes


@dataclass(frozen=True)
class FarmResult:
    result: CheckResult | None
    witness: WitnessResult | None
    diagnostics: tuple[str, ...]
    elapsed: float
    verdict_elapsed: float | None
    cache_hit: bool


class Sandbox:
    """Sem bind da raiz/home; ambientes/FDs privados não são herdados."""

    def __init__(self, encoder: Path, solver: Path) -> None:
        self.encoder, self.solver = encoder.resolve(strict=True), solver.resolve(strict=True)
        spec = importlib.util.find_spec("z3")
        if spec is None or spec.origin is None:
            raise RuntimeError("z3 runtime absent")
        self.z3 = Path(spec.origin).parent.resolve()

    def command(self) -> list[str]:
        base = Path(sys.base_prefix).resolve()
        executable = Path(sys.executable).resolve()
        packages = Path(__file__).resolve().parent.parent
        args = [
            "/usr/bin/bwrap",
            "--die-with-parent",
            "--new-session",
            "--unshare-all",
            "--cap-drop",
            "ALL",
            "--clearenv",
            "--tmpfs",
            "/",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
            "--tmpfs",
            "/tmp",
            "--dir",
            "/tools",
            "--dir",
            "/app",
            "--dir",
            "/deps",
        ]
        for directory in ("/usr", "/lib", "/lib64"):
            if Path(directory).exists():
                args += ["--ro-bind", directory, directory]
        if not base.is_relative_to("/usr"):
            args += ["--ro-bind", str(base), str(base)]
        for source, target in (
            (packages / "logos_client", "/app/logos_client"),
            (packages / "logos_provers", "/app/logos_provers"),
            (self.z3, "/deps/z3"),
            (self.encoder, "/tools/encoder"),
            (self.solver, "/tools/solver"),
        ):
            args += ["--ro-bind", str(source), target]
        args += [
            "--setenv",
            "PYTHONPATH",
            "/app:/deps",
            "--setenv",
            "PATH",
            "/usr/bin",
            "--setenv",
            "LC_ALL",
            "C",
            "--setenv",
            "PYTHONDONTWRITEBYTECODE",
            "1",
            "--chdir",
            "/tmp",
            "--",
            "/usr/bin/prlimit",
            "--as=1073741824",
            "--fsize=134217728",
            "--core=0:0",
            "--nproc=64",
            "--",
            str(executable),
            "-S",
            "-m",
            "logos_provers.farm_worker",
        ]
        return args

    async def produce(
        self, mode: str, obligation: bytes, vocabulary: bytes, timeout_ms: int
    ) -> Candidate | None:
        request = json.dumps(
            {
                "mode": mode,
                "obligation": obligation.hex(),
                "vocabulary": vocabulary.hex(),
                "timeout_ms": timeout_ms,
            }
        ).encode()
        process = await asyncio.create_subprocess_exec(
            *self.command(),
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env={"PATH": os.defpath, "LC_ALL": "C"},
            start_new_session=True,
        )
        assert (
            process.stdin is not None and process.stdout is not None and process.stderr is not None
        )

        async def read(stream: asyncio.StreamReader, limit: int) -> bytes:
            output = bytearray()
            while chunk := await stream.read(65536):
                output.extend(chunk)
                if len(output) > limit:
                    raise ValueError("worker output limit")
            return bytes(output)

        async def write() -> None:
            assert process.stdin is not None
            process.stdin.write(request)
            await process.stdin.drain()
            process.stdin.close()

        readers = [
            asyncio.create_task(read(process.stdout, MAX_OUTPUT)),
            asyncio.create_task(read(process.stderr, 65536)),
        ]
        writer = asyncio.create_task(write())
        try:
            async with asyncio.timeout(timeout_ms / 1000):
                await writer
                out, err = await asyncio.gather(*readers)
                code = await process.wait()
            if code != 0 or err:
                raise RuntimeError(f"isolated worker failed: exit={code}, stderr={err[-1000:]!r}")
            value = json.loads(out)
            if not isinstance(value, dict) or set(value) != {"kind", "candidate"}:
                raise ValueError("worker envelope")
            if value["kind"] not in ("proof", "model"):
                raise ValueError("worker candidate kind")
            payload = value["candidate"]
            if payload is None:
                return None
            if not isinstance(payload, str):
                raise ValueError("worker candidate bytes")
            candidate = bytes.fromhex(payload)
            if not candidate or len(candidate) > MAX_OUTPUT // 2:
                raise ValueError("worker candidate size")
            return Candidate(value["kind"], candidate)
        finally:
            for task in [writer, *readers]:
                task.cancel()
            if process.returncode is None:
                with suppress(ProcessLookupError):
                    os.killpg(process.pid, signal.SIGKILL)
            await process.wait()
            await asyncio.gather(writer, *readers, return_exceptions=True)


class ProverFarm:
    def __init__(
        self,
        client: KernelClient,
        uploader: StagingUploader,
        sandbox: Sandbox,
        *,
        timeout: float = 30,
        cache_size: int = 128,
    ) -> None:
        if type(timeout) not in (float, int) or not 0 < timeout <= 3600:
            raise ValueError("timeout must be in (0,3600]")
        if type(cache_size) is not int or not 0 <= cache_size <= 1024:
            raise ValueError("cache_size must be in 0..1024")
        self.client, self.uploader, self.sandbox = client, uploader, sandbox
        self.timeout, self.cache_size = timeout, cache_size
        self._cache: OrderedDict[bytes, Candidate] = OrderedDict()

    async def run(self, obligation: bytes, obligation_hash: bytes, vocabulary: bytes) -> FarmResult:
        start = asyncio.get_running_loop().time()
        if type(obligation) is not bytes or type(vocabulary) is not bytes:
            raise ValueError("public O and V must be bytes")
        if len(obligation) > MAX_INPUT or len(vocabulary) > MAX_INPUT:
            raise ValueError("farm input limit")
        if (
            type(obligation_hash) is not bytes
            or hashlib.sha256(b"logos/obligation/v1\0" + obligation).digest() != obligation_hash
        ):
            raise ValueError("obligation hash mismatch")
        vocab = model_context._validate_vocabulary(vocabulary)
        model_context._validate_obligation(obligation, vocab)
        tasks: dict[asyncio.Task[Candidate | None], str] = {
            asyncio.create_task(
                self.sandbox.produce(mode, obligation, vocabulary, max(1, int(self.timeout * 1000)))
            ): mode
            for mode in MODES
        }
        cached = self._cache.get(obligation_hash)
        if cached is not None:
            self._cache.move_to_end(obligation_hash)

            async def hit() -> Candidate:
                return cached

            tasks[asyncio.create_task(hit())] = "cache"
        result: CheckResult | None = None
        witness: WitnessResult | None = None
        verdict_elapsed: float | None = None
        diagnostics: list[str] = []
        try:
            async with asyncio.timeout_at(start + self.timeout):
                while tasks:
                    done, _ = await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
                    for task in sorted(done, key=lambda item: tasks[item] != "witness"):
                        mode = tasks.pop(task)
                        if task.cancelled():
                            continue
                        try:
                            candidate = task.result()
                        except (ValueError, RuntimeError, OSError, TimeoutError) as error:
                            diagnostics.append(f"{mode}: {error}")
                            continue
                        if candidate is None:
                            diagnostics.append(f"{mode}: no candidate (inconclusive)")
                            continue
                        if mode == "witness":
                            if candidate.kind != "model":
                                diagnostics.append("witness: invalid candidate kind")
                                continue
                            try:
                                witness = await self.client.witness(
                                    Witness(
                                        obligation=obligation_hash,
                                        model=self.uploader.put(candidate.payload, "model"),
                                    )
                                )
                            except KernelError as error:
                                diagnostics.append(f"witness: {error}")
                        elif result is None:
                            if candidate.kind == "proof":
                                checked = await self.client.certify(
                                    Certify(
                                        obligation=obligation_hash,
                                        format="lrat-rup/v1",
                                        certificate=self.uploader.put(
                                            candidate.payload, "certificate"
                                        ),
                                    )
                                )
                            elif candidate.kind == "model":
                                checked = await self.client.refute(
                                    Refute(
                                        obligation=obligation_hash,
                                        model=self.uploader.put(candidate.payload, "model"),
                                    )
                                )
                            else:
                                diagnostics.append(f"{mode}: invalid candidate kind")
                                continue
                            if checked.state in ("verified", "refuted"):
                                result = checked
                                verdict_elapsed = asyncio.get_running_loop().time() - start
                                if self.cache_size:
                                    self._cache[obligation_hash] = candidate
                                    self._cache.move_to_end(obligation_hash)
                                    while (
                                        len(self._cache) > self.cache_size
                                        or sum(len(c.payload) for c in self._cache.values())
                                        > MAX_OUTPUT
                                    ):
                                        self._cache.popitem(last=False)
                                # Keep the independent witness request alive.
                                for pending, route in list(tasks.items()):
                                    if route != "witness":
                                        pending.cancel()
                            else:
                                diagnostics.append(f"{mode}: checker {checked.state}")
                                if mode == "cache":
                                    self._cache.pop(obligation_hash, None)
                    for task in list(tasks):
                        if task.cancelled():
                            tasks.pop(task)
        except TimeoutError:
            diagnostics.append("global deadline (inconclusive unfinished routes)")
        finally:
            for task in tasks:
                task.cancel()
            await asyncio.gather(*tasks, return_exceptions=True)
        return FarmResult(
            result,
            witness,
            tuple(diagnostics),
            asyncio.get_running_loop().time() - start,
            verdict_elapsed,
            cached is not None,
        )
