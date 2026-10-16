#!/usr/bin/env python3
"""Vetores manuais independentes de lrat-rup/v1; não chama Rust."""

import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PATH = ROOT / "spec/vectors/lrat/vectors.json"


def vectors() -> dict[str, object]:
    base = "p cnf 2 4\n-2 0\n-1 2 0\n1 0\n2 0\n"
    cases: list[tuple[str, str, str, list[int] | None]] = [
        ("unit-conflict", "p cnf 1 2\n-1 0\n1 0\n", "3 0 1 2 0\n", [1, 2]),
        ("history-deleted", base, "5 2 0 3 2 0\n5 d 2 3 0\n6 0 5 1 0\n", [1, 2, 3]),
        ("unused-branch", base, "5 2 0 3 2 0\n6 0 4 1 0\n", [1, 4]),
        ("duplicates", "p cnf 1 2\n-1 -1 0\n1 1 0\n", "3 0 1 2 0\n", [1, 2]),
        ("empty-original", "p cnf 0 1\n0\n", "2 0 1 0\n", [1]),
        ("sparse", base, "18446744073709551615 0 4 1 0\n", [1, 4]),
        ("wrong-hint", base, "5 0 2 1 0\n", None),
        ("changed-clause", base, "5 1 0 3 2 0\n6 0 5 1 0\n", None),
        ("unjustified", base, "5 0 0\n", None),
        ("deleted", base, "4 d 1 0\n5 0 4 1 0\n", None),
        ("tail", base, "5 0 4 1 0\n6 0 0\n", None),
        ("rat-unsupported", base, "5 1 0 -2 3 0\n", None),
        ("sat", "p cnf 1 1\n1 0\n", "2 0 1 0\n", None),
        ("no-empty", base, "5 2 0 3 2 0\n", None),
    ]
    return {
        "format": "lrat-rup/v1",
        "vectors": [
            {
                "name": name,
                "cnf": cnf,
                "proof": proof,
                "h_cnf": hashlib.sha256(b"logos/cnf/v1\0" + cnf.encode()).hexdigest(),
                "h_proof": hashlib.sha256(b"logos/certificate/v1\0" + proof.encode()).hexdigest(),
                "accepted": used is not None,
                "used": used,
            }
            for name, cnf, proof, used in cases
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = json.dumps(vectors(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not PATH.exists() or PATH.read_text() != data:
            raise SystemExit("vetores LRAT divergentes")
    else:
        PATH.parent.mkdir(parents=True, exist_ok=True)
        PATH.write_text(data)
    print("LRAT_VECTORS_OK count=14")


if __name__ == "__main__":
    main()
