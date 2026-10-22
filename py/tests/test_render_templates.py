"""vocabulary/v2: vetores independentes e round-trip/diferencial reais."""

import hashlib
import subprocess
from pathlib import Path

import pytest

from logos_client import air, canon
from logos_provers.model_context import _validate_vocabulary

ROOT = Path(__file__).resolve().parents[2]


def test_independent_v2_vectors() -> None:
    for line in (ROOT / "spec/vectors/vocabulary-v2/vectors.tsv").read_text().splitlines():
        if line.startswith("#"):
            continue
        name, hex_bytes, outcome = line.split("\t")
        raw = bytes.fromhex(hex_bytes)
        value = canon.decode(raw)
        if outcome == "error":
            with pytest.raises(air.SurfaceError):
                air.print_document(air.Document(vocabulary=value, claims=[]))
        else:
            assert hashlib.sha256(b"logos/vocab/v1\0" + raw).hexdigest() == outcome, name
            printed = air.print_document(air.Document(vocabulary=value, claims=[]))
            assert canon.encode(air.parse_document(printed).vocabulary) == raw
            assert _validate_vocabulary(raw).h_v == bytes.fromhex(outcome)


def test_v2_document_differential_and_legacy_names() -> None:
    sources = [
        (
            f'vocab v{i} {{ sort S pred P(S,S) render_pt ["{{0}} relação{i} {{1}}"] }} '
            "claim C: forall x:S. P(x,x)"
        )
        for i in range(100)
    ]
    sources += [
        "vocab v0 { render_pt [] } claim C: true",
        'vocab v1 { pred render_pt() render_pt ["chove"] } claim C: render_pt()',
    ]
    result = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "logos-air-text", "--example", "air_text_diff"],
        cwd=ROOT,
        input="\n".join(s.encode().hex() for s in sources) + "\n",
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    rows = result.stdout.splitlines()
    assert len(rows) == len(sources)
    for source, row in zip(sources, rows, strict=True):
        document = air.parse_document(source)
        assert air.parse_document(air.print_document(document)) == document
        _, vocabulary_hex, _, printed_hex = row.split("\t")
        assert bytes.fromhex(vocabulary_hex) == canon.encode(document.vocabulary)
        assert air.parse_document(bytes.fromhex(printed_hex).decode()) == document


@pytest.mark.parametrize(
    "source",
    [
        "vocab v1 { pred P() render_pt [] }",
        "vocab v1 { render_pt [] render_pt [] }",
        "vocab v1 { pred P() render_pt [P] }",
        'vocab v1 { sort S pred P(S) render_pt ["glosa"] }',
    ],
)
def test_malformed_templates_are_not_surface_success(source: str) -> None:
    with pytest.raises(air.SurfaceError):
        air.parse_document(source)


def test_malformed_v2_declaration_has_controlled_error() -> None:
    with pytest.raises(air.SurfaceError):
        air.print_document(air.Document(vocabulary=[2, 1, [], [[]], ["a"]], claims=[]))
