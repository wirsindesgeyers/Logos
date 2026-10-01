"""Testes da implementação de referência Python da codificação canônica."""

from __future__ import annotations

from pathlib import Path

import pytest

from logos_client import canon
from logos_client.canon import CanonError, CanonMap

VECTORS = Path(__file__).resolve().parents[2] / "spec/vectors/canon/vectors.tsv"


def load_vectors() -> list[tuple[str, str, str]]:
    rows = []
    for line in VECTORS.read_text().splitlines():
        if line and not line.startswith("#"):
            name, hex_, expected = line.split("\t")
            rows.append((name, hex_, expected))
    return rows


@pytest.mark.parametrize(("name", "hex_", "expected"), load_vectors())
def test_normative_vectors(name: str, hex_: str, expected: str) -> None:
    data = bytes.fromhex(hex_)
    assert canon.verdict(data) == expected, name
    if expected.startswith("ok:"):
        assert canon.encode(canon.decode(data)) == data


def test_vector_count() -> None:
    rows = load_vectors()
    assert len(rows) >= 30
    assert sum(1 for r in rows if r[2].startswith("err:")) >= 10


def test_versions() -> None:
    assert canon.CANON_VERSION == "canon/v1"
    assert canon.UNICODE_VERSION == "16.0.0"


def test_map_sorts_by_encoded_key_bytes() -> None:
    m = CanonMap([(False, 0), ("aa", 0), ("b", 0), (-1, 0), (10, 0)])
    assert [k for k, _ in m.entries] == [10, -1, "b", "aa", False]


def test_map_duplicate_key() -> None:
    with pytest.raises(CanonError) as e:
        CanonMap([(1, 0), (1, 1)])
    assert e.value.kind == "DuplicateKey"


@pytest.mark.parametrize("n", [2**64, -(2**64) - 1])
def test_integer_out_of_range(n: int) -> None:
    with pytest.raises(CanonError) as e:
        canon.encode(n)
    assert e.value.kind == "IntOutOfRange"


def test_encoder_rejects_non_canonical_text() -> None:
    for s, kind in [("é", "NotNfc"), ("͸", "UnassignedCodePoint"), ("\ud800", "InvalidUtf8")]:
        with pytest.raises(CanonError) as e:
            canon.encode(s)
        assert e.value.kind == kind


def test_depth_limit() -> None:
    v: canon.Value = None
    for _ in range(canon.MAX_DEPTH):
        v = [v]
    assert canon.decode(canon.encode(v)) == v
    with pytest.raises(CanonError) as e:
        canon.encode([v])
    assert e.value.kind == "DepthExceeded"
