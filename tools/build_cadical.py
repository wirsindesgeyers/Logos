#!/usr/bin/env python3
"""Build offline do produtor CaDiCaL fixado; não é parte da TCB (ADR0032)."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import tarfile
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
VENDOR = ROOT / "vendor/cadical"


def build(output: Path, *, rebuild: bool = False) -> None:
    lock: dict[str, Any] = json.loads((VENDOR / "lock.json").read_text())
    archive = VENDOR / "source.tar.xz"
    if hashlib.sha256(archive.read_bytes()).hexdigest() != lock["source_sha256"]:
        raise ValueError("CaDiCaL source hash mismatch")
    report_path = output.with_suffix(".build.json")
    if not rebuild and output.exists() and report_path.exists():
        report = json.loads(report_path.read_text())
        if (
            report["source_sha256"] == lock["source_sha256"]
            and hashlib.sha256(output.read_bytes()).hexdigest() == report["binary_sha256"]
        ):
            print("CADICAL_BUILD_OK cached=true revision=" + lock["revision"])
            return
    work = ROOT / "target/cadical-source"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)
    with tarfile.open(archive) as source:
        source.extractall(work, filter="data")
    env = {"PATH": os.defpath, "LC_ALL": "C"}
    log_path = ROOT / "target/cadical-build.log"
    with log_path.open("w") as log:
        subprocess.run(["./configure"], cwd=work, env=env, stdout=log, stderr=log, check=True)
        subprocess.run(
            ["make", "-j2", "cadical"],
            cwd=work / "build",
            env=env,
            stdout=log,
            stderr=log,
            check=True,
        )
    output.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(work / "build/cadical", output)
    output.chmod(0o755)
    report = {
        **lock,
        "binary_sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
        "compiler": subprocess.check_output(["g++", "--version"], env=env, text=True).splitlines()[
            0
        ],
    }
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    print("CADICAL_BUILD_OK cached=false revision=" + lock["revision"])


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/cadical/cadical")
    parser.add_argument("--rebuild", action="store_true")
    args = parser.parse_args()
    build(args.output.resolve(), rebuild=args.rebuild)


if __name__ == "__main__":
    main()
