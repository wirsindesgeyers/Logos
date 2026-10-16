"""Vetores DIMACS independentes do encoder Rust, derivados do contrato cnf/v1."""

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "py"))
from logos_client import canon  # noqa: E402

OUT = ROOT / "spec/vectors/cnf/vectors.json"


def build() -> dict[str, Any]:
    """Implementa as equivalências da spec, sem chamar código Rust."""
    result = []
    ground = json.loads((ROOT / "spec/vectors/grounding/vectors.json").read_text())
    for case in ground["vectors"]:
        g: Any = canon.decode(bytes.fromhex(case["ground"]))
        ids = []
        next_id = len(g[3])
        records: list[tuple[list[int], list[Any]]] = []

        def add(
            literals: list[int],
            origin: list[Any],
            target: list[tuple[list[int], list[Any]]] = records,
        ) -> None:
            target.append((sorted(literals), origin))

        for node in g[4]:
            tag = node[0]
            if tag == 2:
                ids.append(node[1] + 1)
                continue
            next_id += 1
            v = next_id
            if tag in (0, 1):
                add([v if tag else -v], [3])
            elif tag == 3:
                a = ids[node[1]]
                add([-v, -a], [3])
                add([v, a], [3])
            else:
                children = [ids[i] for i in node[1]]
                for a in children:
                    add([-v, a] if tag == 4 else [v, -a], [3])
                add([v, *[-a for a in children]] if tag == 4 else [-v, *children], [3])
            ids.append(v)
        add([ids[g[5]]], [3])
        for ref, node_id in g[6]:
            add([ids[node_id]], [2 if ref in g[9] else 0, ref.hex()])
        add([-ids[g[7]]], [1])
        records.sort()
        dimacs = f"p cnf {next_id} {len(records)}\n"
        dimacs += "".join(" ".join(map(str, [*literals, 0])) + "\n" for literals, _ in records)
        result.append(
            dict(
                **case,
                cnf=dimacs,
                cnf_digest=hashlib.sha256(b"logos/cnf/v1\0" + dimacs.encode()).hexdigest(),
                artifact=hashlib.sha256(dimacs.encode()).hexdigest(),
                origins=[origin for _, origin in records],
                domain_support=[ref.hex() for ref in g[9]],
            )
        )
    return dict(format="cnf/v1", encoder_version=1, vectors=result)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if OUT.read_text() != output:
            raise SystemExit("CNF vectors differ")
        print("CNF_VECTORS_OK cases=12")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(output)


if __name__ == "__main__":
    main()
