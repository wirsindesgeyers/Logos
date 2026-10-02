#!/usr/bin/env python3
"""Gera os vetores de teste normativos de `core/v1` de forma independente do Rust.

Utiliza a biblioteca de referência canônica `py/logos_client/canon.py` para codificar
os tipos de `core/v1` (Budget, Exhausted, Reason, CheckOutcome, Verdict) conforme a
especificação `spec/core.md`.

Uso:
    python tools/gen_core_vectors.py          # regrava spec/vectors/core/vectors.tsv
    python tools/gen_core_vectors.py --check  # confere se os vetores estão em sincronia
"""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "py"))

import logos_client.canon as canon  # noqa: E402

OUT = ROOT / "spec/vectors/core/vectors.tsv"

# Versão normativa do formato core/v1.
CORE_VERSION = 1


def _to_hex(val: Any) -> str:
    """Codifica um valor canônico para hexadecimal minúsculo."""
    return canon.encode(val).hex()


def build_vectors() -> list[tuple[str, str, str, str]]:
    """Constrói todos os vetores normativos (positivos e negativos) de core/v1.

    Retorna lista de tuplas `(name, type, hex, expected)`.
    """
    vectors: list[tuple[str, str, str, str]] = []

    def add_ok(name: str, target_type: str, raw_hex: str, expected_json: Any) -> None:
        formatted = json.dumps(expected_json, ensure_ascii=False, separators=(",", ":"))
        vectors.append((name, target_type, raw_hex, f"ok:{formatted}"))

    def add_err(name: str, target_type: str, raw_hex: str, err_name: str) -> None:
        vectors.append((name, target_type, raw_hex, f"err:{err_name}"))

    # =========================================================================
    # 1. VETORES POSITIVOS: BUDGET
    # =========================================================================
    budgets: list[tuple[str, int, int, int, int]] = [
        ("budget-zero", 0, 0, 0, 0),
        ("budget-one", 1, 1, 1, 1),
        ("budget-mixed", 10_000, 67_108_864, 16_777_216, 5_000),
        ("budget-boundary-small", 23, 24, 255, 256),
        ("budget-boundary-medium", 65_535, 65_536, 4_294_967_295, 4_294_967_296),
        (
            "budget-max",
            18_446_744_073_709_551_615,
            18_446_744_073_709_551_615,
            18_446_744_073_709_551_615,
            18_446_744_073_709_551_615,
        ),
        ("budget-zero-wall", 500, 1_048_576, 65_536, 0),
    ]
    for b_name, steps, mem, size, wall in budgets:
        val = [CORE_VERSION, steps, mem, size, wall]
        raw_hex = _to_hex(val)
        exp = {"steps": steps, "mem_bytes": mem, "size_bytes": size, "wall_ms": wall}
        add_ok(b_name, "Budget", raw_hex, exp)

    # =========================================================================
    # 2. VETORES POSITIVOS: EXHAUSTED
    # =========================================================================
    exhausted_cases: list[tuple[str, int, str]] = [
        ("exhausted-time", 0, "Time"),
        ("exhausted-steps", 1, "Steps"),
        ("exhausted-memory", 2, "Memory"),
        ("exhausted-size", 3, "Size"),
    ]
    for e_name, tag, tag_name in exhausted_cases:
        val = [CORE_VERSION, tag]
        add_ok(e_name, "Exhausted", _to_hex(val), tag_name)

    # =========================================================================
    # 3. VETORES POSITIVOS: REASON
    # =========================================================================
    reason_cases: list[tuple[str, str]] = [
        ("reason-empty", ""),
        ("reason-ascii", "invalid signature"),
        ("reason-ascii-symbols", "step limit: 42 <= 100!"),
        ("reason-nfc-accents", "validação falhou: café"),
        ("reason-nfc-hangul", "검증 오류"),
        ("reason-nfc-emoji", "prova recusada ❌"),
    ]
    for r_name, text in reason_cases:
        reason_value = [CORE_VERSION, text]
        add_ok(r_name, "Reason", _to_hex(reason_value), {"text": text})

    # =========================================================================
    # 4. VETORES POSITIVOS: CHECKOUTCOME
    # =========================================================================
    # Accepted: [1, 0]
    add_ok("outcome-accepted", "CheckOutcome", _to_hex([CORE_VERSION, 0]), "Accepted")

    # Rejected: [1, 1, [1, text]]
    rejected_cases: list[tuple[str, str]] = [
        ("outcome-rejected-empty", ""),
        ("outcome-rejected-ascii", "proof malformed"),
        ("outcome-rejected-nfc", "rejeitado: café inválido"),
    ]
    for o_name, text in rejected_cases:
        rejected_value = [CORE_VERSION, 1, [CORE_VERSION, text]]
        add_ok(o_name, "CheckOutcome", _to_hex(rejected_value), {"Rejected": {"text": text}})

    # Unknown: [1, 2, [1, tag]]
    for _e_name, tag, tag_name in exhausted_cases:
        o_name = f"outcome-unknown-{tag_name.lower()}"
        unknown_value = [CORE_VERSION, 2, [CORE_VERSION, tag]]
        add_ok(o_name, "CheckOutcome", _to_hex(unknown_value), {"Unknown": tag_name})

    # =========================================================================
    # 5. VETORES POSITIVOS: VERDICT
    # =========================================================================
    verdict_cases: list[tuple[str, int, str]] = [
        ("verdict-verified", 0, "Verified"),
        ("verdict-refuted", 1, "Refuted"),
        ("verdict-unknown", 2, "Unknown"),
    ]
    for v_name, tag, tag_name in verdict_cases:
        val = [CORE_VERSION, tag]
        add_ok(v_name, "Verdict", _to_hex(val), tag_name)

    # =========================================================================
    # 6. VETORES NEGATIVOS: VERSÕES DESCONHECIDAS OU INVÁLIDAS (WrongType)
    # =========================================================================
    invalid_versions = [
        ("version-0", 0),
        ("version-2", 2),
        ("version-neg", -1),
        ("version-text", "1"),
    ]
    for suffix, bad_ver in invalid_versions:
        add_err(f"err-budget-{suffix}", "Budget", _to_hex([bad_ver, 0, 0, 0, 0]), "WrongType")
        add_err(f"err-exhausted-{suffix}", "Exhausted", _to_hex([bad_ver, 0]), "WrongType")
        add_err(f"err-reason-{suffix}", "Reason", _to_hex([bad_ver, ""]), "WrongType")
        add_err(f"err-outcome-{suffix}", "CheckOutcome", _to_hex([bad_ver, 0]), "WrongType")
        add_err(f"err-verdict-{suffix}", "Verdict", _to_hex([bad_ver, 0]), "WrongType")

    add_err("err-budget-version-null", "Budget", _to_hex([None, 0, 0, 0, 0]), "WrongType")
    add_err("err-verdict-version-null", "Verdict", _to_hex([None, 0]), "WrongType")

    # =========================================================================
    # 7. VETORES NEGATIVOS: TAGS DESCONHECIDAS / INVÁLIDAS (WrongType)
    # =========================================================================
    # Exhausted: tag fora de 0..3
    add_err("err-exhausted-tag-4", "Exhausted", _to_hex([CORE_VERSION, 4]), "WrongType")
    add_err("err-exhausted-tag-255", "Exhausted", _to_hex([CORE_VERSION, 255]), "WrongType")
    add_err("err-exhausted-tag-neg", "Exhausted", _to_hex([CORE_VERSION, -1]), "WrongType")
    add_err("err-exhausted-tag-text", "Exhausted", _to_hex([CORE_VERSION, "Steps"]), "WrongType")

    # CheckOutcome: tag fora de 0..2
    add_err("err-outcome-tag-3", "CheckOutcome", _to_hex([CORE_VERSION, 3]), "WrongType")
    add_err("err-outcome-tag-255", "CheckOutcome", _to_hex([CORE_VERSION, 255]), "WrongType")
    add_err("err-outcome-tag-neg", "CheckOutcome", _to_hex([CORE_VERSION, -1]), "WrongType")
    add_err(
        "err-outcome-tag-text", "CheckOutcome", _to_hex([CORE_VERSION, "Accepted"]), "WrongType"
    )

    # Verdict: tag fora de 0..2
    add_err("err-verdict-tag-3", "Verdict", _to_hex([CORE_VERSION, 3]), "WrongType")
    add_err("err-verdict-tag-255", "Verdict", _to_hex([CORE_VERSION, 255]), "WrongType")
    add_err("err-verdict-tag-neg", "Verdict", _to_hex([CORE_VERSION, -1]), "WrongType")
    add_err("err-verdict-tag-text", "Verdict", _to_hex([CORE_VERSION, "Verified"]), "WrongType")

    # =========================================================================
    # 8. VETORES NEGATIVOS: ARIDADES ERRADAS (WrongType)
    # =========================================================================
    # Budget: aridade esperada 5
    add_err("err-budget-arity-0", "Budget", _to_hex([]), "WrongType")
    add_err("err-budget-arity-1", "Budget", _to_hex([CORE_VERSION]), "WrongType")
    add_err("err-budget-arity-4", "Budget", _to_hex([CORE_VERSION, 0, 0, 0]), "WrongType")
    add_err(
        "err-budget-arity-6",
        "Budget",
        _to_hex([CORE_VERSION, 0, 0, 0, 0, 0]),
        "WrongType",
    )

    # Exhausted: aridade esperada 2
    add_err("err-exhausted-arity-0", "Exhausted", _to_hex([]), "WrongType")
    add_err("err-exhausted-arity-1", "Exhausted", _to_hex([CORE_VERSION]), "WrongType")
    add_err("err-exhausted-arity-3", "Exhausted", _to_hex([CORE_VERSION, 0, 0]), "WrongType")

    # Reason: aridade esperada 2
    add_err("err-reason-arity-0", "Reason", _to_hex([]), "WrongType")
    add_err("err-reason-arity-1", "Reason", _to_hex([CORE_VERSION]), "WrongType")
    add_err("err-reason-arity-3", "Reason", _to_hex([CORE_VERSION, "", ""]), "WrongType")

    # CheckOutcome:
    # Accepted: aridade esperada 2
    add_err("err-outcome-arity-0", "CheckOutcome", _to_hex([]), "WrongType")
    add_err("err-outcome-arity-1", "CheckOutcome", _to_hex([CORE_VERSION]), "WrongType")
    add_err(
        "err-outcome-accepted-extra",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 0, 0]),
        "WrongType",
    )
    add_err(
        "err-outcome-accepted-extra-text",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 0, "ok"]),
        "WrongType",
    )
    # Rejected: aridade esperada 3
    add_err(
        "err-outcome-rejected-missing-payload",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 1]),
        "WrongType",
    )
    add_err(
        "err-outcome-rejected-extra-payload",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 1, [CORE_VERSION, ""], 0]),
        "WrongType",
    )
    # Unknown: aridade esperada 3
    add_err(
        "err-outcome-unknown-missing-payload",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 2]),
        "WrongType",
    )
    add_err(
        "err-outcome-unknown-extra-payload",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 2, [CORE_VERSION, 0], 0]),
        "WrongType",
    )

    # Verdict: aridade esperada 2
    add_err("err-verdict-arity-0", "Verdict", _to_hex([]), "WrongType")
    add_err("err-verdict-arity-1", "Verdict", _to_hex([CORE_VERSION]), "WrongType")
    add_err("err-verdict-arity-3", "Verdict", _to_hex([CORE_VERSION, 0, 0]), "WrongType")

    # =========================================================================
    # 9. VETORES NEGATIVOS: NÃO-ARRAY (WrongType)
    # =========================================================================
    add_err("err-budget-not-array-int", "Budget", _to_hex(1), "WrongType")
    add_err("err-budget-not-array-text", "Budget", _to_hex("budget"), "WrongType")
    add_err("err-exhausted-not-array", "Exhausted", _to_hex(1), "WrongType")
    add_err("err-reason-not-array", "Reason", _to_hex("reason"), "WrongType")
    add_err("err-outcome-not-array", "CheckOutcome", _to_hex(0), "WrongType")
    add_err("err-verdict-not-array", "Verdict", _to_hex(0), "WrongType")

    # =========================================================================
    # 10. VETORES NEGATIVOS: PAYLOADS ERRADOS (WrongType)
    # =========================================================================
    # Reason com payload não-texto
    add_err("err-reason-payload-int", "Reason", _to_hex([CORE_VERSION, 42]), "WrongType")
    add_err("err-reason-payload-bytes", "Reason", _to_hex([CORE_VERSION, b"abc"]), "WrongType")
    add_err("err-reason-payload-null", "Reason", _to_hex([CORE_VERSION, None]), "WrongType")
    add_err("err-reason-payload-array", "Reason", _to_hex([CORE_VERSION, []]), "WrongType")

    # CheckOutcome::Rejected com payload inválido
    add_err(
        "err-outcome-rejected-payload-text",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 1, "text"]),
        "WrongType",
    )
    add_err(
        "err-outcome-rejected-payload-int",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 1, 0]),
        "WrongType",
    )
    add_err(
        "err-outcome-rejected-payload-wrong-version",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 1, [2, ""]]),
        "WrongType",
    )
    add_err(
        "err-outcome-rejected-payload-wrong-arity",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 1, [CORE_VERSION]]),
        "WrongType",
    )

    # CheckOutcome::Unknown com payload inválido
    add_err(
        "err-outcome-unknown-payload-int",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 2, 0]),
        "WrongType",
    )
    add_err(
        "err-outcome-unknown-payload-wrong-version",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 2, [2, 0]]),
        "WrongType",
    )
    add_err(
        "err-outcome-unknown-payload-wrong-tag",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 2, [CORE_VERSION, 4]]),
        "WrongType",
    )
    add_err(
        "err-outcome-unknown-payload-wrong-arity",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 2, [CORE_VERSION]]),
        "WrongType",
    )

    # =========================================================================
    # 11. VETORES NEGATIVOS: CAMPOS BUDGET NEGATIVOS (IntOutOfRange)
    # =========================================================================
    add_err(
        "err-budget-neg-steps",
        "Budget",
        _to_hex([CORE_VERSION, -1, 0, 0, 0]),
        "IntOutOfRange",
    )
    add_err(
        "err-budget-neg-mem",
        "Budget",
        _to_hex([CORE_VERSION, 0, -1, 0, 0]),
        "IntOutOfRange",
    )
    add_err(
        "err-budget-neg-size",
        "Budget",
        _to_hex([CORE_VERSION, 0, 0, -1, 0]),
        "IntOutOfRange",
    )
    add_err(
        "err-budget-neg-wall",
        "Budget",
        _to_hex([CORE_VERSION, 0, 0, 0, -1]),
        "IntOutOfRange",
    )

    # =========================================================================
    # 12. VETORES NEGATIVOS: CAMPOS BUDGET DE TIPO ERRADO (WrongType)
    # =========================================================================
    add_err(
        "err-budget-type-steps",
        "Budget",
        _to_hex([CORE_VERSION, "0", 0, 0, 0]),
        "WrongType",
    )
    add_err(
        "err-budget-type-mem",
        "Budget",
        _to_hex([CORE_VERSION, 0, "0", 0, 0]),
        "WrongType",
    )
    add_err(
        "err-budget-type-size",
        "Budget",
        _to_hex([CORE_VERSION, 0, 0, "0", 0]),
        "WrongType",
    )
    add_err(
        "err-budget-type-wall",
        "Budget",
        _to_hex([CORE_VERSION, 0, 0, 0, "0"]),
        "WrongType",
    )
    add_err(
        "err-budget-null-steps",
        "Budget",
        _to_hex([CORE_VERSION, None, 0, 0, 0]),
        "WrongType",
    )
    add_err(
        "err-budget-bool-wall",
        "Budget",
        _to_hex([CORE_VERSION, 0, 0, 0, True]),
        "WrongType",
    )

    # =========================================================================
    # 13. VETORES NEGATIVOS: TEXTO NÃO-NFC (NotNfc)
    # =========================================================================
    # String não decomposta 'e' + U+0301 (combining acute accent) = 3 bytes 0x65, 0xcc, 0x81
    raw_not_nfc_reason = b"\x82\x01\x63\x65\xcc\x81".hex()
    add_err("err-reason-not-nfc", "Reason", raw_not_nfc_reason, "NotNfc")

    raw_not_nfc_outcome = b"\x83\x01\x01\x82\x01\x63\x65\xcc\x81".hex()
    add_err("err-outcome-rejected-not-nfc", "CheckOutcome", raw_not_nfc_outcome, "NotNfc")

    # =========================================================================
    # 14. VETORES NEGATIVOS: UTF-8 INVÁLIDO (InvalidUtf8)
    # =========================================================================
    # String de texto com byte inválido 0xff
    raw_invalid_utf8_reason = b"\x82\x01\x61\xff".hex()
    add_err("err-reason-invalid-utf8", "Reason", raw_invalid_utf8_reason, "InvalidUtf8")

    # =========================================================================
    # 15. VETORES NEGATIVOS: ENCODING NÃO-MÍNIMO (NonMinimalHead)
    # =========================================================================
    # Versão 1 codificada com 2 bytes (0x18, 0x01) em vez de 1 byte (0x01)
    raw_nonminimal_head_ver = b"\x85\x18\x01\x00\x00\x00\x00".hex()
    add_err(
        "err-budget-nonminimal-head-version",
        "Budget",
        raw_nonminimal_head_ver,
        "NonMinimalHead",
    )

    # Cabeçalho de array com 2 bytes (0x98, 0x05) em vez de (0x85)
    raw_nonminimal_head_arr = b"\x98\x05\x01\x00\x00\x00\x00".hex()
    add_err(
        "err-budget-nonminimal-head-array",
        "Budget",
        raw_nonminimal_head_arr,
        "NonMinimalHead",
    )

    # Tag 0 codificada como 2 bytes (0x18, 0x00) em Exhausted
    raw_nonminimal_head_tag = b"\x82\x01\x18\x00".hex()
    add_err(
        "err-exhausted-nonminimal-head-tag",
        "Exhausted",
        raw_nonminimal_head_tag,
        "NonMinimalHead",
    )

    # Versão 1 codificada como 2 bytes em Verdict
    raw_nonminimal_verdict = b"\x82\x18\x01\x00".hex()
    add_err(
        "err-verdict-nonminimal-head-version",
        "Verdict",
        raw_nonminimal_verdict,
        "NonMinimalHead",
    )

    # =========================================================================
    # 16. VETORES NEGATIVOS: TRAILING BYTES (TrailingBytes)
    # =========================================================================
    # Adicionar 0x00 após objetos válidos
    add_err(
        "err-budget-trailing",
        "Budget",
        _to_hex([CORE_VERSION, 0, 0, 0, 0]) + "00",
        "TrailingBytes",
    )
    add_err(
        "err-exhausted-trailing",
        "Exhausted",
        _to_hex([CORE_VERSION, 0]) + "00",
        "TrailingBytes",
    )
    add_err(
        "err-reason-trailing",
        "Reason",
        _to_hex([CORE_VERSION, ""]) + "00",
        "TrailingBytes",
    )
    add_err(
        "err-outcome-trailing",
        "CheckOutcome",
        _to_hex([CORE_VERSION, 0]) + "00",
        "TrailingBytes",
    )
    add_err(
        "err-verdict-trailing",
        "Verdict",
        _to_hex([CORE_VERSION, 0]) + "00",
        "TrailingBytes",
    )

    # =========================================================================
    # 17. VETORES NEGATIVOS: ENTRADA VAZIA E TRUNCAMENTO (Eof)
    # =========================================================================
    add_err("err-empty-bytes", "Budget", "", "Eof")
    add_err("err-budget-truncated-head", "Budget", "850100", "Eof")
    add_err("err-exhausted-truncated", "Exhausted", "8201", "Eof")

    return vectors


def render_tsv(vectors: list[tuple[str, str, str, str]]) -> str:
    """Renderiza a lista de vetores como arquivo TSV normativo."""
    lines = [
        "# name\ttype\thex\texpected — spec/core.md §4",
        "# expected = ok:<JSON> | err:<CanonErrorKind>; hex vazio representa entrada vazia",
    ]
    for name, target_type, raw_hex, expected in vectors:
        lines.append(f"{name}\t{target_type}\t{raw_hex}\t{expected}")
    return "\n".join(lines) + "\n"


def main() -> int:
    """Ponto de entrada do gerador de vetores de core/v1."""
    vectors = build_vectors()
    rendered = render_tsv(vectors)

    if "--check" in sys.argv:
        if not OUT.exists():
            print(
                f"erro: arquivo de vetores {OUT.relative_to(ROOT)} não existe; "
                "rode python tools/gen_core_vectors.py",
                file=sys.stderr,
            )
            return 1
        current = OUT.read_text(encoding="utf-8")
        if current != rendered:
            print(
                f"erro: vetores desatualizados em {OUT.relative_to(ROOT)}; "
                "rode python tools/gen_core_vectors.py",
                file=sys.stderr,
            )
            return 1
        print(f"ok: vetores de core atualizados ({len(vectors)} vetores conferidos)")
        return 0

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(rendered, encoding="utf-8")
    print(f"escrito {OUT.relative_to(ROOT)} ({len(vectors)} vetores)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
