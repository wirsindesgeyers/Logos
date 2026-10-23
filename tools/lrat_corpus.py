#!/usr/bin/env python3
"""Corpus SAT-2002: verifica fixtures offline ou reproduz provas com solver externo."""

import argparse
import hashlib
import json
import lzma
import os
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path
from typing import TypedDict, cast

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "spec/vectors/lrat/sat2002.json.xz"


class Case(TypedDict):
    source: str
    source_sha256: str
    cnf: str
    proof: str
    h_cnf: str
    h_proof: str
    used: list[int]
    solver_options: list[str]
    raw_proof_sha256: str


def digest(tag: bytes, wire: str) -> str:
    return hashlib.sha256(tag + wire.encode("ascii")).hexdigest()


def canonicalize(data: bytes) -> str:
    variables = -1
    count = -1
    clauses: list[list[int]] = []
    current: list[int] = []
    for line in data.decode("ascii").splitlines():
        fields = line.split()
        if not fields or fields[0] in ("c", "%"):
            continue
        if fields[0] == "p":
            assert variables == -1 and fields[:2] == ["p", "cnf"] and len(fields) == 4
            variables, count = map(int, fields[2:])
            continue
        for field in fields:
            value = int(field)
            if value:
                assert 0 < abs(value) <= variables
                current.append(value)
            else:
                clauses.append(sorted(current))
                current = []
    assert variables >= 0 and not current and len(clauses) == count
    clauses.sort()
    return f"p cnf {variables} {count}\n" + "".join(
        "".join(f"{lit} " for lit in clause) + "0\n" for clause in clauses
    )


def reference(cnf: str, proof: str) -> list[int]:
    """Oráculo de conjuntos/dicionários, independente dos vetores/índices Rust."""
    rows = cnf.splitlines()
    variables, originals = map(int, rows[0].split()[2:])
    assert len(rows) == originals + 1
    database = {i: set(map(int, row.split()[:-1])) for i, row in enumerate(rows[1:], 1)}
    live = set(database)
    dependencies: dict[int, list[int]] = {}
    last = originals
    empty: int | None = None
    for line in proof.splitlines():
        fields = line.split()
        if not fields or fields[0] == "c":
            continue
        identifier = int(fields[0])
        if fields[1] == "d":
            assert identifier == last and fields[-1] == "0"
            for field in fields[2:-1]:
                item = int(field)
                assert item in live
                live.remove(item)
            continue
        assert identifier > last
        values = list(map(int, fields[1:]))
        split = values.index(0)
        clause = set(values[:split])
        hints = values[split + 1 : -1]
        assert values[-1] == 0 and hints and all(h in live for h in hints)
        assert all(0 < abs(lit) <= variables and -lit not in clause for lit in clause)
        false = set(clause)
        conflict = False
        for position, hint in enumerate(hints):
            assert not any(-lit in false for lit in database[hint])
            remaining = database[hint] - false
            if not remaining:
                assert position == len(hints) - 1
                conflict = True
            else:
                assert len(remaining) == 1
                false.add(-next(iter(remaining)))
        assert conflict
        database[identifier] = clause
        live.add(identifier)
        dependencies[identifier] = hints
        last = identifier
        if not clause and empty is None:
            empty = identifier
    assert empty is not None
    seen: set[int] = set()
    stack = [empty]
    while stack:
        item = stack.pop()
        if item not in seen:
            seen.add(item)
            stack.extend(dependencies.get(item, []))
    return sorted(i for i in seen if i <= originals)


def trim(proof: str) -> str:
    """Remove adições fora do fecho e deleções, sem alterar cláusulas/hints/IDs."""
    additions: dict[int, tuple[str, list[int]]] = {}
    empty: int | None = None
    for line in proof.splitlines():
        fields = line.split()
        if not fields or fields[0] == "c" or fields[1] == "d":
            continue
        identifier = int(fields[0])
        values = list(map(int, fields[1:]))
        split = values.index(0)
        additions[identifier] = (line, values[split + 1 : -1])
        if split == 0 and empty is None:
            empty = identifier
    assert empty is not None
    seen: set[int] = set()
    stack = [empty]
    while stack:
        item = stack.pop()
        if item not in seen:
            seen.add(item)
            if item in additions:
                stack.extend(additions[item][1])
    return "".join(line + "\n" for i, (line, _) in additions.items() if i in seen)


def check(cases: list[Case]) -> None:
    assert len(cases) >= 200
    assert len({c["source"] for c in cases}) == len(cases)
    assert len({c["h_cnf"] for c in cases}) == len(cases)
    for index, case in enumerate(cases, 1):
        assert case["cnf"] == canonicalize(case["cnf"].encode())
        assert digest(b"logos/cnf/v1\0", case["cnf"]) == case["h_cnf"]
        assert digest(b"logos/certificate/v1\0", case["proof"]) == case["h_proof"]
        # ADR0057: hashes em todos os casos; a releitura RUP (cara) em 1 de cada 10 no
        # nível rápido, em todos no completo.
        if os.environ.get("LOGOS_TIER") != "fast" or index % 10 == 1:
            assert reference(case["cnf"], case["proof"]) == case["used"]
        if index % 25 == 0:
            print(f"LRAT_CORPUS_CHECK processed={index}/{len(cases)}", flush=True)
    print(f"LRAT_CORPUS_OK sat2002={len(cases)}")


def rebuild(archive: Path, solver: Path, cases: list[Case]) -> None:
    expected = {c["source"]: c for c in cases}
    with (
        tarfile.open(archive, "r|gz") as source,
        tempfile.TemporaryDirectory() as directory,
    ):
        for member in source:
            if member.name not in expected:
                continue
            stream = source.extractfile(member)
            assert stream is not None
            data = stream.read()
            case = expected.pop(member.name)
            assert hashlib.sha256(data).hexdigest() == case["source_sha256"]
            cnf = canonicalize(data)
            assert cnf == case["cnf"]
            input_path, output_path = (
                Path(directory) / "input.cnf",
                Path(directory) / "proof.lrat",
            )
            input_path.write_text(cnf)
            result = subprocess.run(
                [
                    str(solver.resolve()),
                    "--lrat",
                    "--no-binary",
                    str(input_path),
                    str(output_path),
                ],
                stdout=subprocess.DEVNULL,
                check=False,
                timeout=60,
            )
            assert result.returncode == 20
            raw_proof = output_path.read_text()
            assert hashlib.sha256(raw_proof.encode()).hexdigest() == case["raw_proof_sha256"]
            assert trim(raw_proof) == case["proof"], member.name
    assert not expected
    check(cases)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--emit", action="store_true")
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--solver", type=Path)
    args = parser.parse_args()
    compressed = FIXTURE.read_bytes()
    manifest = json.loads((FIXTURE.parent / "corpus-manifest.json").read_text())
    assert hashlib.sha256(compressed).hexdigest() == manifest["fixture_sha256"]
    assert len(compressed) == manifest["fixture_bytes"]
    plain = lzma.decompress(compressed)
    assert len(plain) == manifest["uncompressed_bytes"]
    cases = cast(list[Case], json.loads(plain)["cases"])
    assert len(cases) == manifest["count"]
    assert [c["source"] for c in cases] == [c["source"] for c in manifest["cases"]]
    for case, metadata in zip(cases, manifest["cases"], strict=True):
        for key in (
            "h_cnf",
            "h_proof",
            "source_sha256",
            "raw_proof_sha256",
            "solver_options",
        ):
            assert case.get(key) == metadata[key]
    if args.emit:
        sys.stdout.write(json.dumps({"cases": cases}))
        return
    if args.archive is not None or args.solver is not None:
        assert args.archive is not None and args.solver is not None
        rebuild(args.archive, args.solver, cases)
    else:
        check(cases)


if __name__ == "__main__":
    main()
