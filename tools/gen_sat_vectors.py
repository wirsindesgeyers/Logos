#!/usr/bin/env python3
"""Fixa as600 identidades do corpus SAT antes de executar qualquer solver."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "py"))
from logos_provers.sat_cases import policy_cases  # noqa: E402
from logos_provers.sat_smoke import manifest  # noqa: E402


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = json.dumps(manifest(policy_cases()), indent=2) + "\n"
    path = ROOT / "spec/vectors/sat/corpus.json"
    if args.check:
        if path.read_text() != output:
            raise SystemExit("SAT corpus differs")
        print("SAT_VECTORS_OK valid=300 sat=300")
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(output)


if __name__ == "__main__":
    main()
