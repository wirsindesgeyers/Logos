"""Produtor Z3 de modelos finitos (Roadmap Etapa 2.4).

Fora da TCB de correção. Gera contra-modelos e testemunhas finitas no formato
canônico `model/v1` a partir de obrigações e vocabulários untrusted.
"""

from __future__ import annotations

import hashlib
import itertools
import math
import time
from collections.abc import Iterator
from dataclasses import dataclass
from typing import Any

import z3  # type: ignore[import-untyped]

from logos_client import air, canon

__all__ = [
    "Limits",
    "ProducerError",
    "refute",
    "to_smtlib",
    "witness",
]

LOGIC_ID = "FOL-ms-eq/v1"
MAX_UINT32 = 0xFFFF_FFFF
MAX_UINT64 = 0xFFFF_FFFF_FFFF_FFFF
MAX_MODEL_CELLS = 100_000
MAX_MODEL_BYTES = 1_048_576


class ProducerError(ValueError):
    """Erro em entrada ou limites do produtor de modelos."""


@dataclass(frozen=True)
class Limits:
    """Limites de busca para o produtor finito."""

    max_domain: int = 4
    timeout_ms: int = 5000

    def __post_init__(self) -> None:
        if not isinstance(self.max_domain, int) or isinstance(self.max_domain, bool):
            raise ProducerError(f"max_domain must be an integer, got {self.max_domain!r}")
        if self.max_domain <= 0 or self.max_domain > MAX_UINT32:
            raise ProducerError(f"max_domain must be in 1..={MAX_UINT32}, got {self.max_domain}")
        if not isinstance(self.timeout_ms, int) or isinstance(self.timeout_ms, bool):
            raise ProducerError(f"timeout_ms must be an integer, got {self.timeout_ms!r}")
        if self.timeout_ms <= 0 or self.timeout_ms > MAX_UINT32:
            raise ProducerError(f"timeout_ms must be in 1..={MAX_UINT32}")


_DEFAULT_LIMITS = Limits()


# ==============================================================================
# AST de Fórmulas e Termos para validação e tradução
# ==============================================================================


class TermAST:
    """Nó base de AST de termos."""


@dataclass(frozen=True)
class ASTVar(TermAST):
    index: int
    sort: str


@dataclass(frozen=True)
class ASTApp(TermAST):
    symbol_name: str
    symbol_kind: int
    symbol_arity: tuple[str, ...]
    symbol_result: str
    args: list[TermAST]


class FormulaAST:
    """Nó base de AST de fórmulas."""


@dataclass(frozen=True)
class ASTTrue(FormulaAST):
    pass


@dataclass(frozen=True)
class ASTFalse(FormulaAST):
    pass


@dataclass(frozen=True)
class ASTEq(FormulaAST):
    left: TermAST
    right: TermAST


@dataclass(frozen=True)
class ASTPred(FormulaAST):
    symbol_name: str
    symbol_arity: tuple[str, ...]
    args: list[TermAST]


@dataclass(frozen=True)
class ASTNot(FormulaAST):
    child: FormulaAST


@dataclass(frozen=True)
class ASTAnd(FormulaAST):
    children: list[FormulaAST]


@dataclass(frozen=True)
class ASTOr(FormulaAST):
    children: list[FormulaAST]


@dataclass(frozen=True)
class ASTImplies(FormulaAST):
    left: FormulaAST
    right: FormulaAST


@dataclass(frozen=True)
class ASTIff(FormulaAST):
    left: FormulaAST
    right: FormulaAST


@dataclass(frozen=True)
class ASTForall(FormulaAST):
    sort: str
    child: FormulaAST


@dataclass(frozen=True)
class ASTExists(FormulaAST):
    sort: str
    child: FormulaAST


# ==============================================================================
# Especificações e Validação de Vocabulário e Obrigação
# ==============================================================================


@dataclass(frozen=True)
class SortSpec:
    index: int
    name: str
    is_enum: bool
    enum_constants: list[str]


@dataclass(frozen=True)
class SymbolSpec:
    index: int
    name: str
    kind: int  # 0: Const, 1: Func, 2: Pred
    arity: tuple[str, ...]
    result: str | None  # None para Pred (Bool), nome do sort para Const/Func
    grounding_kind: int  # 0: Defined, 1: Anchored, 2: Glossed
    defined_body: FormulaAST | None = None


@dataclass(frozen=True)
class ValidatedVocabulary:
    sorts: list[SortSpec]
    sort_by_name: dict[str, SortSpec]
    symbols: list[SymbolSpec]
    symbol_by_name: dict[str, SymbolSpec]
    definition_order: list[int]
    h_v: bytes


@dataclass(frozen=True)
class ValidatedObligation:
    premises: list[FormulaAST]
    goal: FormulaAST


def _parse_sort_ref(val: Any) -> str:
    if not (
        isinstance(val, list)
        and len(val) == 3
        and val[0] == 1
        and val[1] == 0
        and isinstance(val[2], str)
    ):
        raise ProducerError("invalid sort envelope")
    return val[2]


def _parse_term_envelope(val: Any) -> TermAST:
    if not (
        isinstance(val, list)
        and len(val) == 4
        and val[0] == 1
        and val[1] == 2
        and val[2] == LOGIC_ID
        and isinstance(val[3], list)
    ):
        raise ProducerError("invalid term envelope")

    records = val[3]
    pos = 0

    def _parse_term_rec() -> TermAST:
        nonlocal pos
        if pos >= len(records):
            raise ProducerError("premature end of term records")
        rec = records[pos]
        pos += 1
        if not (isinstance(rec, list) and len(rec) >= 1 and isinstance(rec[0], int)):
            raise ProducerError("invalid term record layout")
        tag = rec[0]
        if tag == 0:  # Var: [0, index, Sort]
            if len(rec) != 3 or not isinstance(rec[1], int) or isinstance(rec[1], bool):
                raise ProducerError("invalid Var record")
            if rec[1] < 0 or rec[1] > MAX_UINT64:
                raise ProducerError("Var index out of range")
            sort_name = _parse_sort_ref(rec[2])
            return ASTVar(rec[1], sort_name)
        if tag == 1:  # App: [1, Symbol, child_count]
            if len(rec) != 3 or not isinstance(rec[2], int) or isinstance(rec[2], bool):
                raise ProducerError("invalid App record")
            child_count = rec[2]
            if child_count < 0:
                raise ProducerError("App child_count negative")
            sym_envelope = rec[1]
            if not (
                isinstance(sym_envelope, list)
                and len(sym_envelope) == 6
                and sym_envelope[0] == 1
                and sym_envelope[1] == 1
                and isinstance(sym_envelope[2], str)
                and isinstance(sym_envelope[3], int)
                and isinstance(sym_envelope[4], list)
                and isinstance(sym_envelope[5], list)
            ):
                raise ProducerError("invalid Symbol envelope in App")
            sym_name = sym_envelope[2]
            sym_kind = sym_envelope[3]
            if sym_kind not in (0, 1):
                raise ProducerError("App symbol must be Const or Func")
            arity = tuple(_parse_sort_ref(s) for s in sym_envelope[4])
            res_val = sym_envelope[5]
            if len(res_val) != 2 or res_val[0] != 0:
                raise ProducerError("App symbol result must be Sort")
            res_sort = _parse_sort_ref(res_val[1])
            args = [_parse_term_rec() for _ in range(child_count)]
            return ASTApp(sym_name, sym_kind, arity, res_sort, args)
        raise ProducerError(f"unknown term node tag {tag}")

    result = _parse_term_rec()
    if pos != len(records):
        raise ProducerError("trailing records in term prefix")
    return result


def _parse_formula_envelope(val: Any) -> FormulaAST:
    # Reuse AIR's strict structural/depth/node checks before recursive AST building.
    try:
        air.print_formula(val)
    except air.SurfaceError as error:
        raise ProducerError(f"invalid AIR formula: {error}") from error
    if not (
        isinstance(val, list)
        and len(val) == 4
        and val[0] == 1
        and val[1] == 3
        and val[2] == LOGIC_ID
        and isinstance(val[3], list)
    ):
        raise ProducerError("invalid formula envelope")

    records = val[3]
    pos = 0

    def _parse_formula_rec() -> FormulaAST:
        nonlocal pos
        if pos >= len(records):
            raise ProducerError("premature end of formula records")
        rec = records[pos]
        pos += 1
        if not (isinstance(rec, list) and len(rec) >= 1 and isinstance(rec[0], int)):
            raise ProducerError("invalid formula record layout")
        tag = rec[0]
        if tag == 0:  # True
            if len(rec) != 1:
                raise ProducerError("invalid True record")
            return ASTTrue()
        if tag == 1:  # False
            if len(rec) != 1:
                raise ProducerError("invalid False record")
            return ASTFalse()
        if tag == 2:  # Eq: [2, Term, Term]
            if len(rec) != 3:
                raise ProducerError("invalid Eq record")
            term_left = _parse_term_envelope(rec[1])
            term_right = _parse_term_envelope(rec[2])
            return ASTEq(term_left, term_right)
        if tag == 3:  # Pred: [3, Symbol, [Term...]]
            if len(rec) != 3 or not isinstance(rec[2], list):
                raise ProducerError("invalid Pred record")
            sym_envelope = rec[1]
            if not (
                isinstance(sym_envelope, list)
                and len(sym_envelope) == 6
                and sym_envelope[0] == 1
                and sym_envelope[1] == 1
                and isinstance(sym_envelope[2], str)
                and isinstance(sym_envelope[3], int)
                and isinstance(sym_envelope[4], list)
                and isinstance(sym_envelope[5], list)
            ):
                raise ProducerError("invalid Symbol envelope in Pred")
            sym_name = sym_envelope[2]
            sym_kind = sym_envelope[3]
            if sym_kind != 2:
                raise ProducerError("Pred symbol must have kind Pred (2)")
            arity = tuple(_parse_sort_ref(s) for s in sym_envelope[4])
            if sym_envelope[5] != [1]:
                raise ProducerError("Pred symbol result must be Bool ([1])")
            args = [_parse_term_envelope(t) for t in rec[2]]
            return ASTPred(sym_name, arity, args)
        if tag == 4:  # Not
            if len(rec) != 1:
                raise ProducerError("invalid Not record")
            return ASTNot(_parse_formula_rec())
        if tag == 5:  # And: [5, count]
            if len(rec) != 2 or not isinstance(rec[1], int) or isinstance(rec[1], bool):
                raise ProducerError("invalid And record")
            count = rec[1]
            if count < 0:
                raise ProducerError("And count negative")
            children = [_parse_formula_rec() for _ in range(count)]
            return ASTAnd(children)
        if tag == 6:  # Or: [6, count]
            if len(rec) != 2 or not isinstance(rec[1], int) or isinstance(rec[1], bool):
                raise ProducerError("invalid Or record")
            count = rec[1]
            if count < 0:
                raise ProducerError("Or count negative")
            children = [_parse_formula_rec() for _ in range(count)]
            return ASTOr(children)
        if tag == 7:  # Implies
            if len(rec) != 1:
                raise ProducerError("invalid Implies record")
            left = _parse_formula_rec()
            right = _parse_formula_rec()
            return ASTImplies(left, right)
        if tag == 8:  # Iff
            if len(rec) != 1:
                raise ProducerError("invalid Iff record")
            left = _parse_formula_rec()
            right = _parse_formula_rec()
            return ASTIff(left, right)
        if tag == 9:  # Forall: [9, Sort]
            if len(rec) != 2:
                raise ProducerError("invalid Forall record")
            sort_name = _parse_sort_ref(rec[1])
            child = _parse_formula_rec()
            return ASTForall(sort_name, child)
        if tag == 10:  # Exists: [10, Sort]
            if len(rec) != 2:
                raise ProducerError("invalid Exists record")
            sort_name = _parse_sort_ref(rec[1])
            child = _parse_formula_rec()
            return ASTExists(sort_name, child)
        raise ProducerError(f"unknown formula node tag {tag}")

    result = _parse_formula_rec()
    if pos != len(records):
        raise ProducerError("trailing records in formula prefix")
    return result


def _sort_check_term(term: TermAST, env: list[str], vocab: ValidatedVocabulary) -> str:
    if isinstance(term, ASTVar):
        if term.index >= len(env):
            raise ProducerError(f"unbound variable De Bruijn index {term.index}")
        expected_sort = env[-1 - term.index]
        if expected_sort != term.sort:
            raise ProducerError(
                f"sort mismatch on variable: expected {expected_sort}, got {term.sort}"
            )
        return term.sort

    if isinstance(term, ASTApp):
        sym = vocab.symbol_by_name.get(term.symbol_name)
        if sym is None:
            raise ProducerError(f"unknown symbol in term: {term.symbol_name}")
        if sym.kind not in (0, 1):
            raise ProducerError(f"symbol {term.symbol_name} cannot be used as a term")
        if sym.kind != term.symbol_kind or sym.arity != term.symbol_arity:
            raise ProducerError(f"signature mismatch for symbol {term.symbol_name}")
        assert sym.result is not None
        if sym.result != term.symbol_result:
            raise ProducerError(f"result sort mismatch for symbol {term.symbol_name}")
        if len(term.args) != len(sym.arity):
            raise ProducerError(
                f"arity mismatch for {term.symbol_name}: "
                f"expected {len(sym.arity)}, got {len(term.args)}"
            )
        for arg, expected_s in zip(term.args, sym.arity, strict=True):
            actual_s = _sort_check_term(arg, env, vocab)
            if actual_s != expected_s:
                raise ProducerError(
                    f"argument sort mismatch in {term.symbol_name}: "
                    f"expected {expected_s}, got {actual_s}"
                )
        return sym.result

    raise ProducerError(f"unexpected term AST node {type(term)}")


def _sort_check_formula(formula: FormulaAST, env: list[str], vocab: ValidatedVocabulary) -> None:
    if isinstance(formula, (ASTTrue, ASTFalse)):
        return

    if isinstance(formula, ASTEq):
        sort_l = _sort_check_term(formula.left, env, vocab)
        sort_r = _sort_check_term(formula.right, env, vocab)
        if sort_l != sort_r:
            raise ProducerError(f"equality sort mismatch: {sort_l} != {sort_r}")
        return

    if isinstance(formula, ASTPred):
        sym = vocab.symbol_by_name.get(formula.symbol_name)
        if sym is None:
            raise ProducerError(f"unknown predicate symbol: {formula.symbol_name}")
        if sym.kind != 2:
            raise ProducerError(f"symbol {formula.symbol_name} is not a predicate")
        if sym.arity != formula.symbol_arity:
            raise ProducerError(f"predicate signature mismatch for {formula.symbol_name}")
        if len(formula.args) != len(sym.arity):
            raise ProducerError(
                f"predicate arity mismatch for {formula.symbol_name}: "
                f"expected {len(sym.arity)}, got {len(formula.args)}"
            )
        for arg, expected_s in zip(formula.args, sym.arity, strict=True):
            actual_s = _sort_check_term(arg, env, vocab)
            if actual_s != expected_s:
                raise ProducerError(
                    f"argument sort mismatch in predicate {formula.symbol_name}: "
                    f"expected {expected_s}, got {actual_s}"
                )
        return

    if isinstance(formula, ASTNot):
        _sort_check_formula(formula.child, env, vocab)
        return

    if isinstance(formula, (ASTAnd, ASTOr)):
        for child in formula.children:
            _sort_check_formula(child, env, vocab)
        return

    if isinstance(formula, (ASTImplies, ASTIff)):
        _sort_check_formula(formula.left, env, vocab)
        _sort_check_formula(formula.right, env, vocab)
        return

    if isinstance(formula, (ASTForall, ASTExists)):
        if formula.sort not in vocab.sort_by_name:
            raise ProducerError(f"quantifier references unknown sort: {formula.sort}")
        _sort_check_formula(formula.child, [*env, formula.sort], vocab)
        return

    raise ProducerError(f"unexpected formula AST node {type(formula)}")


def _validate_vocabulary(raw_vocab: bytes) -> ValidatedVocabulary:
    if not isinstance(raw_vocab, bytes):
        raise ProducerError("vocabulary must be bytes")
    try:
        val = canon.decode(raw_vocab)
        # One format convention: the existing public AIR validator also checks
        # strict integer tags, full signatures, grounding and definition cycles.
        air.print_document(air.Document(vocabulary=val, claims=[]))
    except (canon.CanonError, air.SurfaceError) as error:
        raise ProducerError(f"invalid vocabulary: {error}") from error

    raw: Any = val  # Structural layout has been checked by the AIR client.
    sorts: list[SortSpec] = []
    for index, (name, domain) in enumerate(raw[2]):
        is_enum = domain[0] == 1
        constants = [signature[2] for signature in domain[1]] if is_enum else []
        sorts.append(SortSpec(index, name, is_enum, constants))
    sort_by_name = {sort.name: sort for sort in sorts}

    symbols: list[SymbolSpec] = []
    for index, (signature, grounding, _gloss) in enumerate(raw[3]):
        kind = signature[3]
        arity = tuple(_parse_sort_ref(sort) for sort in signature[4])
        if kind in (0, 1):
            if kind == 0 and arity:
                raise ProducerError("constant must have empty arity")
            if len(signature[5]) != 2 or signature[5][0] != 0:
                raise ProducerError("constant/function must return a sort")
            result = _parse_sort_ref(signature[5][1])
        elif kind == 2 and signature[5] == [1]:
            result = None
        else:
            raise ProducerError("invalid symbol kind/result")
        if any(sort not in sort_by_name for sort in arity):
            raise ProducerError("symbol references an undeclared argument sort")
        if result is not None and result not in sort_by_name:
            raise ProducerError("symbol references an undeclared result sort")
        body = _parse_formula_envelope(grounding[1]) if grounding[0] == 0 else None
        symbols.append(SymbolSpec(index, signature[2], kind, arity, result, grounding[0], body))

    vocabulary = ValidatedVocabulary(
        sorts=sorts,
        sort_by_name=sort_by_name,
        symbols=symbols,
        symbol_by_name={symbol.name: symbol for symbol in symbols},
        # Declarations precede all axioms; no topological expansion is needed.
        definition_order=[symbol.index for symbol in symbols if symbol.grounding_kind == 0],
        h_v=hashlib.sha256(b"logos/vocab/v1\x00" + raw_vocab).digest(),
    )
    for index in vocabulary.definition_order:
        symbol = symbols[index]
        assert symbol.defined_body is not None
        environment = list(reversed(symbol.arity))
        if symbol.kind != 2:
            assert symbol.result is not None
            environment.append(symbol.result)
        _sort_check_formula(symbol.defined_body, environment, vocabulary)
    return vocabulary


def _validate_obligation(
    raw_obligation: bytes,
    vocab: ValidatedVocabulary,
) -> ValidatedObligation:
    if not isinstance(raw_obligation, bytes):
        raise ProducerError("obligation must be bytes")
    try:
        val = canon.decode(raw_obligation)
    except canon.CanonError as e:
        raise ProducerError(f"obligation canon decode failed: {e}") from e

    if not (
        isinstance(val, list)
        and len(val) == 7
        and all(type(val[index]) is int for index in (0, 1, 3))
        and val[0] == 1
        and val[1] == 0
        and val[2] == LOGIC_ID
        and val[3] == 1
        and isinstance(val[4], bytes)
        and len(val[4]) == 32
        and isinstance(val[5], list)
    ):
        raise ProducerError("invalid obligation envelope")

    if val[4] != vocab.h_v:
        raise ProducerError("obligation vocabulary digest mismatch with provided vocabulary")

    premises: list[FormulaAST] = []
    last_event_digest: bytes | None = None

    for p_entry in val[5]:
        if not (isinstance(p_entry, list) and len(p_entry) == 2):
            raise ProducerError("invalid premise entry envelope")
        ev_digest = p_entry[0]
        if not (isinstance(ev_digest, bytes) and len(ev_digest) == 32):
            raise ProducerError("invalid premise event digest")
        if last_event_digest is not None and ev_digest <= last_event_digest:
            raise ProducerError("premises event digests must be strictly increasing")
        last_event_digest = ev_digest
        f_ast = _parse_formula_envelope(p_entry[1])
        _sort_check_formula(f_ast, env=[], vocab=vocab)
        premises.append(f_ast)

    goal_ast = _parse_formula_envelope(val[6])
    _sort_check_formula(goal_ast, env=[], vocab=vocab)

    return ValidatedObligation(premises=premises, goal=goal_ast)


# ==============================================================================
# Tradução para SMT-LIB 2 Posicional
# ==============================================================================


def _term_to_smt(term: TermAST, env: list[str], vocab: ValidatedVocabulary) -> str:
    if isinstance(term, ASTVar):
        return env[-1 - term.index]
    if isinstance(term, ASTApp):
        sym = vocab.symbol_by_name[term.symbol_name]
        pos_sym = f"sym_{sym.index}"
        if not term.args:
            return pos_sym
        args_str = " ".join(_term_to_smt(a, env, vocab) for a in term.args)
        return f"({pos_sym} {args_str})"
    raise ProducerError(f"unexpected term node {type(term)}")


def _formula_to_smt(formula: FormulaAST, env: list[str], vocab: ValidatedVocabulary) -> str:
    if isinstance(formula, ASTTrue):
        return "true"
    if isinstance(formula, ASTFalse):
        return "false"
    if isinstance(formula, ASTEq):
        l_str = _term_to_smt(formula.left, env, vocab)
        r_str = _term_to_smt(formula.right, env, vocab)
        return f"(= {l_str} {r_str})"
    if isinstance(formula, ASTPred):
        sym = vocab.symbol_by_name[formula.symbol_name]
        pos_sym = f"sym_{sym.index}"
        if not formula.args:
            return pos_sym
        args_str = " ".join(_term_to_smt(a, env, vocab) for a in formula.args)
        return f"({pos_sym} {args_str})"
    if isinstance(formula, ASTNot):
        return f"(not {_formula_to_smt(formula.child, env, vocab)})"
    if isinstance(formula, ASTAnd):
        if len(formula.children) == 0:
            return "true"
        if len(formula.children) == 1:
            return _formula_to_smt(formula.children[0], env, vocab)
        inner = " ".join(_formula_to_smt(c, env, vocab) for c in formula.children)
        return f"(and {inner})"
    if isinstance(formula, ASTOr):
        if len(formula.children) == 0:
            return "false"
        if len(formula.children) == 1:
            return _formula_to_smt(formula.children[0], env, vocab)
        inner = " ".join(_formula_to_smt(c, env, vocab) for c in formula.children)
        return f"(or {inner})"
    if isinstance(formula, ASTImplies):
        l_str = _formula_to_smt(formula.left, env, vocab)
        r_str = _formula_to_smt(formula.right, env, vocab)
        return f"(=> {l_str} {r_str})"
    if isinstance(formula, ASTIff):
        l_str = _formula_to_smt(formula.left, env, vocab)
        r_str = _formula_to_smt(formula.right, env, vocab)
        return f"(= {l_str} {r_str})"
    if isinstance(formula, (ASTForall, ASTExists)):
        sort_spec = vocab.sort_by_name[formula.sort]
        sort_pos = f"S_{sort_spec.index}"
        var_name = f"v_{len(env)}"
        body_str = _formula_to_smt(formula.child, [*env, var_name], vocab)
        quant = "forall" if isinstance(formula, ASTForall) else "exists"
        return f"({quant} (({var_name} {sort_pos})) {body_str})"
    raise ProducerError(f"unexpected formula node {type(formula)}")


def to_smtlib(
    obligation: bytes,
    vocabulary: bytes,
    domains: tuple[int, ...],
    *,
    mode: str = "refute",
) -> str:
    """Traduz obrigação e vocabulário para script SMT-LIB 2 posicional sobre domínios fixos."""
    if mode not in ("refute", "witness"):
        raise ProducerError(f"invalid mode: {mode!r}; must be 'refute' or 'witness'")

    vocab = _validate_vocabulary(vocabulary)
    oblg = _validate_obligation(obligation, vocab)

    if not isinstance(domains, tuple):
        try:
            domains = tuple(domains)
        except Exception as e:
            raise ProducerError(f"domains must be a tuple of ints: {e}") from e

    if len(domains) != len(vocab.sorts):
        raise ProducerError(
            f"domains length mismatch: expected {len(vocab.sorts)}, got {len(domains)}"
        )

    for s_idx, d in enumerate(domains):
        if not isinstance(d, int) or isinstance(d, bool):
            raise ProducerError(f"domain size must be an integer, got {d!r}")
        if d <= 0:
            raise ProducerError(f"domain size for sort {s_idx} must be positive, got {d}")
        if d > MAX_UINT32:
            raise ProducerError(f"domain size for sort {s_idx} exceeds u32: {d}")
        sort_spec = vocab.sorts[s_idx]
        if sort_spec.is_enum and d != len(sort_spec.enum_constants):
            raise ProducerError(
                f"domain size for enumerated sort {s_idx} must match constants count "
                f"({len(sort_spec.enum_constants)}), got {d}"
            )

    lines: list[str] = [
        ";; LOGOS FOL-ms-eq/v1 SMT-LIB 2 translation (posicional seguro)",
        "(set-logic ALL)",
    ]

    # 1. Declaração de Sorts e fechamento finito
    for s_idx, d in enumerate(domains):
        s_name = f"S_{s_idx}"
        lines.append(f"(declare-sort {s_name} 0)")
        for j in range(d):
            lines.append(f"(declare-const E_{s_idx}_{j} {s_name})")
        if d > 1:
            all_elems = " ".join(f"E_{s_idx}_{j}" for j in range(d))
            lines.append(f"(assert (distinct {all_elems}))")
        if d == 1:
            lines.append(f"(assert (forall ((x {s_name})) (= x E_{s_idx}_0)))")
        else:
            eqs = " ".join(f"(= x E_{s_idx}_{j})" for j in range(d))
            lines.append(f"(assert (forall ((x {s_name})) (or {eqs})))")

    # 2. Declaração de Símbolos posicionais
    for sym in vocab.symbols:
        pos_sym = f"sym_{sym.index}"
        if sym.kind == 0:  # Const
            assert sym.result is not None
            res_s_idx = vocab.sort_by_name[sym.result].index
            lines.append(f"(declare-const {pos_sym} S_{res_s_idx})")
        elif sym.kind == 1:  # Func
            assert sym.result is not None
            res_s_idx = vocab.sort_by_name[sym.result].index
            arg_sorts_str = " ".join(f"S_{vocab.sort_by_name[a].index}" for a in sym.arity)
            lines.append(f"(declare-fun {pos_sym} ({arg_sorts_str}) S_{res_s_idx})")
        elif sym.kind == 2:  # Pred
            if not sym.arity:
                lines.append(f"(declare-const {pos_sym} Bool)")
            else:
                arg_sorts_str = " ".join(f"S_{vocab.sort_by_name[a].index}" for a in sym.arity)
                lines.append(f"(declare-fun {pos_sym} ({arg_sorts_str}) Bool)")

    # 3. Axiomas de constantes em domínios enumerados (representantes fixados)
    for s_idx, sort_spec in enumerate(vocab.sorts):
        if sort_spec.is_enum:
            for ordinal, c_name in enumerate(sort_spec.enum_constants):
                c_sym = vocab.symbol_by_name[c_name]
                lines.append(f"(assert (= sym_{c_sym.index} E_{s_idx}_{ordinal}))")

    # 4. Defined axioms quantify every input/result, including unused symbols.
    # Coverage of each sort makes these exactly the finite evaluator relations.
    for sym_idx in vocab.definition_order:
        sym = vocab.symbols[sym_idx]
        assert sym.defined_body is not None
        arg_terms = [f"a_{sym_idx}_{index}" for index in range(len(sym.arity))]
        bindings = [
            f"({name} S_{vocab.sort_by_name[sort].index})"
            for name, sort in zip(arg_terms, sym.arity, strict=True)
        ]
        application = f"(sym_{sym_idx} {' '.join(arg_terms)})" if arg_terms else f"sym_{sym_idx}"
        environment = list(reversed(arg_terms))
        if sym.kind == 2:
            body = _formula_to_smt(sym.defined_body, environment, vocab)
            relation = f"(= {application} {body})"
        else:
            assert sym.result is not None
            result = f"r_{sym_idx}"
            bindings.append(f"({result} S_{vocab.sort_by_name[sym.result].index})")
            environment.append(result)
            body = _formula_to_smt(sym.defined_body, environment, vocab)
            relation = f"(= (= {application} {result}) {body})"
        if bindings:
            relation = f"(forall ({' '.join(bindings)}) {relation})"
        lines.append(f"(assert {relation})")

    # 5. Premissas de O
    for p in oblg.premises:
        p_str = _formula_to_smt(p, env=[], vocab=vocab)
        lines.append(f"(assert {p_str})")

    # 6. Modo: refute nega o goal; witness não restringe o goal
    if mode == "refute":
        g_str = _formula_to_smt(oblg.goal, env=[], vocab=vocab)
        lines.append(f"(assert (not {g_str}))")

    lines.append("(check-sat)")
    return "\n".join(lines) + "\n"


# ==============================================================================
# Iteração de Vetores de Cardinalidade e Extração Canônica de Modelo
# ==============================================================================


def _domain_vectors(
    sort_fixed: list[int | None],
    max_domain: int,
) -> Iterator[tuple[int, ...]]:
    """Lazy odometer: no recursive stack or materialized range/product pools."""
    values = [1 if fixed is None else fixed for fixed in sort_fixed]
    open_indices = [index for index, fixed in enumerate(sort_fixed) if fixed is None]
    while True:
        yield tuple(values)
        for index in reversed(open_indices):
            if values[index] < max_domain:
                values[index] += 1
                break
            values[index] = 1
        else:
            return


def _extract_model(
    solver: Any,
    ctx: Any,
    vocab: ValidatedVocabulary,
    domains: tuple[int, ...],
    deadline: float,
) -> bytes | None:
    try:
        model = solver.model()
    except z3.Z3Exception as error:
        raise ProducerError(f"sat solver did not provide a model: {error}") from error

    sorts = [z3.DeclareSort(f"S_{index}", ctx=ctx) for index in range(len(domains))]
    elements: list[list[Any]] = []
    values: list[list[Any]] = []
    for index, cardinality in enumerate(domains):
        representatives: list[Any] = []
        interpretations: list[Any] = []
        for ordinal in range(cardinality):
            if time.monotonic() >= deadline:
                return None
            representative = z3.Const(f"E_{index}_{ordinal}", sorts[index])
            representatives.append(representative)
            interpretations.append(model.eval(representative, model_completion=True))
        elements.append(representatives)
        values.append(interpretations)

    consts: list[canon.Value] = []
    funcs: list[canon.Value] = []
    preds: list[canon.Value] = []
    cells = 0
    for symbol in vocab.symbols:
        if time.monotonic() >= deadline:
            return None
        if symbol.grounding_kind == 0:
            continue
        argument_sorts = [vocab.sort_by_name[name].index for name in symbol.arity]
        if symbol.kind != 2:
            cells += math.prod(domains[index] for index in argument_sorts)
            if cells > MAX_MODEL_CELLS:
                return None
            assert symbol.result is not None
            result_index = vocab.sort_by_name[symbol.result].index
            result_sort = sorts[result_index]
        else:
            result_sort = z3.BoolSort(ctx=ctx)
        declaration = z3.Function(
            f"sym_{symbol.index}", *(sorts[index] for index in argument_sorts), result_sort
        )
        outputs: list[int] = []
        true_tuples: list[canon.Value] = []
        ranges = [range(domains[index]) for index in argument_sorts]
        for coordinates in itertools.product(*ranges):
            if time.monotonic() >= deadline:
                return None
            arguments = [
                elements[index][ordinal]
                for index, ordinal in zip(argument_sorts, coordinates, strict=True)
            ]
            value = model.eval(declaration(*arguments), model_completion=True)
            if symbol.kind == 2:
                if z3.is_true(value):
                    cells += max(1, len(coordinates))
                    if cells > MAX_MODEL_CELLS:
                        return None
                    true_tuples.append(list(coordinates))
                elif not z3.is_false(value):
                    raise ProducerError("model left a primitive predicate undecided")
            else:
                output_ordinal = next(
                    (
                        index
                        for index, representative in enumerate(values[result_index])
                        if value.eq(representative)
                    ),
                    None,
                )
                if output_ordinal is None:
                    raise ProducerError("model output is outside the finite representatives")
                outputs.append(output_ordinal)
        if symbol.kind == 0:
            consts.append([symbol.index, outputs[0]])
        elif symbol.kind == 1:
            funcs.append([symbol.index, b"".join(value.to_bytes(4, "big") for value in outputs)])
        else:
            preds.append([symbol.index, true_tuples])

    try:
        wire = canon.encode([1, vocab.h_v, list(domains), consts, funcs, preds])
    except canon.CanonError as error:
        raise ProducerError(f"model encoding failed: {error}") from error
    if len(wire) > MAX_MODEL_BYTES or time.monotonic() >= deadline:
        return None
    return wire


def _produce(
    obligation: bytes,
    vocabulary: bytes,
    mode: str,
    limits: Limits,
) -> bytes | None:
    if not isinstance(limits, Limits):
        raise ProducerError("limits must be a Limits instance")
    deadline = time.monotonic() + limits.timeout_ms / 1000.0
    vocab = _validate_vocabulary(vocabulary)
    _validate_obligation(obligation, vocab)
    sort_fixed: list[int | None] = []
    for sort in vocab.sorts:
        cardinality = len(sort.enum_constants) if sort.is_enum else None
        if cardinality is not None and (cardinality == 0 or cardinality > limits.max_domain):
            return None
        sort_fixed.append(cardinality)

    for domains in _domain_vectors(sort_fixed, limits.max_domain):
        if time.monotonic() >= deadline:
            return None
        script = to_smtlib(obligation, vocabulary, domains, mode=mode)
        if time.monotonic() >= deadline:
            return None
        context = z3.Context()
        solver = z3.Solver(ctx=context)
        try:
            solver.from_string(script)
            remaining_ms = int((deadline - time.monotonic()) * 1000)
            if remaining_ms <= 0:
                return None
            solver.set("timeout", remaining_ms)
            outcome = solver.check()
        except z3.Z3Exception as error:
            raise ProducerError(f"Z3 rejected generated SMT-LIB: {error}") from error
        if time.monotonic() >= deadline or outcome == z3.unknown:
            return None
        if outcome == z3.unsat:
            continue
        if outcome != z3.sat:
            raise ProducerError(f"unexpected Z3 result: {outcome}")
        candidate = _extract_model(solver, context, vocab, domains, deadline)
        if time.monotonic() >= deadline:
            return None
        return candidate
    return None


def refute(
    obligation: bytes,
    vocabulary: bytes,
    *,
    limits: Limits = _DEFAULT_LIMITS,
) -> bytes | None:
    """Busca um contra-modelo finito para a obrigação O sob o vocabulário V."""
    return _produce(obligation, vocabulary, mode="refute", limits=limits)


def witness(
    obligation: bytes,
    vocabulary: bytes,
    *,
    limits: Limits = _DEFAULT_LIMITS,
) -> bytes | None:
    """Busca uma testemunha finita de consistência das premissas de O sob o vocabulário V."""
    return _produce(obligation, vocabulary, mode="witness", limits=limits)
