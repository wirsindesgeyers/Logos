#!/usr/bin/env python3
"""Gera crates/logos-canon/src/unicode_assigned.rs a partir do `unicodedata` do Python.

A tabela lista os intervalos de code points *atribuídos* (categoria geral != Cn) na versão
de Unicode fixada em spec/encoding.md. O decoder canônico rejeita o resto, para que
implementações com tabelas de versões diferentes nunca discordem sobre NFC.

Uso:  python3.14 tools/gen_unicode_assigned.py          # reescreve o arquivo
      python3.14 tools/gen_unicode_assigned.py --check  # falha se o arquivo estiver desatualizado
"""

from __future__ import annotations

import sys
import unicodedata
from pathlib import Path

EXPECTED_VERSION = "16.0.0"
OUT = Path(__file__).resolve().parent.parent / "crates/logos-canon/src/unicode_assigned.rs"
PER_LINE = 4


def assigned_ranges() -> list[tuple[int, int]]:
    ranges: list[tuple[int, int]] = []
    start: int | None = None
    for cp in range(0x110000):
        # Surrogados (Cs) não são escalares Unicode e nunca aparecem em UTF-8 válido.
        ok = cp < 0xD800 or cp > 0xDFFF
        ok = ok and unicodedata.category(chr(cp)) != "Cn"
        if ok and start is None:
            start = cp
        elif not ok and start is not None:
            ranges.append((start, cp - 1))
            start = None
    if start is not None:
        ranges.append((start, 0x10FFFF))
    return ranges


def render() -> str:
    if unicodedata.unidata_version != EXPECTED_VERSION:
        raise SystemExit(
            f"erro: Unicode {unicodedata.unidata_version} no Python; esperado {EXPECTED_VERSION} "
            "(use Python 3.14)"
        )
    major, minor, patch = (int(x) for x in EXPECTED_VERSION.split("."))
    ranges = assigned_ranges()
    cells = [f"(0x{a:04X}, 0x{b:04X})" for a, b in ranges]
    rows = [
        "    " + ", ".join(cells[i : i + PER_LINE]) + "," for i in range(0, len(cells), PER_LINE)
    ]
    body = "\n".join(rows)
    return (
        "// @generated por tools/gen_unicode_assigned.py — NÃO EDITE.\n"
        f"// Unicode {EXPECTED_VERSION}: intervalos de code points atribuídos "
        "(categoria geral != Cn).\n"
        "\n"
        f"pub(crate) const UNICODE_VERSION: (u8, u8, u8) = ({major}, {minor}, {patch});\n"
        "\n"
        "/// Intervalos fechados `(início, fim)`, ordenados e disjuntos.\n"
        "#[rustfmt::skip]\n"
        f"pub(crate) const ASSIGNED: [(u32, u32); {len(ranges)}] = [\n{body}\n];\n"
    )


def main() -> int:
    text = render()
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            print(
                f"erro: {OUT.name} desatualizado; rode tools/gen_unicode_assigned.py",
                file=sys.stderr,
            )
            return 1
        print("ok: unicode_assigned.rs atualizado")
        return 0
    OUT.write_text(text)
    print(f"escrito {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
