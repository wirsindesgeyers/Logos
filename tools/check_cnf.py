"""Compara 5k traduções reais com Z3 fora da TCB e tabela verdade exaustiva."""

import json
import subprocess
from pathlib import Path

import z3  # type: ignore[import-untyped]

ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    worker = subprocess.Popen(
        ["cargo", "run", "--quiet", "-p", "logos-ground", "--example", "cnf_corpus"],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        text=True,
    )
    assert worker.stdout is not None
    seen: set[str] = set()
    counts = {True: 0, False: 0}
    count = 0
    try:
        for line in worker.stdout:
            case = json.loads(line)
            assert case["case"] == count
            assert case["obligation"] not in seen
            seen.add(case["obligation"])
            lines = case["dimacs"].splitlines()
            header = lines[0].split()
            assert header[:2] == ["p", "cnf"] and len(header) == 4
            variables, clauses = map(int, header[2:])
            assert len(lines) == clauses + 1
            solver = z3.Solver()
            solver.set(timeout=5000)
            atoms = [z3.Bool(f"v{i}") for i in range(variables)]
            previous: list[int] | None = None
            for row in lines[1:]:
                values = list(map(int, row.split()))
                assert values[-1] == 0
                literals = values[:-1]
                assert literals == sorted(literals)
                assert previous is None or previous <= literals
                previous = literals
                assert all(0 < abs(x) <= variables for x in literals)
                solver.add(
                    z3.Or([atoms[x - 1] if x > 0 else z3.Not(atoms[-x - 1]) for x in literals])
                )
            status = solver.check()
            assert status in (z3.sat, z3.unsat), (count, status)
            expected = case["valid"]
            assert isinstance(expected, bool)
            assert (status == z3.unsat) == expected, count
            counts[expected] += 1
            count += 1
        assert worker.wait() == 0
    finally:
        if worker.poll() is None:
            worker.terminate()
            worker.wait()
    assert count == 5000 and min(counts.values()) > 100
    print(
        f"CNF_DIFFERENTIAL_OK cases={count} distinct={len(seen)} atom_valuations=4 "
        f"valid={counts[True]} invalid={counts[False]} divergences=0"
    )


if __name__ == "__main__":
    main()
