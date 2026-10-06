"""Contexto AIR/model-v1 dos produtores não confiáveis; sem solver ou autoridade."""

from __future__ import annotations

import hashlib
from collections.abc import Iterator
from dataclasses import dataclass
from typing import Any

from logos_client import air, canon

LOGIC_ID = "FOL-ms-eq/v1"
MAX_UINT32 = 0xFFFF_FFFF
MAX_UINT64 = 0xFFFF_FFFF_FFFF_FFFF
MAX_MODEL_CELLS = 100_000
MAX_MODEL_BYTES = 1_048_576


class ProducerError(ValueError):
    """Erro em entrada ou limites do produtor de modelos."""


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
