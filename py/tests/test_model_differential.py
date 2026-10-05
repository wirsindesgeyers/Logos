"""Teste diferencial Rust x Z3 do avaliador contextual (Roadmap 2.2).

Gera 10.000 pares aleatórios reproduzíveis (M, φ) cobrindo:
  - Múltiplos sorts e domínios variados (Open e Enumerated).
  - Funções densas (unárias, binárias, nulas/constantes).
  - Predicados (inclusive nulários de aridade 0).
  - Quantificadores (Forall, Exists), sombreamento de variáveis e termos mistos.
  - Todos os conectivos (True, False, Eq, Pred, Not, And, Or, Implies, Iff).
  - Predicados Defined com parâmetros livres e funções/constantes Defined totalmente
    e unicamente determinadas.

Compara a avaliação do Z3 (oráculo dev Python independente fora da TCB) contra
o interpretador Rust (via example `eval_diff`).
"""

from __future__ import annotations

import hashlib
import itertools
import random
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import z3  # type: ignore[import-untyped]

from logos_client import canon

ROOT = Path(__file__).resolve().parents[2]
N = 10_000
CARGO = ["cargo", "run", "--quiet", "-p", "logos-model", "--example", "eval_diff", "--"]


# ==============================================================================
# AST de Termos e Fórmulas
# ==============================================================================


class TermTree:
    """Nó base de AST de termos."""


@dataclass(frozen=True)
class VarTerm(TermTree):
    """Variável De Bruijn."""

    index: int
    sort: str


@dataclass(frozen=True)
class AppTerm(TermTree):
    """Aplicação de função ou constante."""

    symbol: SymbolSpec
    args: list[TermTree]


class FormulaTree:
    """Nó base de AST de fórmulas."""


@dataclass(frozen=True)
class TrueFormula(FormulaTree):
    """Verdade lógica."""


@dataclass(frozen=True)
class FalseFormula(FormulaTree):
    """Falsidade lógica."""


@dataclass(frozen=True)
class EqFormula(FormulaTree):
    """Igualdade de termos."""

    left: TermTree
    right: TermTree


@dataclass(frozen=True)
class PredAppFormula(FormulaTree):
    """Aplicação de predicado."""

    symbol: SymbolSpec
    args: list[TermTree]


@dataclass(frozen=True)
class NotFormula(FormulaTree):
    """Negação."""

    child: FormulaTree


@dataclass(frozen=True)
class AndFormula(FormulaTree):
    """Conjunção n-ária."""

    children: list[FormulaTree]


@dataclass(frozen=True)
class OrFormula(FormulaTree):
    """Disjunção n-ária."""

    children: list[FormulaTree]


@dataclass(frozen=True)
class ImpliesFormula(FormulaTree):
    """Implicação."""

    left: FormulaTree
    right: FormulaTree


@dataclass(frozen=True)
class IffFormula(FormulaTree):
    """Bicondicional."""

    left: FormulaTree
    right: FormulaTree


@dataclass(frozen=True)
class ForallFormula(FormulaTree):
    """Quantificador universal."""

    sort: str
    child: FormulaTree


@dataclass(frozen=True)
class ExistsFormula(FormulaTree):
    """Quantificador existencial."""

    sort: str
    child: FormulaTree


# ==============================================================================
# Especificações de Vocabulário e Modelo
# ==============================================================================


@dataclass(frozen=True)
class SortSpec:
    """Especificação de um sort."""

    name: str
    domain_size: int
    is_enum: bool = False
    enum_consts: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class SymbolSpec:
    """Especificação de assinatura de símbolo."""

    name: str
    kind: int  # 0: Const, 1: Func, 2: Pred
    arity: tuple[str, ...]
    result: str | None
    is_defined: bool = False
    defined_body: FormulaTree | None = None


@dataclass
class VocabularySpec:
    """Vocabulário finito tipado."""

    sorts: list[SortSpec]
    symbols: list[SymbolSpec]

    def get_sort(self, name: str) -> SortSpec:
        for s in self.sorts:
            if s.name == name:
                return s
        raise KeyError(name)

    def get_symbol(self, name: str) -> SymbolSpec:
        for sym in self.symbols:
            if sym.name == name:
                return sym
        raise KeyError(name)


@dataclass
class ModelInterpretation:
    """Interpretação finita explícita do vocabulário."""

    vocab: VocabularySpec
    const_values: dict[int, int]
    func_tables: dict[int, list[int]]
    pred_tables: dict[int, list[list[int]]]


# ==============================================================================
# Gerador Canônico Wire (Independente do Rust)
# ==============================================================================


def sort_to_canon(sort_name: str) -> canon.Value:
    return [1, 0, sort_name]


def sig_to_canon(sym: SymbolSpec) -> canon.Value:
    arity_val: list[canon.Value] = [sort_to_canon(s) for s in sym.arity]
    if sym.result is None:
        result_val: canon.Value = [1]
    else:
        result_val = [0, sort_to_canon(sym.result)]
    return [1, 1, sym.name, sym.kind, arity_val, result_val]


def term_to_records(term: TermTree) -> list[canon.Value]:
    if isinstance(term, VarTerm):
        return [[0, term.index, sort_to_canon(term.sort)]]
    if isinstance(term, AppTerm):
        records: list[canon.Value] = [[1, sig_to_canon(term.symbol), len(term.args)]]
        for arg in term.args:
            records.extend(term_to_records(arg))
        return records
    raise ValueError(f"Termo desconhecido: {term}")


def term_to_canon(term: TermTree) -> canon.Value:
    return [1, 2, "FOL-ms-eq/v1", term_to_records(term)]


def formula_to_records(formula: FormulaTree) -> list[canon.Value]:
    if isinstance(formula, TrueFormula):
        return [[0]]
    if isinstance(formula, FalseFormula):
        return [[1]]
    if isinstance(formula, EqFormula):
        return [[2, term_to_canon(formula.left), term_to_canon(formula.right)]]
    if isinstance(formula, PredAppFormula):
        args_val: list[canon.Value] = [term_to_canon(a) for a in formula.args]
        return [[3, sig_to_canon(formula.symbol), args_val]]
    if isinstance(formula, NotFormula):
        return [[4], *formula_to_records(formula.child)]
    if isinstance(formula, AndFormula):
        records: list[canon.Value] = [[5, len(formula.children)]]
        for child in formula.children:
            records.extend(formula_to_records(child))
        return records
    if isinstance(formula, OrFormula):
        records = [[6, len(formula.children)]]
        for child in formula.children:
            records.extend(formula_to_records(child))
        return records
    if isinstance(formula, ImpliesFormula):
        return [[7], *formula_to_records(formula.left), *formula_to_records(formula.right)]
    if isinstance(formula, IffFormula):
        return [[8], *formula_to_records(formula.left), *formula_to_records(formula.right)]
    if isinstance(formula, ForallFormula):
        return [[9, sort_to_canon(formula.sort)], *formula_to_records(formula.child)]
    if isinstance(formula, ExistsFormula):
        return [[10, sort_to_canon(formula.sort)], *formula_to_records(formula.child)]
    raise ValueError(f"Fórmula desconhecida: {formula}")


def formula_to_canon(formula: FormulaTree) -> canon.Value:
    return [1, 3, "FOL-ms-eq/v1", formula_to_records(formula)]


def vocab_to_canon(v: VocabularySpec) -> canon.Value:
    sort_decls: list[canon.Value] = []
    for s in v.sorts:
        if s.is_enum:
            const_sigs: list[canon.Value] = [
                sig_to_canon(v.get_symbol(cname)) for cname in s.enum_consts
            ]
            source_val: canon.Value = [0, b"\x22" * 32]
            domain_val: canon.Value = [1, const_sigs, source_val]
        else:
            domain_val = [0]
        sort_decls.append([s.name, domain_val])

    sym_decls: list[canon.Value] = []
    for sym in v.symbols:
        if sym.is_defined:
            assert sym.defined_body is not None
            grounding_val: canon.Value = [0, formula_to_canon(sym.defined_body)]
        else:
            grounding_val = [2, ""]
        sym_decls.append([sig_to_canon(sym), grounding_val, ""])

    return [1, 1, sort_decls, sym_decls]


def vocab_hash(vocab_val: canon.Value) -> bytes:
    encoded = canon.encode(vocab_val)
    return hashlib.sha256(b"logos/vocab/v1\x00" + encoded).digest()


def model_to_canon(model: ModelInterpretation) -> canon.Value:
    v = model.vocab
    vocab_val = vocab_to_canon(v)
    h_v = vocab_hash(vocab_val)
    domains: list[canon.Value] = [s.domain_size for s in v.sorts]
    consts: list[canon.Value] = []
    funcs: list[canon.Value] = []
    preds: list[canon.Value] = []

    for sym_id, sym in enumerate(v.symbols):
        if sym.is_defined:
            continue
        if sym.kind == 0:
            consts.append([sym_id, model.const_values[sym_id]])
        elif sym.kind == 1:
            outs = model.func_tables[sym_id]
            packed = b"".join(int(x).to_bytes(4, "big") for x in outs)
            funcs.append([sym_id, packed])
        elif sym.kind == 2:
            tuples: list[canon.Value] = [
                [int(value) for value in row] for row in model.pred_tables[sym_id]
            ]
            preds.append([sym_id, tuples])

    return [1, h_v, domains, consts, funcs, preds]


# ==============================================================================
# Oráculo Z3
# ==============================================================================


class Z3Oracle:
    """Oráculo de avaliação semântica independente em Z3."""

    _ids = itertools.count()

    def __init__(self, model: ModelInterpretation) -> None:
        self.vocab = model.vocab
        self.namespace = f"logos_diff_{next(self._ids)}_"
        self.model = model
        self.solver = z3.Solver()
        self.sort_map: dict[str, Any] = {}
        self.sort_elems: dict[str, list[Any]] = {}
        self.funcs: dict[str, Any] = {}
        self.preds: dict[str, Any] = {}
        self._build()

    def _build(self) -> None:
        # 1. Sorts finitos
        for s in self.vocab.sorts:
            z_sort, elems = z3.EnumSort(
                f"{self.namespace}S_{s.name}",
                [f"{self.namespace}{s.name}_{i}" for i in range(s.domain_size)],
            )
            self.sort_map[s.name] = z_sort
            self.sort_elems[s.name] = list(elems)

        # 2. Constantes primitivas
        for sym_id, sym in enumerate(self.vocab.symbols):
            if sym.is_defined or sym.kind != 0:
                continue
            assert sym.result is not None
            c = z3.Const(sym.name, self.sort_map[sym.result])
            val = self.model.const_values[sym_id]
            self.solver.add(c == self.sort_elems[sym.result][val])
            self.funcs[sym.name] = c

        # 3. Funções primitivas densas
        for sym_id, sym in enumerate(self.vocab.symbols):
            if sym.is_defined or sym.kind != 1:
                continue
            assert sym.result is not None
            arg_sorts = [self.sort_map[s] for s in sym.arity]
            res_sort = self.sort_map[sym.result]
            f = z3.Function(sym.name, *arg_sorts, res_sort)
            self.funcs[sym.name] = f() if not sym.arity else f
            ranges = [range(self.vocab.get_sort(s).domain_size) for s in sym.arity]
            for flat_idx, arg_indices in enumerate(itertools.product(*ranges)):
                arg_vals = [
                    self.sort_elems[s][i] for s, i in zip(sym.arity, arg_indices, strict=True)
                ]
                out_idx = self.model.func_tables[sym_id][flat_idx]
                res_val = self.sort_elems[sym.result][out_idx]
                self.solver.add(f(*arg_vals) == res_val)

        # 4. Predicados primitivos
        for sym_id, sym in enumerate(self.vocab.symbols):
            if sym.is_defined or sym.kind != 2:
                continue
            if len(sym.arity) == 0:
                p = z3.Bool(sym.name)
                is_true = len(self.model.pred_tables[sym_id]) > 0
                self.solver.add(p == is_true)
                self.preds[sym.name] = p
            else:
                arg_sorts = [self.sort_map[s] for s in sym.arity]
                p = z3.Function(sym.name, *arg_sorts, z3.BoolSort())
                self.preds[sym.name] = p
                tuple_set = {tuple(t) for t in self.model.pred_tables[sym_id]}
                ranges = [range(self.vocab.get_sort(s).domain_size) for s in sym.arity]
                for arg_indices in itertools.product(*ranges):
                    arg_vals = [
                        self.sort_elems[s][i] for s, i in zip(sym.arity, arg_indices, strict=True)
                    ]
                    self.solver.add(p(*arg_vals) == (arg_indices in tuple_set))

        assert self.solver.check() == z3.sat
        self.z3_model = self.solver.model()

        # Defined symbols become definitional constraints, not a Python evaluator.
        # Declare every head first, permitting acyclic dependencies in any order.
        for sym in self.vocab.symbols:
            if not sym.is_defined:
                continue
            arg_sorts = [self.sort_map[s] for s in sym.arity]
            if sym.kind == 2:
                self.preds[sym.name] = (
                    z3.Function(sym.name, *arg_sorts, z3.BoolSort())
                    if sym.arity
                    else z3.Bool(sym.name)
                )
            else:
                assert sym.result is not None
                self.funcs[sym.name] = (
                    z3.Function(sym.name, *arg_sorts, self.sort_map[sym.result])
                    if sym.arity
                    else z3.Const(sym.name, self.sort_map[sym.result])
                )
        for sym in self.vocab.symbols:
            if not sym.is_defined:
                continue
            assert sym.defined_body is not None
            ranges = [range(self.vocab.get_sort(s).domain_size) for s in sym.arity]
            for indices in itertools.product(*ranges):
                args = [self.sort_elems[s][i] for s, i in zip(sym.arity, indices, strict=True)]
                head = (self.preds if sym.kind == 2 else self.funcs)[sym.name]
                application = head(*args) if sym.arity else head
                if sym.kind == 2:
                    self.solver.add(application == self.translate_formula(sym.defined_body, args))
                else:
                    assert sym.result is not None
                    for candidate in self.sort_elems[sym.result]:
                        relation = self.translate_formula(sym.defined_body, [candidate, *args])
                        self.solver.add((application == candidate) == relation)
        assert self.solver.check() == z3.sat
        self.z3_model = self.solver.model()

    def translate_term(self, term: TermTree, env: list[Any]) -> Any:
        if isinstance(term, VarTerm):
            return env[term.index]
        if isinstance(term, AppTerm):
            fn = self.funcs[term.symbol.name]
            if term.symbol.kind == 0 or len(term.symbol.arity) == 0:
                return fn
            args_z3 = [self.translate_term(a, env) for a in term.args]
            return fn(*args_z3)
        raise ValueError(f"Termo desconhecido: {term}")

    def translate_formula(self, formula: FormulaTree, env: list[Any]) -> Any:
        if isinstance(formula, TrueFormula):
            return z3.BoolVal(True)
        if isinstance(formula, FalseFormula):
            return z3.BoolVal(False)
        if isinstance(formula, EqFormula):
            t1 = self.translate_term(formula.left, env)
            t2 = self.translate_term(formula.right, env)
            return t1 == t2
        if isinstance(formula, PredAppFormula):
            p = self.preds[formula.symbol.name]
            if len(formula.symbol.arity) == 0:
                return p
            args_z3 = [self.translate_term(a, env) for a in formula.args]
            return p(*args_z3)
        if isinstance(formula, NotFormula):
            return z3.Not(self.translate_formula(formula.child, env))
        if isinstance(formula, AndFormula):
            if len(formula.children) == 0:
                return z3.BoolVal(True)
            return z3.And(*[self.translate_formula(c, env) for c in formula.children])
        if isinstance(formula, OrFormula):
            if len(formula.children) == 0:
                return z3.BoolVal(False)
            return z3.Or(*[self.translate_formula(c, env) for c in formula.children])
        if isinstance(formula, ImpliesFormula):
            return z3.Implies(
                self.translate_formula(formula.left, env),
                self.translate_formula(formula.right, env),
            )
        if isinstance(formula, IffFormula):
            return self.translate_formula(formula.left, env) == self.translate_formula(
                formula.right, env
            )
        if isinstance(formula, (ForallFormula, ExistsFormula)):
            variable = z3.FreshConst(self.sort_map[formula.sort], prefix="bound")
            body = self.translate_formula(formula.child, [variable, *env])
            quantifier = z3.ForAll if isinstance(formula, ForallFormula) else z3.Exists
            return quantifier([variable], body)
        raise ValueError(f"Fórmula desconhecida: {formula}")

    def eval_formula(self, formula: FormulaTree) -> bool:
        phi_z3 = self.translate_formula(formula, env=[])
        res = self.z3_model.eval(phi_z3, model_completion=True)
        if z3.is_true(res):
            return True
        if z3.is_false(res):
            return False
        # Model.eval may leave native quantifiers unreduced. The complete finite
        # interpretation fixes truth uniquely; ask Z3 about its negation.
        self.solver.push()
        try:
            self.solver.add(z3.Not(phi_z3))
            outcome = self.solver.check()
            if outcome == z3.unsat:
                return True
            if outcome == z3.sat:
                return False
            raise RuntimeError(f"Z3 oracle unknown: {self.solver.reason_unknown()}")
        finally:
            self.solver.pop()


# ==============================================================================
# Geradores Aleatórios de Modelos e Fórmulas
# ==============================================================================


def generate_term(
    rng: random.Random, vocab: VocabularySpec, sort: str, binders: list[str], depth: int
) -> TermTree:
    matching_vars = [i for i, s in enumerate(binders) if s == sort]
    if matching_vars and (depth == 0 or rng.random() < 0.45):
        return VarTerm(rng.choice(matching_vars), sort)

    candidates = [
        s
        for s in vocab.symbols
        if s.kind in (0, 1) and s.result == sort and (depth > 0 or not s.arity)
    ]
    if not candidates:
        if matching_vars:
            return VarTerm(matching_vars[0], sort)
        raise RuntimeError(f"Sem símbolo ou variável para sort {sort}")

    sym = rng.choice(candidates)
    if sym.kind == 0 or len(sym.arity) == 0:
        return AppTerm(sym, [])

    sub_depth = depth - 1 if depth > 0 else 0
    args = [generate_term(rng, vocab, arg_s, binders, sub_depth) for arg_s in sym.arity]
    return AppTerm(sym, args)


def generate_formula(
    rng: random.Random, vocab: VocabularySpec, depth: int, binders: list[str]
) -> FormulaTree:
    if depth <= 0 or (depth <= 2 and rng.random() < 0.25):
        leaf_choice = rng.random()
        if leaf_choice < 0.12:
            return TrueFormula()
        if leaf_choice < 0.24:
            return FalseFormula()
        if leaf_choice < 0.60:
            target_sort = rng.choice(vocab.sorts).name
            t1 = generate_term(rng, vocab, target_sort, binders, depth=1)
            t2 = generate_term(rng, vocab, target_sort, binders, depth=1)
            return EqFormula(t1, t2)

        pred_candidates = [s for s in vocab.symbols if s.kind == 2]
        pred_sym = rng.choice(pred_candidates)
        args = [generate_term(rng, vocab, arg_s, binders, depth=1) for arg_s in pred_sym.arity]
        return PredAppFormula(pred_sym, args)

    op_choice = rng.randrange(7)
    if op_choice == 0:
        return NotFormula(generate_formula(rng, vocab, depth - 1, binders))
    if op_choice == 1:
        count = rng.randint(0, 3)
        return AndFormula([generate_formula(rng, vocab, depth - 1, binders) for _ in range(count)])
    if op_choice == 2:
        count = rng.randint(0, 3)
        return OrFormula([generate_formula(rng, vocab, depth - 1, binders) for _ in range(count)])
    if op_choice == 3:
        left = generate_formula(rng, vocab, depth - 1, binders)
        right = generate_formula(rng, vocab, depth - 1, binders)
        return ImpliesFormula(left, right)
    if op_choice == 4:
        left = generate_formula(rng, vocab, depth - 1, binders)
        right = generate_formula(rng, vocab, depth - 1, binders)
        return IffFormula(left, right)
    if op_choice == 5:
        sort_name = rng.choice(vocab.sorts).name
        child = generate_formula(rng, vocab, depth - 1, [sort_name, *binders])
        return ForallFormula(sort_name, child)

    sort_name = rng.choice(vocab.sorts).name
    child = generate_formula(rng, vocab, depth - 1, [sort_name, *binders])
    return ExistsFormula(sort_name, child)


def generate_model(rng: random.Random, model_idx: int) -> ModelInterpretation:
    sort_count = rng.randint(1, 3)
    sort_names = ["S", "T", "U"][:sort_count]
    sorts: list[SortSpec] = []
    symbols: list[SymbolSpec] = []
    const_values: dict[int, int] = {}
    func_tables: dict[int, list[int]] = {}
    pred_tables: dict[int, list[list[int]]] = {}

    for i, name in enumerate(sort_names):
        dsize = rng.randint(1, 4)
        # Sort Enumerated se tiver 2+ sorts e índice for múltiplo de 2
        if i == 1 and model_idx % 2 == 0:
            enum_const_names = [f"e_{j}" for j in range(dsize)]
            sorts.append(
                SortSpec(name=name, domain_size=dsize, is_enum=True, enum_consts=enum_const_names)
            )
            for j, cname in enumerate(enum_const_names):
                sym_id = len(symbols)
                symbols.append(SymbolSpec(name=cname, kind=0, arity=(), result=name))
                const_values[sym_id] = j  # Ordinal intrínseco obrigatório
        else:
            sorts.append(SortSpec(name=name, domain_size=dsize, is_enum=False))
            # Pelo menos uma constante para cada sort não enumerado
            sym_id = len(symbols)
            cname = f"c_{name}"
            symbols.append(SymbolSpec(name=cname, kind=0, arity=(), result=name))
            const_values[sym_id] = rng.randint(0, dsize - 1)

    # Funções primitivas densas
    for s in sorts:
        if rng.random() < 0.7:
            sym_id = len(symbols)
            fname = f"f_{s.name}"
            symbols.append(SymbolSpec(name=fname, kind=1, arity=(s.name,), result=s.name))
            func_tables[sym_id] = [rng.randint(0, s.domain_size - 1) for _ in range(s.domain_size)]

    if len(sorts) >= 2 and rng.random() < 0.6:
        s1, s2 = sorts[0], sorts[1]
        sym_id = len(symbols)
        fname = f"h_{s1.name}_{s2.name}"
        symbols.append(SymbolSpec(name=fname, kind=1, arity=(s1.name, s2.name), result=s1.name))
        table_len = s1.domain_size * s2.domain_size
        func_tables[sym_id] = [rng.randint(0, s1.domain_size - 1) for _ in range(table_len)]

    # Predicado nulário (aridade 0)
    if rng.random() < 0.5:
        sym_id = len(symbols)
        symbols.append(SymbolSpec(name="P0", kind=2, arity=(), result=None))
        pred_tables[sym_id] = [[]] if rng.random() < 0.5 else []

    # Predicados unários e binários
    for s in sorts:
        sym_id = len(symbols)
        pname = f"P_{s.name}"
        symbols.append(SymbolSpec(name=pname, kind=2, arity=(s.name,), result=None))
        tuples = [[i] for i in range(s.domain_size) if rng.random() < 0.45]
        pred_tables[sym_id] = tuples

    if len(sorts) >= 2:
        s1, s2 = sorts[0], sorts[1]
        sym_id = len(symbols)
        rname = f"R_{s1.name}_{s2.name}"
        symbols.append(SymbolSpec(name=rname, kind=2, arity=(s1.name, s2.name), result=None))
        tuples = [
            [i, j]
            for i in range(s1.domain_size)
            for j in range(s2.domain_size)
            if rng.random() < 0.40
        ]
        pred_tables[sym_id] = tuples

    # Símbolos Defined:
    # 1. Predicado Defined
    base_sort = sorts[0].name
    p_base = next(s for s in symbols if s.kind == 2 and s.arity == (base_sort,))
    def_pred_body = NotFormula(PredAppFormula(p_base, [VarTerm(0, base_sort)]))
    symbols.append(
        SymbolSpec(
            name="D_not_p",
            kind=2,
            arity=(base_sort,),
            result=None,
            is_defined=True,
            defined_body=def_pred_body,
        )
    )

    # 2. Constante Defined (relação unicamente determinada)
    c_base = next(s for s in symbols if s.kind == 0 and s.result == base_sort)
    def_const_body = EqFormula(VarTerm(0, base_sort), AppTerm(c_base, []))
    symbols.append(
        SymbolSpec(
            name="C_def",
            kind=0,
            arity=(),
            result=base_sort,
            is_defined=True,
            defined_body=def_const_body,
        )
    )

    # 3. Função Defined (relação unicamente determinada)
    f_base = next((s for s in symbols if s.kind == 1 and s.arity == (base_sort,)), None)
    if f_base is not None:
        def_func_body = EqFormula(VarTerm(0, base_sort), AppTerm(f_base, [VarTerm(1, base_sort)]))
        symbols.append(
            SymbolSpec(
                name="F_def",
                kind=1,
                arity=(base_sort,),
                result=base_sort,
                is_defined=True,
                defined_body=def_func_body,
            )
        )

    vocab = VocabularySpec(sorts=sorts, symbols=symbols)
    return ModelInterpretation(
        vocab=vocab,
        const_values=const_values,
        func_tables=func_tables,
        pred_tables=pred_tables,
    )


# ==============================================================================
# Casos Diferenciais
# ==============================================================================


@dataclass(frozen=True)
class DiffCase:
    vocab_bytes: bytes
    model_bytes: bytes
    formula_bytes: bytes
    expected_bool: bool


def generate_diff_cases(count: int, seed: int = 20261005) -> list[DiffCase]:
    rng = random.Random(seed)
    num_models = min(100, count)
    formulas_per_model = (count + num_models - 1) // num_models

    cases: list[DiffCase] = []
    for model_idx in range(num_models):
        model = generate_model(rng, model_idx)
        oracle = Z3Oracle(model)
        v_bytes = canon.encode(vocab_to_canon(model.vocab))
        m_bytes = canon.encode(model_to_canon(model))

        for _ in range(formulas_per_model):
            if len(cases) >= count:
                break
            depth = rng.randint(1, 3)
            formula = generate_formula(rng, model.vocab, depth=depth, binders=[])
            expected = oracle.eval_formula(formula)
            f_bytes = canon.encode(formula_to_canon(formula))
            cases.append(
                DiffCase(
                    vocab_bytes=v_bytes,
                    model_bytes=m_bytes,
                    formula_bytes=f_bytes,
                    expected_bool=expected,
                )
            )

    return cases


# ==============================================================================
# Testes Pytest
# ==============================================================================


def test_10k_model_eval_matches_z3_oracle() -> None:
    cases = generate_diff_cases(N, seed=20261005)
    assert len(cases) == N

    lines = [f"{c.vocab_bytes.hex()} {c.model_bytes.hex()} {c.formula_bytes.hex()}" for c in cases]
    stdin_payload = "\n".join(lines) + "\n"

    result = subprocess.run(
        CARGO,
        cwd=ROOT,
        input=stdin_payload,
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, f"eval_diff retornou erro:\n{result.stderr}\n{result.stdout}"

    rows = result.stdout.splitlines()
    assert len(rows) == N, f"esperado {N} linhas de resultado, recebido {len(rows)}"

    true_count = 0
    false_count = 0
    divergences = 0

    for i, (case, row) in enumerate(zip(cases, rows, strict=True)):
        if row == "true":
            rust_res = True
            true_count += 1
        elif row == "false":
            rust_res = False
            false_count += 1
        else:
            raise AssertionError(f"Caso {i} retornou erro no Rust: {row}")

        if rust_res != case.expected_bool:
            divergences += 1

    assert divergences == 0, f"Divergências detectadas entre Rust e Z3: {divergences}/{N}"
    print(f"MODEL_DIFFERENTIAL_OK pairs={N} true={true_count} false={false_count} divergences=0")
