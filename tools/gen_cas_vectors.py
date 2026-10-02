#!/usr/bin/env python3
"""Gera os vetores de teste do CAS (cas/v1) de forma independente do Rust.

Lê as tabelas normativas de `spec/cas.md` (mapeamento tag -> diretório e os exemplos
normativos) e calcula os hashes SHA-256 usando apenas a biblioteca padrão (`hashlib`).

Uso:  python tools/gen_cas_vectors.py          # regrava spec/vectors/cas/vectors.tsv
      python tools/gen_cas_vectors.py --check  # confere se os vetores estão em sincronia
"""

from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "spec/cas.md"
OUT = ROOT / "spec/vectors/cas/vectors.tsv"


def parse_spec_tag_dirs(spec_text: str) -> dict[str, str]:
    """Mapeamento de tag (sem NUL) para nome de diretório a partir de spec/cas.md §1."""
    tag_to_dir: dict[str, str] = {}
    for line in spec_text.splitlines():
        m = re.match(r"^\|\s*`([^`]+)`\s*\|\s*([a-z0-9-]+)\s*\|", line)
        if m:
            tag_to_dir[m.group(1)] = m.group(2)
    return tag_to_dir


def parse_spec_examples(spec_text: str) -> list[tuple[str, str, bytes]]:
    """Exemplos (nome, tag sem NUL, bytes) lidos de spec/cas.md §4."""
    examples: list[tuple[str, str, bytes]] = []
    for line in spec_text.splitlines():
        m = re.match(r"^\|\s*([a-z0-9-]+)\s*\|\s*`([^`]+)`\s*\|\s*`([^`]+)`\s*\|", line)
        if m and m.group(1) != "Exemplo":
            name = m.group(1)
            tag_str = m.group(2)
            raw_hex = m.group(3)
            b = b"" if raw_hex == "-" else bytes.fromhex(raw_hex)
            examples.append((name, tag_str, b))
    return examples


def render_vectors() -> str:
    """Gera o conteúdo TSV dos vetores do CAS."""
    spec_text = SPEC.read_text(encoding="utf-8")
    tag_to_dir = parse_spec_tag_dirs(spec_text)
    examples = parse_spec_examples(spec_text)

    if len(tag_to_dir) != 16:
        raise ValueError(f"esperado 16 domínios em spec/cas.md, encontrados {len(tag_to_dir)}")
    if len(examples) != 3:
        raise ValueError(f"esperado 3 exemplos em spec/cas.md, encontrados {len(examples)}")

    header = "# name\ttag_hex\tbytes_hex\tdigest_hex\trelative_path — spec/cas.md §4"
    rows = [header]

    for name, tag_str, msg_bytes in examples:
        if tag_str not in tag_to_dir:
            raise KeyError(f"tag {tag_str} não encontrada na tabela de domínios")
        tag_bytes = tag_str.encode("ascii") + b"\x00"
        digest = hashlib.sha256(tag_bytes + msg_bytes).hexdigest()
        dir_name = tag_to_dir[tag_str]
        rel_path = f"{dir_name}/{digest[:2]}/{digest[2:]}"
        rows.append(f"{name}\t{tag_bytes.hex()}\t{msg_bytes.hex()}\t{digest}\t{rel_path}")

    return "\n".join(rows) + "\n"


def main() -> int:
    rendered = render_vectors()
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != rendered:
            print(
                f"erro: vetores desatualizados em {OUT.relative_to(ROOT)}; "
                "rode python tools/gen_cas_vectors.py",
                file=sys.stderr,
            )
            return 1
        print("ok: vetores de CAS atualizados")
        return 0

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(rendered, encoding="utf-8")
    count = rendered.count("\n") - 1
    print(f"escrito {OUT.relative_to(ROOT)} ({count} vetores)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
