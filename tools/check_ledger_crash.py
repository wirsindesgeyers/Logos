#!/usr/bin/env python3
"""SIGKILL real durante 10k appends; oráculo canon/hashlib/Ed25519 independente.

Usa somente chave PÚBLICA de fixture. Cada morte confere todos os registros e
o prefixo já reconhecido pelo escritor. Semente fixa, timing de SO variável.
"""

from __future__ import annotations

import hashlib
import json
import random
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "py"))
from logos_client import canon  # noqa: E402

TOTAL = 10_000
CRASHES = 24
SEED = 320_026


def public_fixture() -> tuple[Ed25519PublicKey, bytes]:
    """Identidade pública produzida pelo gerador independente da 3.1."""
    vectors = json.loads((ROOT / "spec/vectors/events/vectors.json").read_text())
    vector = vectors["vectors"][0]
    return (
        Ed25519PublicKey.from_public_bytes(bytes.fromhex(vector["kernel_public_key_hex"])),
        bytes.fromhex(vector["kernel_key_id_hex"]),
    )


def verify(path: Path, acknowledged: dict[int, str]) -> int:
    """Não chama Event::decode, hash, verify nem replay Rust."""
    public_key, key_id = public_fixture()
    previous: bytes | None = None
    count = 0
    with sqlite3.connect(f"file:{path / 'ledger.sqlite3'}?mode=ro", uri=True) as connection:
        assert connection.execute("PRAGMA integrity_check").fetchall() == [("ok",)]
        for seq, wire in connection.execute("SELECT seq, wire FROM events ORDER BY seq"):
            assert seq == count, (seq, count)
            event = canon.decode(wire)
            assert isinstance(event, list) and len(event) == 8
            assert event[:4] == [1, 0, seq, 0]
            assert event[4] == ([] if previous is None else [previous])
            assert event[6] == key_id
            signature = event[7]
            assert isinstance(signature, canon.CanonMap)
            fields = dict(signature.entries)
            assert fields.keys() == {"alg", "sig"} and fields["alg"] == "ed25519"
            raw_sig = fields["sig"]
            assert isinstance(raw_sig, bytes)
            public_key.verify(raw_sig, b"logos/sig/event/v1\0" + canon.encode(event[:7]))
            if seq == 0:
                assert event[5] == [1, 0, 1, b"\x10" * 32, key_id]
            else:
                payload = seq.to_bytes(8, "big") * 4
                obligation = hashlib.sha256(b"logos/obligation/v1\0" + canon.encode(seq)).digest()
                assert event[5] == [1, 3, obligation, 0, payload]
                address = obligation.hex()
                obj = path / "cas" / "obligation-v1" / address[:2] / address[2:]
                assert obj.read_bytes() == canon.encode(seq), "missing/corrupt CAS before append"
            assert canon.encode(event) == wire
            previous = hashlib.sha256(b"logos/event/v1\0" + wire).digest()
            if seq in acknowledged:
                assert previous.hex() == acknowledged[seq], ("changed acknowledged event", seq)
            count += 1
    assert not acknowledged or max(acknowledged) < count, "lost acknowledged append"
    return count


def acknowledge(line: str, acknowledged: dict[int, str]) -> None:
    """Reconhecimentos só existem depois que append devolveu sucesso."""
    tag, seq, digest = line.split()
    assert tag == "ACK" and len(digest) == 64, line
    position = int(seq)
    assert position not in acknowledged, ("duplicate acknowledgement", position)
    acknowledged[position] = digest


def start(binary: Path, path: Path) -> tuple[subprocess.Popen[str], int]:
    """Timeout global do runner cobre falhas de handshake/lock."""
    process = subprocess.Popen(
        [str(binary), str(path), str(TOTAL)],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    assert process.stdout is not None
    line = process.stdout.readline()
    assert line.startswith("READY "), (line, process.poll())
    return process, int(line.split()[1])


def main() -> None:
    binary = Path(sys.argv[1]).resolve()
    rng = random.Random(SEED)
    acknowledged: dict[int, str] = {}
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="logos-ledger-crash-") as temporary:
        path = Path(temporary)
        previous_count = 0
        for crash in range(CRASHES):
            process, start_count = start(binary, path)
            try:
                assert start_count == previous_count
                # Segundo processo falha mesmo com o primeiro parado fora de transação.
                if crash == 0:
                    second = subprocess.run(
                        [str(binary), str(path), str(TOTAL)],
                        input="",
                        capture_output=True,
                        text=True,
                        timeout=10,
                        check=False,
                    )
                    assert second.returncode == 23 and second.stdout == "WRITER_BUSY\n", second
                assert process.stdin is not None and process.stdout is not None
                process.stdin.write("GO\n")
                process.stdin.flush()
                threshold = min(start_count + rng.randint(100, 250), TOTAL - 2)
                while True:
                    line = process.stdout.readline()
                    assert line, "writer exited before SIGKILL"
                    acknowledge(line, acknowledged)
                    if int(line.split()[1]) >= threshold:
                        break
                time.sleep(rng.randrange(2000) / 1_000_000)
                process.kill()  # SIGKILL, sem cleanup/Drop/checkpoint voluntário.
                process.wait(timeout=10)
                # readline pode ter prefetched ACKs; drenar pelo mesmo TextIO
                # preserva esses reconhecimentos antes de communicate fechar pipes.
                out = process.stdout.read()
                _, err = process.communicate(timeout=10)
                assert process.returncode == -signal.SIGKILL, (process.returncode, err)
                for line in out.splitlines():
                    acknowledge(line, acknowledged)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.communicate(timeout=10)
            previous_count = verify(path, acknowledged)
            print(
                f"crash={crash + 1}/{CRASHES} recovered={previous_count} "
                f"acknowledged={len(acknowledged)}",
                flush=True,
            )
        process, start_count = start(binary, path)
        try:
            assert start_count == previous_count
            out, err = process.communicate(input="GO\n", timeout=60)
            assert process.returncode == 0, err
            for line in out.splitlines():
                acknowledge(line, acknowledged)
        finally:
            if process.poll() is None:
                process.kill()
                process.communicate(timeout=10)
        assert verify(path, acknowledged) == TOTAL
        # Uma abertura posterior também precisa conferir os 10k pelo replay Rust.
        process, final_count = start(binary, path)
        out, err = process.communicate(input="GO\n", timeout=60)
        assert final_count == TOTAL and process.returncode == 0 and not out, err
    print(
        f"LEDGER_CRASH_OK events={TOTAL} sigkill={CRASHES} seed={SEED} "
        f"acknowledged={len(acknowledged)} elapsed={time.monotonic() - started:.2f}s",
        flush=True,
    )


if __name__ == "__main__":
    main()
