#!/usr/bin/env python3
"""Vetores notebook/v1 (5R.3) com CBOR e hashlib independentes do runtime LOGOS."""

import hashlib
import json
import sys
from pathlib import Path

from gen_vocabulary_vectors import encode as encode_scalar
from gen_vocabulary_vectors import head

root = Path(__file__).resolve().parent.parent


def encode(value: object) -> bytes:
    """CBOR independente; acrescenta null (0xf6) ao subconjunto do gerador 1.2."""
    if value is None:
        return b"\xf6"
    if isinstance(value, list):
        return head(4, len(value)) + b"".join(encode(item) for item in value)
    return encode_scalar(value)


LOGIC = "FOL-ms-eq/v1"


def pred(name: str) -> list[object]:
    return [3, [1, 1, name, 2, [], [1]], []]


IMPLIES = [1, 3, LOGIC, [[7], pred("Esgoto"), pred("Nutrientes")]]
ATOM = [1, 3, LOGIC, [pred("Nutrientes")]]
PROV = ["fonte", "pilot.py#X", "método", "ator", 1]
HV = bytes(range(32))


def artifact(
    ident: int, kind: int, nature: int, formula: object, refs: list[object], **extra: object
) -> list[object]:
    return [
        bytes([ident]) * 16,
        kind,
        nature,
        1,
        f"A{ident}",
        "enunciado",
        formula,
        PROV,
        refs,
        extra.get("limitations", []),
        extra.get("attachment"),
        extra.get("hint"),
    ]


def bundle(
    artifacts: list[object], links: list[list[object]], scope: str = "session:v"
) -> list[object]:
    return [1, "logos/notebook", [b"\x01" * 16, "Projeto", scope, HV, 1], artifacts, links]


hypothesis = artifact(2, 1, 0, IMPLIES, [[0, b"\x0a" * 32, None]], limitations=["hipótese"])
conclusion = artifact(3, 5, 5, ATOM, [[1, b"\x0b" * 32, b"\x0c" * 32]], hint=[7, "verified"])
table = artifact(4, 7, 1, None, [], attachment=["text/csv", b"linha,Esgoto\nP1,1\n"])
note = artifact(5, 3, 2, None, [])
links: list[list[object]] = [[b"\x02" * 16, 0, b"\x03" * 16], [b"\x03" * 16, 2, b"\x02" * 16]]

valid = {
    "empty": bundle([], []),
    "complete": bundle([hypothesis, conclusion, table, note], links),
    "evidence-assumption": bundle([artifact(6, 3, 1, ATOM, [[0, b"\x0d" * 32, None]])], []),
}
invalid = {
    "version": [2, *bundle([], [])[1:]],
    "scope": bundle([], [], scope="global"),
    "evidence-as-check": bundle([artifact(6, 3, 1, ATOM, [[1, b"\x0d" * 32, b"\x0e" * 32]])], []),
    "hypothesis-as-check": bundle(
        [artifact(2, 1, 0, IMPLIES, [[1, b"\x0a" * 32, b"\x0c" * 32]])], []
    ),
    "check-without-obligation": bundle([artifact(3, 5, 5, ATOM, [[1, b"\x0b" * 32, None]])], []),
    "conclusion-without-formula": bundle([artifact(3, 5, 5, None, [])], []),
    "unsorted-links": bundle([hypothesis, conclusion], list(reversed(links))),
    "dangling-link": bundle([hypothesis], [links[0]]),
    "duplicate-id": bundle([hypothesis, hypothesis], []),
    "non-nfc": bundle([artifact(5, 3, 2, None, [], limitations=["cafe\u0301"])], []),
}
contents = (
    json.dumps(
        {
            "format": "notebook/v1",
            "valid": [
                {
                    "name": name,
                    "bundle": encode(value).hex(),
                    "digest": hashlib.sha256(b"logos/notebook/v1\0" + encode(value)).hexdigest(),
                }
                for name, value in valid.items()
            ],
            "invalid": [
                {"name": name, "bundle": encode(value).hex()} for name, value in invalid.items()
            ],
        },
        indent=2,
        ensure_ascii=False,
    )
    + "\n"
)
path = root / "spec/vectors/notebook/notebook.json"
if "--check" in sys.argv:
    assert path.read_text() == contents
else:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents)
print(f"NOTEBOOK_VECTORS_OK valid={len(valid)} invalid={len(invalid)}")
