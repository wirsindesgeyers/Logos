#!/usr/bin/env python3
"""Independent kernel-protocol/v1 fixtures; does not invoke the Rust codec."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from gen_air_vectors import head
from gen_protocol_vectors import fixtures as proposals

OUT = Path(__file__).resolve().parent.parent / "spec/vectors/kernel-protocol/vectors.json"


def encode(value: object) -> bytes:
    """Small independent canonical encoder for this array-only protocol."""
    if isinstance(value, bytes):
        return head(2, len(value)) + value
    if isinstance(value, str):
        raw = value.encode()
        return head(3, len(raw)) + raw
    if isinstance(value, int):
        return head(0, value)
    if isinstance(value, list):
        return head(4, len(value)) + b"".join(encode(v) for v in value)
    raise TypeError(type(value))


def fixtures() -> dict[str, Any]:
    """Preserve the six proposal/v1 candidates and cover every new command."""
    vectors: list[dict[str, object]] = []

    def add(name: str, value: object, valid: bool = True) -> None:
        wire = encode(value)
        vectors.append(
            {
                "id": name,
                "valid": valid,
                "wire_hex": wire.hex(),
                "frame_hex": (len(wire).to_bytes(4, "big") + wire).hex(),
            }
        )

    for index, vector in enumerate(proposals()["vectors"]):
        p = vector["expected"]
        add(
            "propose_" + vector["id"],
            [
                1,
                index,
                0,
                [
                    p["text_pt"].encode(),
                    bytes.fromhex(p["goal"]),
                    [bytes.fromhex(h) for h in p["cites"]],
                    [bytes.fromhex(h) for h in p["speculative_on"]],
                    bytes.fromhex(p["vocab"]),
                ],
            ],
        )
    h = bytes(range(32))
    goal = encode([1, 3, "FOL-ms-eq/v1", [[0]]])
    commands: list[tuple[str, int, list[object]]] = [
        ("certify_cas", 1, [h, "lrat-rup/v1", [0, h]]),
        ("certify_stage", 1, [h, "lrat-rup/v1", [1, "ab" * 32, h]]),
        ("refute", 2, [h, [0, h]]),
        ("witness", 3, [h, [1, "12" * 32, h]]),
        ("assume", 4, [h, goal, "local"]),
        ("query_head", 5, [0]),
        ("query_status", 5, [1, h]),
        ("subscribe", 6, [0, 256]),
    ]
    for index, (name, tag, fields) in enumerate(commands):
        add(name, [1, index + 10, tag, fields])
        add(name + "_extra", [1, index + 10, tag, [*fields, 0]], False)
    for name, value in [
        ("version", [2, 1, 5, [0]]),
        ("unknown_tag", [1, 1, 7, []]),
        ("unknown_tag_valid_query_shape", [1, 1, 7, [0]]),
        ("extra_envelope", [1, 1, 5, [0], 0]),
        ("wrong_hash", [1, 1, 2, [b"short", [0, h]]]),
        ("reference_extra", [1, 1, 1, [h, "lrat", [0, h, 0]]]),
        ("reference_unknown", [1, 1, 1, [h, "lrat", [2, h]]]),
        ("path_traversal", [1, 1, 1, [h, "lrat", [1, "../" + "a" * 61, h]]]),
        ("zero_events", [1, 1, 6, [0, 0]]),
        ("events_limit", [1, 1, 6, [0, 257]]),
        ("invalid_utf8", [1, 1, 0, [b"\xff", goal, [], [], h]]),
        ("invalid_air", [1, 1, 0, [b"text", b"\x00", [], [], h]]),
    ]:
        add(name, value, False)
    return {"format": "kernel-protocol/v1", "vectors": vectors}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = json.dumps(fixtures(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if OUT.read_text() != data:
            raise SystemExit("kernel-protocol vectors differ")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(data)
    print("KERNEL_PROTOCOL_VECTORS_OK")


if __name__ == "__main__":
    main()
