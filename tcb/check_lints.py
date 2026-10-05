#!/usr/bin/env python3
"""Confere lints incondicionais do preâmbulo dos crates strict do manifesto.

Scanner lexical, não parser Rust: comentários/literais são opacos e a leitura de
atributos termina no primeiro item. Validade sintática e enforcement permanecem
com rustc/Clippy. Nenhum atributo de módulo ou cfg_attr conta como lint do crate.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections.abc import Iterator
from pathlib import Path

import tcb

_LITERAL = re.compile(
    r'(?:br|cr|r)(?P<hashes>#{0,255})"[\s\S]*?"(?P=hashes)'
    r'|(?:b|c)?"(?:\\[\s\S]|[^"\\])*"'
    r"|b?'(?:[^'\\\r\n]|\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|[^\r\n]))'"
)
_IDENT = re.compile(r"(?:r#)?[A-Za-z_][A-Za-z_0-9]*")
_LINT_PATH = re.compile(r"(?:[A-Za-z_][A-Za-z_0-9]*::)*[A-Za-z_][A-Za-z_0-9]*")
CLIPPY_LINTS = frozenset({"clippy::unwrap_used", "clippy::expect_used", "clippy::panic"})


def _tokens(source: str) -> Iterator[str]:
    source = source.removeprefix("\ufeff")
    i = 0
    while i < len(source):
        if source[i].isspace():
            i += 1
        elif source.startswith("//", i):
            end = source.find("\n", i + 2)
            i = len(source) if end < 0 else end + 1
        elif source.startswith("/*", i):
            i += 2
            depth = 1
            while i < len(source) and depth:
                if source.startswith("/*", i):
                    depth += 1
                    i += 2
                elif source.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
        elif literal := _LITERAL.match(source, i):
            yield "<literal>"
            i = literal.end()
        elif ident := _IDENT.match(source, i):
            yield ident.group().removeprefix("r#")
            i = ident.end()
        else:
            yield source[i]
            i += 1


def lint_levels(source: str) -> dict[str, str]:
    """Níveis explícitos dos lints no preâmbulo; declaração posterior prevalece."""
    tokens = iter(_tokens(source))
    levels: dict[str, str] = {}
    while next(tokens, None) == "#":
        if next(tokens, None) != "!" or next(tokens, None) != "[":
            break  # atributo externo/item: não é mais preâmbulo de crate
        body: list[str] = []
        closers = ["]"]
        for token in tokens:
            if token in ("[", "(", "{"):
                closers.append({"[": "]", "(": ")", "{": "}"}[token])
            elif token in ("]", ")", "}"):
                if token != closers.pop():
                    return levels  # fonte inválida também será recusada pelo compilador
                if not closers:
                    break
            body.append(token)
        else:
            return levels
        if len(body) < 3 or body[0] not in ("allow", "warn", "deny", "forbid", "expect"):
            continue
        if body[1] != "(" or body[-1] != ")":
            continue
        for name in "".join(body[2:-1]).split(","):
            if _LINT_PATH.fullmatch(name):
                levels[name] = body[0]
    return levels


def missing_lints(source: str) -> tuple[bool, frozenset[str]]:
    levels = lint_levels(source)
    missing = frozenset(lint for lint in CLIPPY_LINTS if levels.get(lint) not in ("deny", "forbid"))
    return levels.get("unsafe_code") != "forbid", missing


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=tcb.ROOT)
    root = parser.parse_args().root
    manifest = tcb.load_manifest(root / "tcb" / "manifest.toml")
    status = 0
    for crate in manifest.get("crate", []):
        if not crate.get("strict"):
            continue
        lib = Path("crates") / crate["name"] / "src" / "lib.rs"
        unsafe_missing, clippy_missing = missing_lints((root / lib).read_text(encoding="utf-8"))
        if unsafe_missing:
            print(f"ERRO: {lib} não contém #![forbid(unsafe_code)]", file=sys.stderr)
            status = 1
        if clippy_missing:
            print(f"ERRO: {lib} não nega clippy::unwrap_used, expect_used e panic", file=sys.stderr)
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main())
