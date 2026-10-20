"""500 obrigações mistas no kernel real: sem chaves no Python."""

from __future__ import annotations

import argparse
import asyncio
import hashlib
import json
import os
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

from logos_client import air, canon
from logos_client.client import KernelClient
from logos_client.protocol import EventRecord, Propose, array
from logos_client.staging import StagingUploader
from logos_provers.farm import Candidate, ProverFarm, Sandbox

ROOT = Path(__file__).resolve().parents[2]


VOCABULARY = "vocab v1 { pred P(); pred Q(); pred R(); } "


def goals(count: int) -> list[tuple[bytes, str]]:
    bases = [
        "P() -> P()",
        "P() or not P()",
        "(P() and Q()) -> P()",
        "not (P() and not P())",
        "(P() -> Q()) <-> (not P() or Q())",
        "((P() -> Q()) and (Q() -> R())) -> (P() -> R())",
        "(P() and (Q() or R())) <-> ((P() and Q()) or (P() and R()))",
        "not (P() or Q()) <-> (not P() and not Q())",
        "(P() <-> Q()) -> (Q() <-> P())",
        "(P() or Q()) -> (Q() or P())",
    ]
    result: list[tuple[bytes, str]] = []
    for index in range(count):
        tautologies = [
            "(Q() -> Q())" if (index // 2) & (1 << bit) else "(R() or not R())" for bit in range(8)
        ]
        base = bases[(index // 2) % len(bases)]
        goal = "(" + base + ") and " + " and ".join(tautologies)
        if index % 2:
            goal += " and P()"
        formula = air.parse_document(VOCABULARY + "claim G:" + goal + ";").claims[0].formula
        result.append((canon.encode(formula), "verified" if index % 2 == 0 else "refuted"))
    assert len({goal for goal, _ in result}) == count
    return result


async def benchmark(root: Path, public: dict[str, str], count: int) -> dict[str, object]:
    client = KernelClient(root / "s", timeout=30)
    farm = ProverFarm(
        client,
        StagingUploader(root / "staging"),
        Sandbox(ROOT / "target/debug/examples/prover_bridge", ROOT / "target/cadical/cadical"),
        timeout=30,
    )
    vocabulary = bytes.fromhex(public["vocabulary_bytes"])
    rows: list[dict[str, object]] = []
    for index, (goal, expected) in enumerate(goals(count)):
        proposed = await client.propose(
            Propose(
                text_pt=f"farm benchmark {index}",
                goal=goal,
                vocabulary=bytes.fromhex(public["vocabulary"]),
            )
        )
        result = await farm.run(proposed.obligation_bytes, proposed.obligation, vocabulary)
        if result.result is None or result.result.state != expected:
            raise RuntimeError(f"case {index} expected {expected}: {result}")
        status = await client.status(result.result.event)
        if status is None or status.kind != (2 if expected == "verified" else 5):
            raise RuntimeError("checker result absent from ledger projection")
        rows.append(
            {
                "obligation": proposed.obligation.hex(),
                "state": result.result.state,
                "event": result.result.event.hex(),
                "verdict_seconds": result.verdict_elapsed,
                "total_seconds": result.elapsed,
                "witness": None if result.witness is None else result.witness.outcome,
                "diagnostics": result.diagnostics,
            }
        )
        if (index + 1) % 50 == 0:
            print(f"FARM_PROGRESS {index + 1}/{count}", flush=True)
    # Reproposal gets a fresh Pending; cached candidates must create fresh checked events.
    cached_goal = goals(count)[-2][0]

    async def repeat() -> tuple[bytes, bytes]:
        proposal = await client.propose(
            Propose(
                text_pt="cache control",
                goal=cached_goal,
                vocabulary=bytes.fromhex(public["vocabulary"]),
            )
        )
        return proposal.obligation_bytes, proposal.obligation

    repeated_o, repeated_hash = await repeat()
    hit = await farm.run(repeated_o, repeated_hash, vocabulary)
    if not hit.cache_hit or hit.result is None or hit.result.state != "verified":
        raise RuntimeError("cache hit was not rechecked")
    if hit.result.event == bytes.fromhex(str(rows[-2]["event"])):
        raise RuntimeError("cache reused a historical acceptance")
    repeated_o, repeated_hash = await repeat()
    farm._cache[repeated_hash] = Candidate("proof", b"invalid cached proof")
    corrupted = await farm.run(repeated_o, repeated_hash, vocabulary)
    if (
        not corrupted.cache_hit
        or corrupted.result is None
        or not any("cache: checker rejected_pending" in d for d in corrupted.diagnostics)
    ):
        raise RuntimeError("corrupted cache did not pass through rejection and fallback")
    head = await client.head()
    assert head is not None
    recorded = {}
    for start in range(0, head.seq + 1, 256):
        async for event in client.subscribe(start, max_events=min(256, head.seq + 1 - start)):
            if isinstance(event, EventRecord):
                recorded[event.hash.hex()] = array(array(canon.decode(event.canonical_bytes), 8)[5])
    # Every reported verdict names an actual Verified/Refuted event, carrying its O.
    for row in rows:
        body = recorded[str(row["event"])]
        if body[1] != (4 if row["state"] == "verified" else 5):
            raise RuntimeError("reported verdict is not a checker event")
        check = array(body[2], 5)
        if check[4] != [1, 0]:
            raise RuntimeError("verdict without an Accepted check")
        if bytes.fromhex(str(row["obligation"])) != check[0]:
            raise RuntimeError("event does not bind the reported obligation")
    return {
        "count": count,
        "distinct_obligations": len({str(row["obligation"]) for row in rows}),
        "p50_seconds_to_verdict": statistics.median(
            float(str(row["verdict_seconds"])) for row in rows
        ),
        "verified": sum(row["state"] == "verified" for row in rows),
        "refuted": sum(row["state"] == "refuted" for row in rows),
        "checker_events_bound": len(rows),
        "cache_recheck": True,
        "corrupted_cache_rejected_then_fallback": True,
        "witness_searches_requested": len(rows),
        "kernel_sha256": hashlib.sha256(
            (ROOT / "target/release/logos-kernel").read_bytes()
        ).hexdigest(),
        "cases": rows,
    }


def run(report: Path, count: int = 500) -> None:
    with tempfile.TemporaryDirectory(prefix="logos-farm-") as directory:
        root = Path(directory)
        public_vocabulary = root / "public-vocabulary"
        public_vocabulary.write_bytes(canon.encode(air.parse_document(VOCABULARY).vocabulary))
        public = json.loads(
            subprocess.run(
                [
                    str(ROOT / "target/debug/examples/client_fixture"),
                    str(root),
                    str(public_vocabulary),
                ],
                capture_output=True,
                check=True,
                timeout=30,
            ).stdout
        )
        for name in ("staging", "private"):
            (root / name).mkdir(mode=0o700)
        with (root / "kernel.log").open("wb") as log:
            process = subprocess.Popen(
                [
                    str(ROOT / "target/release/logos-kernel"),
                    str(root / "s"),
                    str(root / "ledger"),
                    str(root / "operator.key"),
                    str(root / "staging"),
                    str(root / "private"),
                    str(os.geteuid()),
                ],
                stdout=log,
                stderr=log,
            )
            try:
                deadline = time.monotonic() + 120
                while not (root / "s").exists():
                    if process.poll() is not None or time.monotonic() >= deadline:
                        raise RuntimeError((root / "kernel.log").read_text())
                    time.sleep(0.02)
                evidence = asyncio.run(benchmark(root, public, count))
                report.parent.mkdir(parents=True, exist_ok=True)
                report.write_text(json.dumps(evidence, indent=2) + "\n")
                print(f"FARM_SMOKE_OK count={count} p50={evidence['p50_seconds_to_verdict']}")
            finally:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=10)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, default=ROOT / "target/farm-report.json")
    parser.add_argument("--count", type=int, default=500)
    args = parser.parse_args()
    if not 2 <= args.count <= 500:
        parser.error("count must be 2..500; the completion criterion requires500")
    run(args.report, args.count)


if __name__ == "__main__":
    main()
