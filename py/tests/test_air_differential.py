"""Independent parser differential: exact AIR/Vocabulary bytes, 500 formulas."""

from __future__ import annotations

import subprocess
from pathlib import Path

from logos_client import air, canon
from tests.air_gen import corpus

ROOT = Path(__file__).resolve().parents[2]


def test_500_surface_formulas_match_independent_ast_and_rust() -> None:
    cases = corpus(500)
    documents = []
    for index, case in enumerate(cases):
        document = air.parse_document(case.source)
        assert canon.encode(document.vocabulary) == canon.encode(case.vocabulary), index
        assert document.claims[0].formula == case.formula, (index, case.source)
        assert document.claims[0].name == f"C{index}"
        assert document.claims[0].cites == ["F3", "P2", "C5", "F3"]
        assert air.parse_document(air.print_document(document)) == document, index
        documents.append(document)

    result = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "logos-air-text", "--example", "air_text_diff"],
        cwd=ROOT,
        input="\n".join(case.source.encode().hex() for case in cases) + "\n",
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr + result.stdout
    rows = result.stdout.splitlines()
    assert len(rows) == 500, "Rust must answer every corpus formula exactly once"
    for index, (case, document, row) in enumerate(zip(cases, documents, rows, strict=True)):
        marker, vocabulary_hex, formula_hex, printed_hex = row.split("\t")
        assert marker == "ok", (index, row)
        assert bytes.fromhex(vocabulary_hex) == canon.encode(case.vocabulary), index
        assert bytes.fromhex(formula_hex) == canon.encode(case.formula), (index, case.source)
        rust_printed = bytes.fromhex(printed_hex).decode("utf-8")
        assert air.parse_document(rust_printed) == document, index


def test_cross_language_nominal_precedence_shadowing_and_nary_identity() -> None:
    # Expected AST does not come from either parser or a printer. Distinguishes
    # right-associative implication, left Iff, n-arity and sibling binder scopes.
    prefix = "vocab v0 { sort S pred P(S) pred v1(S) } claim C: "
    texts = [
        "true -> false -> true",
        "true <-> false <-> true",
        "true or false and true",
        "and([forall x:S. P(x),forall y:S. P(y),or([])])",
        "forall x:S. exists x:S. P(x)",
        "forall v01:S. exists v12name:S. P(v01)",
        "forall v1:S. v1(v1)",
        "((true) and false)",
        "(((true) and false) -> true)",
        "(true -> (false -> true)) <-> false",
        "(forall x:S. P(x)) and (forall y:S. P(y))",
        "true -> forall x:S. P(x) and false",
        "not forall x:S. P(x) or false",
    ]
    sort: canon.Value = [1, 0, "S"]
    symbol: canon.Value = [1, 1, "P", 2, [sort], [1]]
    term: canon.Value = [1, 2, "FOL-ms-eq/v1", [[0, 0, sort]]]
    atom: canon.Value = [3, symbol, [term]]
    outer_term: canon.Value = [1, 2, "FOL-ms-eq/v1", [[0, 1, sort]]]
    version_named_symbol: canon.Value = [1, 1, "v1", 2, [sort], [1]]
    expected: list[canon.Value] = [
        [[7], [0], [7], [1], [0]],
        [[8], [8], [0], [1], [0]],
        [[6, 2], [0], [5, 2], [1], [0]],
        [[5, 3], [9, sort], atom, [9, sort], atom, [6, 0]],
        [[9, sort], [10, sort], atom],
        [[9, sort], [10, sort], [3, symbol, [outer_term]]],
        [[9, sort], [3, version_named_symbol, [term]]],
        [[5, 2], [0], [1]],
        [[7], [5, 2], [0], [1], [0]],
        [[8], [7], [0], [7], [1], [0], [1]],
        [[5, 2], [9, sort], atom, [9, sort], atom],
        [[7], [0], [9, sort], [5, 2], atom, [1]],
        [[4], [9, sort], [6, 2], atom, [1]],
    ]
    sources = [prefix + text for text in texts]
    result = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "logos-air-text", "--example", "air_text_diff"],
        cwd=ROOT,
        input="\n".join(source.encode().hex() for source in sources) + "\n",
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr + result.stdout
    for source, records, row in zip(sources, expected, result.stdout.splitlines(), strict=True):
        formula: canon.Value = [1, 3, "FOL-ms-eq/v1", records]
        parsed = air.parse_document(source)
        assert parsed.claims[0].formula == formula
        assert bytes.fromhex(row.split("\t")[2]) == canon.encode(formula)
