"""Worker sem autoridade; executado exclusivamente dentro do sandbox da farm."""

from __future__ import annotations

import json
import sys
from pathlib import Path

from logos_provers import model_finder, sat, z3_models


def main() -> None:
    data = json.loads(sys.stdin.buffer.read(4_198_400))
    obligation, vocabulary = bytes.fromhex(data["obligation"]), bytes.fromhex(data["vocabulary"])
    mode, milliseconds = data["mode"], data["timeout_ms"]
    candidate: bytes | None = None
    kind = "model"
    if mode == "sat":
        result = sat.produce(
            obligation,
            vocabulary,
            encoder=Path("/tools/encoder"),
            solver=Path("/tools/solver"),
            limits=sat.Limits(timeout_ms=milliseconds),
        )
        candidate = result.certificate if result.certificate is not None else result.model
        kind = "proof" if result.certificate is not None else "model"
    elif mode == "finder":
        candidate = model_finder.refute(
            obligation, vocabulary, limits=model_finder.Limits(timeout_ms=milliseconds)
        ).model
    elif mode in ("refute", "witness"):
        function = z3_models.witness if mode == "witness" else z3_models.refute
        candidate = function(
            obligation, vocabulary, limits=z3_models.Limits(timeout_ms=milliseconds)
        )
    else:
        raise ValueError("unknown worker mode")
    print(json.dumps({"kind": kind, "candidate": None if candidate is None else candidate.hex()}))


if __name__ == "__main__":
    main()
