#!/usr/bin/env python3
"""Reproduz observações de perfil4.5; diagnóstico fora da TCB e do corpus200/600."""

import hashlib
import json
import lzma
import platform
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
subprocess.run(["cargo", "test", "-p", "logos-lrat", "--test", "vectors"], cwd=ROOT, check=True)
vectors = json.loads((ROOT / "spec/vectors/lrat/vectors.json").read_text())["vectors"]
with tempfile.TemporaryDirectory(prefix="logos-cake-profile-") as temporary:
    directory = Path(temporary)
    executable = directory / "checker"
    data = lzma.decompress((ROOT / "vendor/cake-lpr" / (platform.machine() + ".xz")).read_bytes())
    lock = json.loads((ROOT / "vendor/cake-lpr/lock.json").read_text())
    assert hashlib.sha256(data).hexdigest() == lock["binaries"][platform.machine()]["sha256"]
    executable.write_bytes(data)
    executable.chmod(0o700)
    for vector in vectors:
        (directory / "cnf").write_text(vector["cnf"])
        (directory / "proof").write_text(vector["proof"])
        command = [
            "/usr/bin/prlimit",
            "--as=268435456:268435456",
            "--cpu=3:4",
            "--",
            "/usr/bin/bwrap",
            "--unshare-user",
            "--unshare-pid",
            "--unshare-net",
            "--clearenv",
            "--cap-drop",
            "ALL",
        ]
        for source, target in [
            (executable, "/checker"),
            (directory / "cnf", "/cnf"),
            (directory / "proof", "/proof"),
        ]:
            command.extend(["--ro-bind", str(source), target])
        command.extend(
            ["--", "/checker", "--CML_HEAP_SIZE=64", "--CML_STACK_SIZE=64", "/cnf", "/proof"]
        )
        result = subprocess.run(command, capture_output=True, timeout=5, check=False)
        accepted = (
            result.returncode == 0
            and result.stdout == b"s VERIFIED UNSAT\n"
            and result.stderr == b""
        )
        print(
            vector["name"],
            "Rust=" + str(vector["accepted"]),
            "cake=" + str(accepted),
            result.stderr.decode().strip(),
        )
