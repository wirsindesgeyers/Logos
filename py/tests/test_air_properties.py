"""Generated parse(print(f)) identity, independent of parser-produced ASTs."""

from __future__ import annotations

import pytest

from logos_client import air, canon
from tests.air_gen import corpus


def test_generated_500_formula_values_survive_python_debug_roundtrip() -> None:
    for index, case in enumerate(corpus(500, seed=20261006)):
        printed = air.print_formula(case.formula)
        actual = air.parse_formula(printed, case.vocabulary)
        assert actual == case.formula, (index, printed)


@pytest.mark.parametrize(
    "value",
    [
        [True, 3, "FOL-ms-eq/v1", [[0]]],
        [1, True, "FOL-ms-eq/v1", [[0]]],
        [1, 3, "FOL-ms-eq/v1", [[False]]],
        [1, 3, "FOL-ms-eq/v1", [[5, False]]],
        [
            1,
            3,
            "FOL-ms-eq/v1",
            [
                [
                    2,
                    [1, 2, "FOL-ms-eq/v1", [[0, True, [1, 0, "U"]]]],
                    [1, 2, "FOL-ms-eq/v1", [[0, 0, [1, 0, "U"]]]],
                ]
            ],
        ],
    ],
)
def test_cbor_boolean_is_not_an_unsigned_air_tag_count_or_index(value: canon.Value) -> None:
    with pytest.raises(air.SurfaceError):
        air.print_formula(value)
