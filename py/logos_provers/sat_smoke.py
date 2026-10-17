"""Corpus público:300 políticas e300 pares SAT nos checkers Rust/cake/model."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any

from logos_provers.sat import Candidate, Limits, Status, produce
from logos_provers.sat_cases import Case, policy_cases, reconstruction_cases

ROOT = Path(__file__).resolve().parents[2]


def manifest(cases: list[Case]) -> dict[str, Any]:
    return {
        "version": 1,
        "valid": 300,
        "sat": 300,
        "cases": [
            {
                "name": c.name,
                "family": c.family,
                "sat": c.sat,
                "obligation_sha256": hashlib.sha256(
                    b"logos/obligation/v1\0" + c.obligation
                ).hexdigest(),
                "vocabulary_sha256": hashlib.sha256(b"logos/vocab/v1\0" + c.vocabulary).hexdigest(),
            }
            for c in cases
        ],
    }


def request(case: Case, **fields: str) -> dict[str, Any]:
    return {
        "version": 1,
        "vocabulary": case.vocabulary.hex(),
        "obligation": case.obligation.hex(),
        **fields,
    }


def check_requests(
    encoder: Path,
    cake: Path,
    requests: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    result = subprocess.run(
        [str(encoder), "--cake-lpr", str(cake)],
        input="".join(json.dumps(r) + "\n" for r in requests),
        capture_output=True,
        text=True,
        timeout=600,
        check=True,
    )
    responses = [json.loads(line) for line in result.stdout.splitlines()]
    if len(responses) != len(requests) or result.stderr:
        raise RuntimeError("checker transport incomplete")
    return responses


def run(encoder: Path, solver: Path, cake: Path, report_path: Path) -> None:
    cases = policy_cases()
    fixed = json.loads((ROOT / "spec/vectors/sat/corpus.json").read_text())
    if manifest(cases) != fixed:
        raise RuntimeError("corpus differs from fixed manifest")
    cases.extend(reconstruction_cases())
    candidates: list[Candidate] = []
    checks: list[dict[str, Any]] = []
    records: list[dict[str, Any]] = []
    for index, case in enumerate(cases):
        result = produce(
            case.obligation,
            case.vocabulary,
            encoder=encoder,
            solver=solver,
            limits=Limits(timeout_ms=10_000),
        )
        candidates.append(result)
        expected = Status.SAT if case.sat else Status.UNSAT
        if result.status != expected:
            raise RuntimeError(
                f"{case.name}: expected {expected}, got {result.status}/{result.reason}"
            )
        record = {
            "name": case.name,
            "family": case.family,
            "status": str(result.status),
            "h_obligation": result.obligation_hash.hex(),
            "h_cnf": result.cnf_hash.hex(),
            "cnf_bytes": len(result.cnf),
            "layer": 1 if result.status == Status.UNSAT else 0,
        }
        if result.status == Status.UNSAT:
            assert result.certificate is not None and result.certificate_hash is not None
            checks.append(
                request(case, mode="proof", cnf=result.cnf.hex(), proof=result.certificate.hex())
            )
            record["h_certificate"] = result.certificate_hash.hex()
            record["certificate_bytes"] = len(result.certificate)
        else:
            assert result.model is not None
            checks.append(request(case, mode="model", model=result.model.hex()))
            record["h_model"] = hashlib.sha256(b"logos/model/v1\0" + result.model).hexdigest()
            record["certificate_bytes"] = len(result.model)
        records.append(record)
        if (index + 1) % 50 == 0:
            print(f"SAT_PRODUCER_PROGRESS cases={index + 1}/{len(cases)}", flush=True)
    # Os pares SAT demonstram também a consistência de cada conjunto de premissas válido.
    for index in range(0, 600, 2):
        model = candidates[index + 1].model
        assert model is not None
        checks.append(request(cases[index], mode="witness", model=model.hex()))
    print(f"SAT_PRODUCER_RECHECK total={len(checks)}", flush=True)
    responses = check_requests(encoder, cake, checks)
    proofs = models = witnesses = 0
    for index, (check, response) in enumerate(zip(checks, responses, strict=True)):
        if response.get("version") != 1 or response.get("ok") is not True:
            raise RuntimeError(f"recheck {index}: {response}")
        outcome = response["result"]
        if check["mode"] == "proof":
            if outcome != {"rust": "Accepted", "cake": "Accepted"}:
                raise RuntimeError(f"proof {index}: {outcome}")
            proofs += 1
        else:
            if outcome != {"model": "Accepted", "accepted": True}:
                raise RuntimeError(f"model {index}: {outcome}")
            if check["mode"] == "witness":
                witnesses += 1
            else:
                models += 1
        if index < len(records):
            records[index]["recheck"] = outcome
    positives = records[:600:2]
    if len({r["h_cnf"] for r in positives}) != 300:
        raise RuntimeError("positive CNFs are not distinct")
    report = {
        "version": 1,
        "solver_sha256": hashlib.sha256(solver.read_bytes()).hexdigest(),
        "cake_sha256": hashlib.sha256(cake.read_bytes()).hexdigest(),
        "encoder_sha256": hashlib.sha256(encoder.read_bytes()).hexdigest(),
        "policy_valid": 300,
        "policy_sat": 300,
        "proofs_rechecked": proofs,
        "models_rechecked": models,
        "consistent_premises": witnesses,
        "families": dict(Counter(r["family"] for r in positives)),
        "records": records,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    print(
        f"SAT_PRODUCER_OK valid=300/300 distinct_cnf=300 sat=300/300 "
        f"controls=7 proofs={proofs} models={models} consistent={witnesses} claims=0",
        flush=True,
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--encoder", type=Path, default=ROOT / "target/debug/examples/prover_bridge"
    )
    parser.add_argument("--solver", type=Path, default=ROOT / "target/cadical/cadical")
    parser.add_argument("--cake-lpr", type=Path, default=ROOT / "target/cake-lpr/cake_lpr")
    parser.add_argument("--report", type=Path, default=ROOT / "target/sat-producer-report.json")
    args = parser.parse_args()
    run(args.encoder.resolve(), args.solver.resolve(), args.cake_lpr.resolve(), args.report)


if __name__ == "__main__":
    main()
