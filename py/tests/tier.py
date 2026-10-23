"""Escala dos testes por nível de verificação (ci/verify.sh, ADR0057).

`LOGOS_TIER=full` (padrão quando ausente) roda a escala dos critérios; `fast` roda a
mesma lógica com menos casos. Testes cuja asserção depende do tamanho usam o marcador
`heavy` em vez disto.
"""

from __future__ import annotations

import os


def scaled(full: int, fast: int) -> int:
    return fast if os.environ.get("LOGOS_TIER") == "fast" else full
