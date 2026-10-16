#!/usr/bin/env python3
"""Instala fixtures offline ou reconstrói cake_lpr com fonte/toolchain fixadas (ADR0031)."""

from __future__ import annotations

import argparse
import hashlib
import json
import lzma
import platform
import subprocess
import tarfile
import tempfile
import urllib.request
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
VENDOR = ROOT / "vendor/cake-lpr"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_lock() -> dict[str, Any]:
    lock: dict[str, Any] = json.loads((VENDOR / "lock.json").read_text())
    source = VENDOR / "source.tar.xz"
    if digest(source.read_bytes()) != lock["source_sha256"]:
        raise ValueError("source archive hash mismatch")
    with tarfile.open(source) as archive:
        if sorted(archive.getnames()) != sorted(lock["files"]):
            raise ValueError("source file inventory mismatch")
        for item in archive:
            file = archive.extractfile(item)
            if file is None or digest(file.read()) != lock["files"][item.name]:
                raise ValueError("upstream source hash mismatch")
    return lock


def rebuild(lock: dict[str, Any], target: str, output: Path) -> None:
    host = {"x86_64": "x86_64", "aarch64": "aarch64"}[platform.machine()]
    tool = lock["zig"][host + "-linux"]
    cache = ROOT / "target/cake-build-tools"
    cache.mkdir(parents=True, exist_ok=True)
    archive = cache / tool["tarball"].rsplit("/", 1)[1]
    if not archive.exists():
        urllib.request.urlretrieve(tool["tarball"], archive)
    if digest(archive.read_bytes()) != tool["shasum"]:
        raise ValueError("Zig archive hash mismatch")
    directory = cache / archive.name.removesuffix(".tar.xz")
    if not directory.exists():
        with tarfile.open(archive) as tar:
            tar.extractall(cache, filter="data")
    with tempfile.TemporaryDirectory(prefix="logos-cake-build-") as temporary:
        work = Path(temporary)
        with tarfile.open(VENDOR / "source.tar.xz") as tar:
            tar.extractall(work, filter="data")
        assembly = "cake_lpr.S" if target == "x86_64" else "cake_lpr_arm8.S"
        compiler = directory / "zig"
        subprocess.run(
            [
                str(compiler),
                "cc",
                "-target",
                target + "-linux-musl",
                "-mcpu=baseline",
                "-O2",
                "-g0",
                "-s",
                "-static",
                "-fno-ident",
                "-Wl,--build-id=none",
                "-std=c99",
                "basis_ffi.c",
                assembly,
                "-o",
                str(output),
            ],
            cwd=work,
            env={
                "PATH": "/usr/bin:/bin",
                "LC_ALL": "C",
                "SOURCE_DATE_EPOCH": "0",
                "ZIG_GLOBAL_CACHE_DIR": str(work / "cache"),
                "ZIG_LOCAL_CACHE_DIR": str(work / "local"),
            },
            check=True,
        )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rebuild", action="store_true")
    parser.add_argument("--target", choices=["x86_64", "aarch64"], default=platform.machine())
    parser.add_argument("--output", type=Path, default=ROOT / "target/cake-lpr/cake_lpr")
    args = parser.parse_args()
    lock = load_lock()
    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    if args.rebuild:
        rebuild(lock, args.target, output)
    else:
        data = lzma.decompress((VENDOR / (args.target + ".xz")).read_bytes())
        output.write_bytes(data)
    actual = digest(output.read_bytes())
    if actual != lock["binaries"][args.target]["sha256"]:
        output.unlink()
        raise ValueError("cake_lpr binary hash mismatch")
    output.chmod(0o755)
    report = {
        "host": platform.node(),
        "machine": platform.machine(),
        "target": args.target,
        "revision": lock["revision"],
        "zig": lock["zig_version"],
        "sha256": actual,
        "rebuilt": args.rebuild,
    }
    output.with_suffix(".build.json").write_text(json.dumps(report, indent=2) + "\n")
    print("CAKE_BUILD_OK " + json.dumps(report))


if __name__ == "__main__":
    main()
