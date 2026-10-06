"""Busca exaustiva não confiável em domínios ≤4; candidatos exigem checker Rust."""

from __future__ import annotations

import time
from collections import deque
from collections.abc import Iterator
from dataclasses import dataclass
from enum import StrEnum

from logos_client import canon
from logos_provers import model_context as context


@dataclass(frozen=True)
class Limits:
    max_domain: int = 4
    max_steps: int = 1_000_000
    timeout_ms: int = 5000

    def __post_init__(self) -> None:
        for name, maximum in (
            ("max_domain", 4),
            ("max_steps", context.MAX_UINT64),
            ("timeout_ms", context.MAX_UINT32),
        ):
            value = getattr(self, name)
            if type(value) is not int or not 1 <= value <= maximum:
                raise context.ProducerError(f"{name} must be an integer in 1..={maximum}")


class SearchStatus(StrEnum):
    FOUND = "found"
    NO_MODEL_WITHIN_BOUND = "no_model_within_bound"
    EXHAUSTED = "exhausted"


@dataclass(frozen=True)
class SearchResult:
    status: SearchStatus
    model: bytes | None = None


_DEFAULT_LIMITS = Limits()


class _Exhausted(Exception):
    """Internal resource interruption; never a semantic absence conclusion."""


@dataclass
class _Budget:
    remaining: int
    deadline: float

    def tick(self, amount: int = 1) -> None:
        if self.remaining < amount or time.monotonic() >= self.deadline:
            raise _Exhausted
        self.remaining -= amount


def _coordinates(sizes: tuple[int, ...], budget: _Budget) -> Iterator[tuple[int, ...]]:
    """Zero-based lazy odometer, last axis fastest; empty product has one tuple."""
    budget.tick(len(sizes) + 1)
    values = [0] * len(sizes)
    while True:
        budget.tick()
        yield tuple(values)
        for index in reversed(range(len(sizes))):
            budget.tick()
            if values[index] + 1 < sizes[index]:
                values[index] += 1
                break
            values[index] = 0
        else:
            return


@dataclass(frozen=True)
class _Shape:
    sizes: tuple[int, ...]
    cells: int
    radix: int
    pinned: int | None


def _layout(
    vocabulary: context.ValidatedVocabulary, domains: tuple[int, ...], budget: _Budget
) -> tuple[list[_Shape], list[list[int]], list[tuple[int, int, int]]]:
    shapes: list[_Shape] = []
    tables: list[list[int]] = []
    axes: list[tuple[int, int, int]] = []
    cells_total = 0
    for symbol in vocabulary.symbols:
        budget.tick(len(symbol.arity) + 1)
        sizes = tuple(domains[vocabulary.sort_by_name[name].index] for name in symbol.arity)
        cells = 1
        for size in sizes:
            cells *= size
            if cells > context.MAX_MODEL_CELLS:
                raise _Exhausted
        cells_total += cells
        if cells_total > context.MAX_MODEL_CELLS:
            raise _Exhausted
        radix = 2
        pinned = None
        if symbol.kind != 2:
            assert symbol.result is not None
            sort = vocabulary.sort_by_name[symbol.result]
            radix = domains[sort.index]
            if symbol.kind == 0 and symbol.name in sort.enum_constants:
                pinned = sort.enum_constants.index(symbol.name)
        shapes.append(_Shape(sizes, cells, radix, pinned))
        if symbol.grounding_kind == 0:
            tables.append([])
            continue
        budget.tick(cells)
        tables.append([0 if pinned is None else pinned] * cells)
        if pinned is None and radix > 1:
            budget.tick(cells)
            axes.extend((symbol.index, cell, radix) for cell in range(cells))
    return shapes, tables, axes


def _definition_order(vocabulary: context.ValidatedVocabulary, budget: _Budget) -> list[int]:
    """Iterative dependency ordering: forward definitions never read stale tables."""
    users: dict[int, list[int]] = {index: [] for index in vocabulary.definition_order}
    remaining: dict[int, int] = {}
    for index in vocabulary.definition_order:
        symbol = vocabulary.symbols[index]
        assert symbol.defined_body is not None
        pending: list[context.FormulaAST | context.TermAST] = [symbol.defined_body]
        dependencies: set[int] = set()
        while pending:
            budget.tick()
            node = pending.pop()
            if isinstance(node, (context.ASTApp, context.ASTPred)):
                referenced_symbol = vocabulary.symbol_by_name[node.symbol_name]
                if referenced_symbol.grounding_kind == 0:
                    dependencies.add(referenced_symbol.index)
                pending.extend(node.args)
            elif isinstance(node, context.ASTEq):
                pending.extend((node.left, node.right))
            elif isinstance(node, (context.ASTAnd, context.ASTOr)):
                pending.extend(node.children)
            elif isinstance(node, (context.ASTNot, context.ASTForall, context.ASTExists)):
                pending.append(node.child)
            elif isinstance(node, (context.ASTImplies, context.ASTIff)):
                pending.extend((node.left, node.right))
        remaining[index] = len(dependencies)
        for dependency in dependencies:
            users[dependency].append(index)
    ready = deque(index for index, count in remaining.items() if count == 0)
    order: list[int] = []
    while ready:
        budget.tick()
        index = ready.popleft()
        order.append(index)
        for user in users[index]:
            budget.tick()
            remaining[user] -= 1
            if remaining[user] == 0:
                ready.append(user)
    if len(order) != len(remaining):
        raise context.ProducerError("cyclic Defined dependencies")
    return order


@dataclass
class _Interpreter:
    vocabulary: context.ValidatedVocabulary
    domains: tuple[int, ...]
    shapes: list[_Shape]
    tables: list[list[int]]
    budget: _Budget

    def value(self, name: str, arguments: list[int]) -> int:
        symbol = self.vocabulary.symbol_by_name[name]
        index = 0
        for size, argument in zip(self.shapes[symbol.index].sizes, arguments, strict=True):
            self.budget.tick()
            index = index * size + argument
        return self.tables[symbol.index][index]

    def term(self, node: context.TermAST, environment: list[int]) -> int:
        self.budget.tick()
        if isinstance(node, context.ASTVar):
            return environment[-1 - node.index]
        if isinstance(node, context.ASTApp):
            return self.value(node.symbol_name, [self.term(arg, environment) for arg in node.args])
        raise context.ProducerError(f"unexpected term AST {type(node)}")

    def formula(self, node: context.FormulaAST, environment: list[int]) -> bool:
        self.budget.tick()
        if isinstance(node, context.ASTTrue):
            return True
        if isinstance(node, context.ASTFalse):
            return False
        if isinstance(node, context.ASTEq):
            return self.term(node.left, environment) == self.term(node.right, environment)
        if isinstance(node, context.ASTPred):
            return bool(
                self.value(node.symbol_name, [self.term(arg, environment) for arg in node.args])
            )
        if isinstance(node, context.ASTNot):
            return not self.formula(node.child, environment)
        if isinstance(node, context.ASTAnd):
            return all(self.formula(child, environment) for child in node.children)
        if isinstance(node, context.ASTOr):
            return any(self.formula(child, environment) for child in node.children)
        if isinstance(node, context.ASTImplies):
            return not self.formula(node.left, environment) or self.formula(node.right, environment)
        if isinstance(node, context.ASTIff):
            return self.formula(node.left, environment) == self.formula(node.right, environment)
        if isinstance(node, (context.ASTForall, context.ASTExists)):
            universal = isinstance(node, context.ASTForall)
            size = self.domains[self.vocabulary.sort_by_name[node.sort].index]
            for value in range(size):
                self.budget.tick()
                environment.append(value)
                try:
                    outcome = self.formula(node.child, environment)
                finally:
                    environment.pop()
                if outcome != universal:
                    return outcome
            return universal
        raise context.ProducerError(f"unexpected formula AST {type(node)}")

    def derive(self, order: list[int]) -> bool:
        for index in order:
            self.budget.tick()
            symbol = self.vocabulary.symbols[index]
            shape = self.shapes[index]
            assert symbol.defined_body is not None
            outputs: list[int] = []
            for coordinates in _coordinates(shape.sizes, self.budget):
                environment = list(reversed(coordinates))
                if symbol.kind == 2:
                    outputs.append(int(self.formula(symbol.defined_body, environment)))
                    continue
                chosen = None
                for result in range(shape.radix):
                    self.budget.tick()
                    environment.append(result)
                    try:
                        holds = self.formula(symbol.defined_body, environment)
                    finally:
                        environment.pop()
                    if holds:
                        if chosen is not None:
                            return False
                        chosen = result
                if chosen is None or (shape.pinned is not None and chosen != shape.pinned):
                    return False
                outputs.append(chosen)
            self.tables[index] = outputs
        return True

    def wire(self) -> bytes:
        consts: list[canon.Value] = []
        funcs: list[canon.Value] = []
        preds: list[canon.Value] = []
        cells = 0
        for symbol in self.vocabulary.symbols:
            self.budget.tick()
            if symbol.grounding_kind == 0:
                continue
            outputs = self.tables[symbol.index]
            if symbol.kind == 0:
                cells += 1
                consts.append([symbol.index, outputs[0]])
            elif symbol.kind == 1:
                cells += len(outputs)
                self.budget.tick(len(outputs))
                funcs.append(
                    [symbol.index, b"".join(value.to_bytes(4, "big") for value in outputs)]
                )
            else:
                tuples: list[canon.Value] = []
                for coordinates, holds in zip(
                    _coordinates(self.shapes[symbol.index].sizes, self.budget), outputs, strict=True
                ):
                    if holds:
                        cells += max(1, len(coordinates))
                        if cells > context.MAX_MODEL_CELLS:
                            raise _Exhausted
                        tuples.append(list(coordinates))
                preds.append([symbol.index, tuples])
            if cells > context.MAX_MODEL_CELLS:
                raise _Exhausted
        self.budget.tick()
        try:
            wire = canon.encode([1, self.vocabulary.h_v, list(self.domains), consts, funcs, preds])
        except canon.CanonError as error:
            raise context.ProducerError(f"model encoding failed: {error}") from error
        if len(wire) > context.MAX_MODEL_BYTES:
            raise _Exhausted
        self.budget.tick()
        return wire


def _advance(tables: list[list[int]], axes: list[tuple[int, int, int]], budget: _Budget) -> bool:
    for symbol, cell, radix in reversed(axes):
        budget.tick()
        if tables[symbol][cell] + 1 < radix:
            tables[symbol][cell] += 1
            return True
        tables[symbol][cell] = 0
    return False


def refute(
    obligation: bytes, vocabulary: bytes, *, limits: Limits = _DEFAULT_LIMITS
) -> SearchResult:
    """Enumerate Γ ∧ ¬goal; absence within the bound is not a trusted validity proof."""
    if not isinstance(limits, Limits):
        raise context.ProducerError("limits must be a model_finder.Limits instance")
    budget = _Budget(limits.max_steps, time.monotonic() + limits.timeout_ms / 1000)
    vocab = context._validate_vocabulary(vocabulary)
    oblg = context._validate_obligation(obligation, vocab)
    try:
        budget.tick()
        sort_fixed: list[int | None] = []
        for sort in vocab.sorts:
            budget.tick()
            cardinality = len(sort.enum_constants) if sort.is_enum else None
            if cardinality is not None and (cardinality == 0 or cardinality > limits.max_domain):
                return SearchResult(SearchStatus.NO_MODEL_WITHIN_BOUND)
            sort_fixed.append(cardinality)
        order = _definition_order(vocab, budget)
        for domains in context._domain_vectors(sort_fixed, limits.max_domain):
            budget.tick()
            shapes, tables, axes = _layout(vocab, domains, budget)
            interpreter = _Interpreter(vocab, domains, shapes, tables, budget)
            while True:
                budget.tick()
                if (
                    interpreter.derive(order)
                    and all(interpreter.formula(premise, []) for premise in oblg.premises)
                    and not interpreter.formula(oblg.goal, [])
                ):
                    return SearchResult(SearchStatus.FOUND, interpreter.wire())
                if not _advance(tables, axes, budget):
                    break
        budget.tick()
        return SearchResult(SearchStatus.NO_MODEL_WITHIN_BOUND)
    except _Exhausted:
        return SearchResult(SearchStatus.EXHAUSTED)
