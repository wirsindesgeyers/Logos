"""Produtor Z3 de modelos finitos (Roadmap Etapa 2.4).

Fora da TCB de correção. Gera contra-modelos e testemunhas finitas no formato
canônico `model/v1` a partir de obrigações e vocabulários untrusted.
"""

from __future__ import annotations

import itertools
import math
import time
from dataclasses import dataclass
from typing import Any

import z3  # type: ignore[import-untyped]

from logos_client import canon
from logos_provers import model_context

__all__ = [
    "Limits",
    "refute",
    "to_smtlib",
    "witness",
]


@dataclass(frozen=True)
class Limits:
    """Limites de busca para o produtor finito."""

    max_domain: int = 4
    timeout_ms: int = 5000

    def __post_init__(self) -> None:
        if not isinstance(self.max_domain, int) or isinstance(self.max_domain, bool):
            raise model_context.ProducerError(
                f"max_domain must be an integer, got {self.max_domain!r}"
            )
        if self.max_domain <= 0 or self.max_domain > model_context.MAX_UINT32:
            raise model_context.ProducerError(
                f"max_domain must be in 1..={model_context.MAX_UINT32}, got {self.max_domain}"
            )
        if not isinstance(self.timeout_ms, int) or isinstance(self.timeout_ms, bool):
            raise model_context.ProducerError(
                f"timeout_ms must be an integer, got {self.timeout_ms!r}"
            )
        if self.timeout_ms <= 0 or self.timeout_ms > model_context.MAX_UINT32:
            raise model_context.ProducerError(
                f"timeout_ms must be in 1..={model_context.MAX_UINT32}"
            )


_DEFAULT_LIMITS = Limits()


# ==============================================================================
# Tradução para SMT-LIB 2 Posicional
# ==============================================================================


def _term_to_smt(
    term: model_context.TermAST, env: list[str], vocab: model_context.ValidatedVocabulary
) -> str:
    if isinstance(term, model_context.ASTVar):
        return env[-1 - term.index]
    if isinstance(term, model_context.ASTApp):
        sym = vocab.symbol_by_name[term.symbol_name]
        pos_sym = f"sym_{sym.index}"
        if not term.args:
            return pos_sym
        args_str = " ".join(_term_to_smt(a, env, vocab) for a in term.args)
        return f"({pos_sym} {args_str})"
    raise model_context.ProducerError(f"unexpected term node {type(term)}")


def _formula_to_smt(
    formula: model_context.FormulaAST, env: list[str], vocab: model_context.ValidatedVocabulary
) -> str:
    if isinstance(formula, model_context.ASTTrue):
        return "true"
    if isinstance(formula, model_context.ASTFalse):
        return "false"
    if isinstance(formula, model_context.ASTEq):
        l_str = _term_to_smt(formula.left, env, vocab)
        r_str = _term_to_smt(formula.right, env, vocab)
        return f"(= {l_str} {r_str})"
    if isinstance(formula, model_context.ASTPred):
        sym = vocab.symbol_by_name[formula.symbol_name]
        pos_sym = f"sym_{sym.index}"
        if not formula.args:
            return pos_sym
        args_str = " ".join(_term_to_smt(a, env, vocab) for a in formula.args)
        return f"({pos_sym} {args_str})"
    if isinstance(formula, model_context.ASTNot):
        return f"(not {_formula_to_smt(formula.child, env, vocab)})"
    if isinstance(formula, model_context.ASTAnd):
        if len(formula.children) == 0:
            return "true"
        if len(formula.children) == 1:
            return _formula_to_smt(formula.children[0], env, vocab)
        inner = " ".join(_formula_to_smt(c, env, vocab) for c in formula.children)
        return f"(and {inner})"
    if isinstance(formula, model_context.ASTOr):
        if len(formula.children) == 0:
            return "false"
        if len(formula.children) == 1:
            return _formula_to_smt(formula.children[0], env, vocab)
        inner = " ".join(_formula_to_smt(c, env, vocab) for c in formula.children)
        return f"(or {inner})"
    if isinstance(formula, model_context.ASTImplies):
        l_str = _formula_to_smt(formula.left, env, vocab)
        r_str = _formula_to_smt(formula.right, env, vocab)
        return f"(=> {l_str} {r_str})"
    if isinstance(formula, model_context.ASTIff):
        l_str = _formula_to_smt(formula.left, env, vocab)
        r_str = _formula_to_smt(formula.right, env, vocab)
        return f"(= {l_str} {r_str})"
    if isinstance(formula, (model_context.ASTForall, model_context.ASTExists)):
        sort_spec = vocab.sort_by_name[formula.sort]
        sort_pos = f"S_{sort_spec.index}"
        var_name = f"v_{len(env)}"
        body_str = _formula_to_smt(formula.child, [*env, var_name], vocab)
        quant = "forall" if isinstance(formula, model_context.ASTForall) else "exists"
        return f"({quant} (({var_name} {sort_pos})) {body_str})"
    raise model_context.ProducerError(f"unexpected formula node {type(formula)}")


def to_smtlib(
    obligation: bytes,
    vocabulary: bytes,
    domains: tuple[int, ...],
    *,
    mode: str = "refute",
) -> str:
    """Traduz obrigação e vocabulário para script SMT-LIB 2 posicional sobre domínios fixos."""
    if mode not in ("refute", "witness"):
        raise model_context.ProducerError(f"invalid mode: {mode!r}; must be 'refute' or 'witness'")

    vocab = model_context._validate_vocabulary(vocabulary)
    oblg = model_context._validate_obligation(obligation, vocab)

    if not isinstance(domains, tuple):
        try:
            domains = tuple(domains)
        except Exception as e:
            raise model_context.ProducerError(f"domains must be a tuple of ints: {e}") from e

    if len(domains) != len(vocab.sorts):
        raise model_context.ProducerError(
            f"domains length mismatch: expected {len(vocab.sorts)}, got {len(domains)}"
        )

    for s_idx, d in enumerate(domains):
        if not isinstance(d, int) or isinstance(d, bool):
            raise model_context.ProducerError(f"domain size must be an integer, got {d!r}")
        if d <= 0:
            raise model_context.ProducerError(
                f"domain size for sort {s_idx} must be positive, got {d}"
            )
        if d > model_context.MAX_UINT32:
            raise model_context.ProducerError(f"domain size for sort {s_idx} exceeds u32: {d}")
        sort_spec = vocab.sorts[s_idx]
        if sort_spec.is_enum and d != len(sort_spec.enum_constants):
            raise model_context.ProducerError(
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


def _extract_model(
    solver: Any,
    ctx: Any,
    vocab: model_context.ValidatedVocabulary,
    domains: tuple[int, ...],
    deadline: float,
) -> bytes | None:
    try:
        model = solver.model()
    except z3.Z3Exception as error:
        raise model_context.ProducerError(f"sat solver did not provide a model: {error}") from error

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
            if cells > model_context.MAX_MODEL_CELLS:
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
                    if cells > model_context.MAX_MODEL_CELLS:
                        return None
                    true_tuples.append(list(coordinates))
                elif not z3.is_false(value):
                    raise model_context.ProducerError("model left a primitive predicate undecided")
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
                    raise model_context.ProducerError(
                        "model output is outside the finite representatives"
                    )
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
        raise model_context.ProducerError(f"model encoding failed: {error}") from error
    if len(wire) > model_context.MAX_MODEL_BYTES or time.monotonic() >= deadline:
        return None
    return wire


def _produce(
    obligation: bytes,
    vocabulary: bytes,
    mode: str,
    limits: Limits,
) -> bytes | None:
    if not isinstance(limits, Limits):
        raise model_context.ProducerError("limits must be a Limits instance")
    deadline = time.monotonic() + limits.timeout_ms / 1000.0
    vocab = model_context._validate_vocabulary(vocabulary)
    model_context._validate_obligation(obligation, vocab)
    sort_fixed: list[int | None] = []
    for sort in vocab.sorts:
        cardinality = len(sort.enum_constants) if sort.is_enum else None
        if cardinality is not None and (cardinality == 0 or cardinality > limits.max_domain):
            return None
        sort_fixed.append(cardinality)

    for domains in model_context._domain_vectors(sort_fixed, limits.max_domain):
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
            raise model_context.ProducerError(f"Z3 rejected generated SMT-LIB: {error}") from error
        if time.monotonic() >= deadline or outcome == z3.unknown:
            return None
        if outcome == z3.unsat:
            continue
        if outcome != z3.sat:
            raise model_context.ProducerError(f"unexpected Z3 result: {outcome}")
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
