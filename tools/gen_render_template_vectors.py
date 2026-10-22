#!/usr/bin/env python3
"""Fixtures vocabulary/v2 com CBOR e hashlib independentes do runtime LOGOS."""

import hashlib
import sys
from pathlib import Path

from gen_vocabulary_vectors import encode

root = Path(__file__).resolve().parent.parent
sort = [1, 0, "S"]
sig = [1, 1, "P", 2, [sort, sort], [1]]
base = [2, 7, [["S", [0]]], [[sig, [2, "glosa"], "documentação"]]]
rows = ["# name\thex\thash-or-error"]
cases = [
    ("valid", ["{0} é dono de {1}"], True),
    ("swapped", ["{1} é dono de {0}"], False),
    ("repeated", ["{0} é dono de {0}"], False),
    ("leading-zero", ["{00} é dono de {1}"], False),
    ("missing", ["{0} é dono"], False),
    ("count", [], False),
    ("extra", ["a", "b"], False),
    ("literal-brace", ["{0} é {x} {1}"], False),
    ("reserved", ["{0} é ⟦ {1}"], False),
    ("bidi", ["{0} é \u202e {1}"], False),
    ("control", ["{0} é\n {1}"], False),
    ("no-literal", ["{0} {1}"], False),
    ("too-long", ["x" * 4097], False),
    ("changed", ["{0} administra {1}"], True),
]
for name, templates, ok in cases:
    wire = encode([*base, templates])
    outcome = hashlib.sha256(b"logos/vocab/v1\0" + wire).hexdigest() if ok else "error"
    rows.append(f"{name}\t{wire.hex()}\t{outcome}")
contents = "\n".join(rows) + "\n"
path = root / "spec/vectors/vocabulary-v2/vectors.tsv"
if "--check" in sys.argv:
    assert path.read_text() == contents
else:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(contents)
print("RENDER_TEMPLATE_VECTORS_OK cases=" + str(len(cases)))
