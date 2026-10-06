"""Corpus de 20 refutações determinísticas e ponte Rust para apresentação PT.

Implementação da Etapa 2.6 fora da TCB. Módulo público/importável pelo smoke
sem dependências de desenvolvimento (sem pytest, z3 ou oráculos diferenciais).
"""

from __future__ import annotations

import hashlib
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from logos_client import air, canon
from logos_provers import model_finder as finder

ROOT = Path(__file__).resolve().parents[2]


@dataclass(frozen=True)
class ExplainCase:
    name: str
    source: str
    vocabulary: bytes
    obligation: bytes
    expected_hash: bytes
    model: bytes
    relevant_symbols: tuple[str, ...] = ()
    irrelevant_symbols: tuple[str, ...] = ()
    hidden_symbols: tuple[str, ...] = ()
    sentinel_glosses: tuple[str, ...] = ()
    relevant_sorts: tuple[str, ...] = ()


def obligation(vocabulary: bytes, premises: list[canon.Value], goal: canon.Value) -> bytes:
    """Codifica obrigação wire no formato canônico da lógica FOL-ms-eq/v1."""
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + vocabulary).digest()
    entries: list[canon.Value] = [
        [(index + 1).to_bytes(32, "big"), formula] for index, formula in enumerate(premises)
    ]
    return canon.encode([1, 0, air.LOGIC_ID, 1, h_v, entries, goal])


def case(
    name: str,
    source: str,
    vocabulary: bytes,
    obligation_bytes: bytes,
    model_bytes: bytes,
    *,
    relevant_symbols: tuple[str, ...] = (),
    irrelevant_symbols: tuple[str, ...] = (),
    hidden_symbols: tuple[str, ...] = (),
    sentinel_glosses: tuple[str, ...] = (),
    relevant_sorts: tuple[str, ...] = (),
) -> ExplainCase:
    expected_hash = hashlib.sha256(b"logos/obligation/v1\x00" + obligation_bytes).digest()
    return ExplainCase(
        name=name,
        source=source,
        vocabulary=vocabulary,
        obligation=obligation_bytes,
        expected_hash=expected_hash,
        model=model_bytes,
        relevant_symbols=relevant_symbols,
        irrelevant_symbols=irrelevant_symbols,
        hidden_symbols=hidden_symbols,
        sentinel_glosses=sentinel_glosses,
        relevant_sorts=relevant_sorts,
    )


def document_case(source: str) -> tuple[bytes, bytes]:
    """Extrai bytes da obrigação e do vocabulário de um documento AIR texto.

    O último claim é o objetivo (goal); claims anteriores são premissas.
    """
    document = air.parse_document(source)
    v_bytes = canon.encode(document.vocabulary)
    o_bytes = obligation(
        v_bytes, [claim.formula for claim in document.claims[:-1]], document.claims[-1].formula
    )
    return o_bytes, v_bytes


def check(cases: list[tuple[str, ExplainCase] | ExplainCase]) -> list[str]:
    """Ponte de validação com o checker Rust real via subprocess."""
    normalized: list[tuple[str, ExplainCase]] = []
    for item in cases:
        if isinstance(item, tuple):
            normalized.append(item)
        else:
            normalized.append(("refute", item))
    rows = [
        f"{mode} {item.vocabulary.hex()} {item.obligation.hex()} "
        f"{item.expected_hash.hex()} {item.model.hex()}"
        for mode, item in normalized
    ]
    result = subprocess.run(
        ["cargo", "run", "--quiet", "-p", "logos-model", "--example", "checker_diff", "--"],
        input="\n".join(rows) + "\n",
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
        timeout=300,
    )
    outcomes = result.stdout.splitlines()
    assert len(outcomes) == len(cases), result.stderr
    assert all(outcome in ("Accepted", "Rejected", "Unknown") for outcome in outcomes), (
        result.stderr
    )
    return outcomes


# Definição dos 20 casos com metadados estruturais
_CORPUS_DEFS: list[dict[str, Any]] = [
    {
        "name": "01_open_single_const_pred",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const c: S gloss "alvo";\n'
            '  const unused_c: S gloss "SENTINEL_IRRELEVANT_01";\n'
            '  pred P(S) gloss "propriedade P";\n'
            "}\n"
            "claim H: P(c);\n"
            "claim G: forall x: S. P(x);"
        ),
        "relevant_symbols": ("c", "P"),
        "irrelevant_symbols": ("unused_c",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_IRRELEVANT_01",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "02_enum_binary_equality",
        "source": (
            "vocab v1 {\n"
            "  sort Color = enum [red, blue] from fact "
            '"1111111111111111111111111111111111111111111111111111111111111111";\n'
            '  const red: Color gloss "vermelho";\n'
            '  const blue: Color gloss "azul";\n'
            '  pred UnusedP(Color) gloss "SENTINEL_UNUSED_PRED_02";\n'
            "}\n"
            "claim H: not (red = blue);\n"
            "claim G: red = blue;"
        ),
        "relevant_symbols": ("red", "blue"),
        "irrelevant_symbols": ("UnusedP",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_UNUSED_PRED_02",),
        "relevant_sorts": ("Color",),
    },
    {
        "name": "03_multi_sort_func",
        "source": (
            "vocab v1 {\n"
            "  sort A;\n"
            "  sort B;\n"
            "  sort UnusedSort;\n"
            '  const a: A gloss "ponto A";\n'
            '  const b: B gloss "ponto B";\n'
            '  const unused_c: UnusedSort gloss "SENTINEL_SORT_03";\n'
            '  func g(A): B gloss "mapeador";\n'
            "}\n"
            "claim H: g(a) = b;\n"
            "claim G: forall x: A. g(x) = b;"
        ),
        "relevant_symbols": ("a", "b", "g"),
        "irrelevant_symbols": ("unused_c",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_SORT_03",),
        "relevant_sorts": ("A", "B"),
    },
    {
        "name": "04_nullary_func_and_const",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const c: S gloss "constante c";\n'
            '  func unit(): S gloss "unidade nula";\n'
            '  func unused_fn(S): S gloss "SENTINEL_UNUSED_FN_04";\n'
            "}\n"
            "claim H: unit() = c;\n"
            "claim G: forall x: S. x = unit();"
        ),
        "relevant_symbols": ("c", "unit"),
        "irrelevant_symbols": ("unused_fn",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_UNUSED_FN_04",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "05_nullary_pred_boolean_flag",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const c: S gloss "constante c";\n'
            '  pred Flag() gloss "bandeira";\n'
            '  pred UnusedFlag() gloss "SENTINEL_FLAG_05";\n'
            '  pred P(S) gloss "propriedade P";\n'
            "}\n"
            "claim H: Flag() and P(c);\n"
            "claim G: not Flag();"
        ),
        "relevant_symbols": ("c", "Flag", "P"),
        "irrelevant_symbols": ("UnusedFlag",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_FLAG_05",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "06_dense_order_function",
        "source": (
            "vocab v1 {\n"
            "  sort S = enum [s0, s1] from fact "
            '"1111111111111111111111111111111111111111111111111111111111111111";\n'
            "  sort T = enum [t0, t1] from fact "
            '"2222222222222222222222222222222222222222222222222222222222222222";\n'
            '  const s0: S gloss "estado 0";\n'
            '  const s1: S gloss "estado 1";\n'
            '  const t0: T gloss "tempo 0";\n'
            '  const t1: T gloss "tempo 1";\n'
            '  func f(S, T): S gloss "transicao";\n'
            '  func unused_dense(T, S): T gloss "SENTINEL_DENSE_06";\n'
            "}\n"
            "claim H: f(s0, t0) = s0 and f(s0, t1) = s1 and f(s1, t0) = s1 and f(s1, t1) = s0;\n"
            "claim G: forall x: S. forall y: T. f(x, y) = s0;"
        ),
        "relevant_symbols": ("s0", "s1", "t0", "t1", "f"),
        "irrelevant_symbols": ("unused_dense",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_DENSE_06",),
        "relevant_sorts": ("S", "T"),
    },
    {
        "name": "07_existential_quantifier",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const c: S gloss "testigo c";\n'
            '  const irr_const: S gloss "SENTINEL_CONST_07";\n'
            '  pred P(S) gloss "predicado P";\n'
            "}\n"
            "claim H: not P(c);\n"
            "claim G: forall x: S. not P(x);"
        ),
        "relevant_symbols": ("c", "P"),
        "irrelevant_symbols": ("irr_const",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_CONST_07",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "08_defined_const_with_hidden_dependency",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const hidden_base: S gloss "SENTINEL_HIDDEN_DEP_BASE";\n'
            '  const def_c: S = defined(eq(var(0, "S"), '
            'app(symbol("hidden_base", const, [], "S"), []))) gloss "constante derivada";\n'
            '  pred P(S) gloss "predicado principal";\n'
            "}\n"
            "claim H: P(def_c);\n"
            "claim G: forall x: S. P(x);"
        ),
        "relevant_symbols": ("def_c", "P"),
        "irrelevant_symbols": (),
        "hidden_symbols": ("hidden_base",),
        "sentinel_glosses": ("SENTINEL_HIDDEN_DEP_BASE",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "09_defined_pred_with_hidden_dependency",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const hidden_marker: S gloss "SENTINEL_HIDDEN_MARKER";\n'
            '  pred IsMarker(S) = defined(eq(var(0, "S"), '
            'app(symbol("hidden_marker", const, [], "S"), []))) gloss "predicado de marco";\n'
            '  const c: S gloss "ponto c";\n'
            "}\n"
            "claim H: IsMarker(c);\n"
            "claim G: forall x: S. IsMarker(x);"
        ),
        "relevant_symbols": ("IsMarker", "c"),
        "irrelevant_symbols": (),
        "hidden_symbols": ("hidden_marker",),
        "sentinel_glosses": ("SENTINEL_HIDDEN_MARKER",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "10_connectives_implication_iff",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const c: S gloss "ponto c";\n'
            '  pred P(S) gloss "predicado P";\n'
            '  pred Q(S) gloss "predicado Q";\n'
            '  pred R_irr(S) gloss "SENTINEL_R_10";\n'
            "}\n"
            "claim H: P(c) -> Q(c);\n"
            "claim G: P(c) <-> Q(c);"
        ),
        "relevant_symbols": ("c", "P", "Q"),
        "irrelevant_symbols": ("R_irr",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_R_10",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "11_connectives_disjunction_conjunction",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const c: S gloss "ponto c";\n'
            '  const irr_11: S gloss "SENTINEL_11";\n'
            '  pred A(S) gloss "predicado A";\n'
            '  pred B(S) gloss "predicado B";\n'
            "}\n"
            "claim H: A(c) or B(c);\n"
            "claim G: A(c) and B(c);"
        ),
        "relevant_symbols": ("c", "A", "B"),
        "irrelevant_symbols": ("irr_11",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_11",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "12_multi_sort_three_sorts",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            "  sort T;\n"
            "  sort U;\n"
            '  const s: S gloss "origem";\n'
            '  const t: T gloss "meio";\n'
            '  const u: U gloss "destino";\n'
            '  func f(S): T gloss "salto1";\n'
            '  func g(T): U gloss "salto2";\n'
            '  pred Irr12(U) gloss "SENTINEL_12";\n'
            "}\n"
            "claim H: f(s) = t and g(t) = u;\n"
            "claim G: forall x: S. g(f(x)) = u;"
        ),
        "relevant_symbols": ("s", "t", "u", "f", "g"),
        "irrelevant_symbols": ("Irr12",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_12",),
        "relevant_sorts": ("S", "T", "U"),
    },
    {
        "name": "13_predicate_no_true_tuples",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const c: S gloss "elemento c";\n'
            '  const irr13: S gloss "SENTINEL_13";\n'
            '  pred EmptyP(S) gloss "predicado vazio";\n'
            "}\n"
            "claim H: c = c;\n"
            "claim G: exists x: S. EmptyP(x);"
        ),
        "relevant_symbols": ("c", "EmptyP"),
        "irrelevant_symbols": ("irr13",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_13",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "14_multiple_premises_chain",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const a: S gloss "no a";\n'
            '  const b: S gloss "no b";\n'
            '  const c: S gloss "no c";\n'
            '  func irr14(S): S gloss "SENTINEL_14";\n'
            "}\n"
            "claim H1: not (a = b);\n"
            "claim H2: not (b = c);\n"
            "claim G: not (a = c);"
        ),
        "relevant_symbols": ("a", "b", "c"),
        "irrelevant_symbols": ("irr14",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_14",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "15_function_fixed_point_refutation",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const c: S gloss "semente";\n'
            '  func f(S): S gloss "involucao";\n'
            '  pred Irr15(S, S) gloss "SENTINEL_15";\n'
            "}\n"
            "claim H: f(f(c)) = c and not (f(c) = c);\n"
            "claim G: forall x: S. f(x) = x;"
        ),
        "relevant_symbols": ("c", "f"),
        "irrelevant_symbols": ("Irr15",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_15",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "16_binary_predicate_asymmetry",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const a: S gloss "origem a";\n'
            '  const b: S gloss "origem b";\n'
            '  pred R(S, S) gloss "relacao R";\n'
            '  pred Irr16(S) gloss "SENTINEL_16";\n'
            "}\n"
            "claim H: R(a, b) and not R(b, a);\n"
            "claim G: forall x: S. forall y: S. (R(x, y) -> R(y, x));"
        ),
        "relevant_symbols": ("a", "b", "R"),
        "irrelevant_symbols": ("Irr16",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_16",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "17_nested_quantifiers_alternation",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const irr17: S gloss "SENTINEL_17";\n'
            '  pred R(S, S) gloss "conexoes";\n'
            "}\n"
            "claim H: forall x: S. exists y: S. R(x, y);\n"
            "claim G: exists y: S. forall x: S. R(x, y);"
        ),
        "relevant_symbols": ("R",),
        "irrelevant_symbols": ("irr17",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_17",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "18_enum_three_elements",
        "source": (
            "vocab v1 {\n"
            "  sort Tri = enum [first, second, third] from fact "
            '"3333333333333333333333333333333333333333333333333333333333333333";\n'
            '  const first: Tri gloss "primeiro";\n'
            '  const second: Tri gloss "segundo";\n'
            '  const third: Tri gloss "terceiro";\n'
            '  pred Active(Tri) gloss "ativo";\n'
            '  pred Irr18(Tri) gloss "SENTINEL_18";\n'
            "}\n"
            "claim H: Active(first) and Active(second);\n"
            "claim G: Active(third);"
        ),
        "relevant_symbols": ("first", "second", "third", "Active"),
        "irrelevant_symbols": ("Irr18",),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_18",),
        "relevant_sorts": ("Tri",),
    },
    {
        "name": "19_defined_func_with_hidden_dependency",
        "source": (
            "vocab v1 {\n"
            "  sort S;\n"
            '  const hidden_const: S gloss "SENTINEL_HIDDEN_CONST_19";\n'
            '  func def_fn(S): S = defined(eq(var(0, "S"), '
            'app(symbol("hidden_const", const, [], "S"), []))) gloss "funcao constante definida";\n'
            '  const c: S gloss "ponto c";\n'
            "}\n"
            "claim H: def_fn(c) = c;\n"
            "claim G: forall x: S. def_fn(x) = x;"
        ),
        "relevant_symbols": ("def_fn", "c"),
        "irrelevant_symbols": (),
        "hidden_symbols": ("hidden_const",),
        "sentinel_glosses": ("SENTINEL_HIDDEN_CONST_19",),
        "relevant_sorts": ("S",),
    },
    {
        "name": "20_mixed_enum_open_and_constants",
        "source": (
            "vocab v1 {\n"
            "  sort Item;\n"
            "  sort Status = enum [ok, err] from fact "
            '"4444444444444444444444444444444444444444444444444444444444444444";\n'
            '  const ok: Status gloss "sucesso";\n'
            '  const err: Status gloss "erro";\n'
            '  const it: Item gloss "item foco";\n'
            '  const irr20: Item gloss "SENTINEL_20";\n'
            '  func check_item(Item): Status gloss "validador de item";\n'
            "}\n"
            "claim H: check_item(it) = ok;\n"
            "claim G: forall x: Item. check_item(x) = ok;"
        ),
        "relevant_symbols": ("ok", "it", "check_item"),
        "irrelevant_symbols": ("err", "irr20"),
        "hidden_symbols": (),
        "sentinel_glosses": ("SENTINEL_20",),
        "relevant_sorts": ("Item", "Status"),
    },
]


def corpus_20() -> list[ExplainCase]:
    """Build 20 distinct refutations; fixtures do not authenticate events."""

    cases: list[ExplainCase] = []
    for definition in _CORPUS_DEFS:
        o_bytes, v_bytes = document_case(definition["source"])
        result = finder.refute(o_bytes, v_bytes, limits=finder.Limits(max_domain=3))
        if result.status is not finder.SearchStatus.FOUND or result.model is None:
            raise RuntimeError(f"failed to find countermodel for case {definition['name']}")
        cases.append(
            case(
                name=definition["name"],
                source=definition["source"],
                vocabulary=v_bytes,
                obligation_bytes=o_bytes,
                model_bytes=result.model,
                relevant_symbols=definition["relevant_symbols"],
                irrelevant_symbols=definition["irrelevant_symbols"],
                hidden_symbols=definition["hidden_symbols"],
                sentinel_glosses=definition["sentinel_glosses"],
                relevant_sorts=definition["relevant_sorts"],
            )
        )
    return cases
