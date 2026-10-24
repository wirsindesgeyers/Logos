"""Adapter da CLI independente: somente configuração do operador, sem chaves privadas."""

from __future__ import annotations

import asyncio
import hashlib
import os
import re
import signal
import tempfile
from contextlib import suppress
from dataclasses import dataclass
from pathlib import Path

MAX_STREAM = 2_097_152
REPORT = re.compile(
    r"AUDIT_OK events=(\d+) refutations_rechecked=(\d+) consistencies_rechecked=(\d+) "
    r"proofs_unchecked=(\d+) failures=0 head=([0-9a-f]{64}|none) "
    r"cnf_rederived=(\d+) lrat_rust_rechecked=(\d+) lrat_cake_rechecked=(\d+) "
    r"invalidated_by_recheck=(\d+) facts_verified=(\d+) trust_configs=(\d+)\n"
)


class AuditFailure(Exception):
    pass


@dataclass(frozen=True)
class AuditConfig:
    executable: Path
    cas: Path
    public_key: str
    cake: Path
    rust_checkers: tuple[str, ...]

    def __post_init__(self) -> None:
        for value in (self.public_key, *self.rust_checkers):
            if re.fullmatch(r"[0-9a-f]{64}", value) is None:
                raise ValueError("public key/checker hash must be lowercase hex32")
        if not self.rust_checkers:
            raise ValueError("explicit Rust checker allowlist required")


class Auditor:
    def __init__(self, config: AuditConfig, *, timeout: float = 120) -> None:
        self.config, self.timeout = config, timeout

    async def _run(self, args: list[str], limit: int) -> bytes:
        process = await asyncio.create_subprocess_exec(
            str(self.config.executable),
            *args,
            stdin=asyncio.subprocess.DEVNULL,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env={"PATH": os.defpath, "LC_ALL": "C"},
            start_new_session=True,
        )
        assert process.stdout is not None and process.stderr is not None

        async def read(stream: asyncio.StreamReader, cap: int) -> bytes:
            output = bytearray()
            while chunk := await stream.read(65536):
                output.extend(chunk)
                if len(output) > cap:
                    raise AuditFailure("auditor output limit")
            return bytes(output)

        readers = [
            asyncio.create_task(read(process.stdout, limit)),
            asyncio.create_task(read(process.stderr, 32768)),
        ]
        try:
            async with asyncio.timeout(self.timeout):
                out, _ = await asyncio.gather(*readers)
                code = await process.wait()
            if code != 0:
                raise AuditFailure("auditor refused authentication/recheck")
            return out
        finally:
            for reader in readers:
                reader.cancel()
            if process.returncode is None:
                with suppress(ProcessLookupError):
                    os.killpg(process.pid, signal.SIGKILL)
            await process.wait()
            await asyncio.gather(*readers, return_exceptions=True)

    async def verify(self, source: Path) -> dict[str, int | str | None]:
        config = self.config
        args = [
            "verify",
            str(source),
            "--cas",
            str(config.cas),
            "--public-key",
            config.public_key,
            "--cake-lpr",
            str(config.cake),
        ]
        for checker in config.rust_checkers:
            args += ["--rust-checker", checker]
        out = await self._run(args, 4096)
        match = REPORT.fullmatch(out.decode("ascii"))
        if match is None:
            raise AuditFailure("malformed auditor report")
        fields = match.groups()
        if int(fields[3]) != 0:
            raise AuditFailure("unchecked proofs remain")
        names = (
            "events",
            "refutations_rechecked",
            "consistencies_rechecked",
            "proofs_unchecked",
            "head",
            "cnf_rederived",
            "lrat_rust_rechecked",
            "lrat_cake_rechecked",
            "invalidated_by_recheck",
            "facts_verified",
            "trust_configs",
        )
        return {
            name: (None if value == "none" else value) if name == "head" else int(value)
            for name, value in zip(names, fields, strict=True)
        }

    async def export(self, source: Path) -> dict[str, object]:
        # Complete a read/recheck on the actual exported bytes; compare the snapshot head.
        async with asyncio.timeout(self.timeout):
            before = await self.verify(source)
            config = self.config
            wire = await self._run(
                ["export", str(source), "--public-key", config.public_key], MAX_STREAM
            )
            with tempfile.TemporaryDirectory(prefix="logos-mcp-export-") as directory:
                source = Path(directory) / "events.logos"
                source.write_bytes(wire)
                after = await self.verify(source)
            if before != after:
                raise AuditFailure("ledger changed during export; no coherent report returned")
        return {
            "format": "audit-stream/v1",
            "audit_stream_hex": wire.hex(),
            "sha256": hashlib.sha256(wire).hexdigest(),
            "audit": after,
            "requires_external_cas": True,
        }
