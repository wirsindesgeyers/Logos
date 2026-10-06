"""Apresentação PT não autoritativa de model/v1; não substitui checker Rust."""

from __future__ import annotations

import hashlib
import json
import time
from typing import Any

from logos_client import air, canon
from logos_provers import model_context as context
from logos_provers import model_finder as finder

_MAX_FACTS = 100


def _uint(value: Any, maximum: int) -> int:
    if type(value) is not int or not 0 <= value <= maximum:
        raise context.ProducerError("invalid model integer")
    return int(value)


def _interpreter(model: bytes, vocabulary: context.ValidatedVocabulary) -> finder._Interpreter:
    if not isinstance(model, bytes) or len(model) > context.MAX_MODEL_BYTES:
        raise context.ProducerError("model must be bytes within model/v1 size limit")
    wire: Any = canon.decode(model)
    if not (
        isinstance(wire, list)
        and len(wire) == 6
        and type(wire[0]) is int
        and wire[0] == 1
        and wire[1] == vocabulary.h_v
        and all(isinstance(wire[index], list) for index in range(2, 6))
    ):
        raise context.ProducerError("invalid model envelope or vocabulary digest")
    if len(wire[2]) != len(vocabulary.sorts):
        raise context.ProducerError("invalid model domains")
    domains = tuple(_uint(size, 65536) for size in wire[2])
    for sort, size in zip(vocabulary.sorts, domains, strict=True):
        if size == 0 or (sort.is_enum and size != len(sort.enum_constants)):
            raise context.ProducerError("invalid model domain cardinality")

    budget = finder._Budget(1_000_000, time.monotonic() + 5)
    shapes: list[finder._Shape] = []
    tables: list[list[int]] = []
    total_cells = 0
    for symbol in vocabulary.symbols:
        sizes = tuple(domains[vocabulary.sort_by_name[name].index] for name in symbol.arity)
        cells = 1
        for size in sizes:
            cells *= size
            if cells > context.MAX_MODEL_CELLS:
                raise context.ProducerError("presentation table exceeds cell limit")
        budget.tick(cells)
        radix = 2
        pinned = None
        if symbol.result is not None:
            sort = vocabulary.sort_by_name[symbol.result]
            radix = domains[sort.index]
            if symbol.kind == 0 and symbol.name in sort.enum_constants:
                pinned = sort.enum_constants.index(symbol.name)
        shapes.append(finder._Shape(sizes, cells, radix, pinned))
        total_cells += cells
        if total_cells > context.MAX_MODEL_CELLS:
            raise context.ProducerError("presentation tables exceed aggregate cell limit")
        tables.append([])

    for kind, entries in enumerate(wire[3:]):
        symbols = [s for s in vocabulary.symbols if s.kind == kind and s.grounding_kind != 0]
        if len(entries) != len(symbols):
            raise context.ProducerError("missing or extra model interpretation")
        for symbol, entry in zip(symbols, entries, strict=True):
            if not isinstance(entry, list) or len(entry) != 2:
                raise context.ProducerError("invalid model interpretation")
            if _uint(entry[0], context.MAX_UINT32) != symbol.index:
                raise context.ProducerError("model interpretation ID/category/order mismatch")
            shape = shapes[symbol.index]
            if kind == 0:
                values = [_uint(entry[1], shape.radix - 1)]
                if shape.pinned is not None and values[0] != shape.pinned:
                    raise context.ProducerError("invalid enum constant ordinal")
            elif kind == 1:
                packed = entry[1]
                if not isinstance(packed, bytes) or len(packed) != shape.cells * 4:
                    raise context.ProducerError("non-total model function")
                values = [
                    int.from_bytes(packed[i : i + 4], "big") for i in range(0, len(packed), 4)
                ]
                if any(value >= shape.radix for value in values):
                    raise context.ProducerError("model function value outside domain")
            else:
                if not isinstance(entry[1], list):
                    raise context.ProducerError("invalid model predicate")
                values = [0] * shape.cells
                previous: tuple[int, ...] | None = None
                for row in entry[1]:
                    if not isinstance(row, list) or len(row) != len(shape.sizes):
                        raise context.ProducerError("invalid model predicate tuple")
                    coordinates = tuple(
                        _uint(value, size - 1) for value, size in zip(row, shape.sizes, strict=True)
                    )
                    if previous is not None and coordinates <= previous:
                        raise context.ProducerError("model predicate tuples out of order")
                    previous = coordinates
                    index = 0
                    for size, value in zip(shape.sizes, coordinates, strict=True):
                        index = index * size + value
                    values[index] = 1
            tables[symbol.index] = values

    interpreter = finder._Interpreter(vocabulary, domains, shapes, tables, budget)
    if not interpreter.derive(finder._definition_order(vocabulary, budget)):
        raise context.ProducerError("Defined interpretation is not total and unique")
    for symbol, shape in zip(vocabulary.symbols, shapes, strict=True):
        if shape.pinned is not None and tables[symbol.index] != [shape.pinned]:
            raise context.ProducerError("Defined enum ordinal mismatch")
    return interpreter


def _references(obligation: context.ValidatedObligation) -> tuple[set[str], set[str]]:
    """Only syntactic O; never follow Defined bodies or enum representatives."""
    symbols: set[str] = set()
    sorts: set[str] = set()
    pending: list[context.FormulaAST | context.TermAST] = [*obligation.premises, obligation.goal]
    while pending:
        node = pending.pop()
        if isinstance(node, (context.ASTApp, context.ASTPred)):
            symbols.add(node.symbol_name)
            sorts.update(node.symbol_arity)
            if isinstance(node, context.ASTApp):
                sorts.add(node.symbol_result)
            pending.extend(node.args)
        elif isinstance(node, context.ASTVar):
            sorts.add(node.sort)
        elif isinstance(node, context.ASTEq):
            pending.extend((node.left, node.right))
        elif isinstance(node, (context.ASTAnd, context.ASTOr)):
            pending.extend(node.children)
        elif isinstance(node, (context.ASTNot, context.ASTForall, context.ASTExists)):
            if isinstance(node, (context.ASTForall, context.ASTExists)):
                sorts.add(node.sort)
            pending.append(node.child)
        elif isinstance(node, (context.ASTImplies, context.ASTIff)):
            pending.extend((node.left, node.right))
    return symbols, sorts


def explain_model(model: bytes, vocabulary: bytes, obligation: bytes) -> str:
    """Describe M/V/O, not acceptance. False premises and true goals stay visible.

    Inputs are contextual canonical bytes. Invalid structures or cooperative
    evaluation exhaustion raise ProducerError, never an invented explanation.
    """
    try:
        vocab = context._validate_vocabulary(vocabulary)
        checked_o = context._validate_obligation(obligation, vocab)
        interpreter = _interpreter(model, vocab)
        symbols, sorts = _references(checked_o)
        raw_v: Any = canon.decode(vocabulary)
        raw_o: Any = canon.decode(obligation)
        lines = [
            "Apresentação não autoritativa; não substitui o checker nem garante fidelidade.",
            "h(M): " + hashlib.sha256(b"logos/model/v1\x00" + model).hexdigest(),
            "Domínios relevantes (elementos por ordinal, sem nomes de representantes):",
        ]
        for sort in vocab.sorts:
            if sort.name in sorts:
                size = interpreter.domains[sort.index]
                lines.append(
                    f"- {sort.name}: {sort.name}#0 até {sort.name}#{size - 1} ({size} elementos)"
                )
        if not sorts:
            lines.append("- Nenhum domínio ocorre na obrigação.")
        lines.append("Valores e fatos verdadeiros dos símbolos da obrigação:")
        displayed = 0
        omitted = False
        for symbol in vocab.symbols:
            if symbol.name not in symbols:
                continue
            gloss = raw_v[3][symbol.index][2]
            label = symbol.name + (f" — {json.dumps(gloss, ensure_ascii=False)}" if gloss else "")
            shape = interpreter.shapes[symbol.index]
            table = interpreter.tables[symbol.index]
            any_fact = False
            for index, coordinates in enumerate(
                finder._coordinates(shape.sizes, interpreter.budget)
            ):
                value = table[index]
                if symbol.kind == 2 and not value:
                    continue
                any_fact = True
                if displayed == _MAX_FACTS:
                    omitted = True
                    break
                arguments = ", ".join(
                    f"{sort}#{ordinal}"
                    for sort, ordinal in zip(symbol.arity, coordinates, strict=True)
                )
                application = label if symbol.kind == 0 else f"{label} ({arguments})"
                if symbol.kind == 2:
                    lines.append(f"- {application}: verdadeiro")
                else:
                    lines.append(f"- {application} = {symbol.result}#{value}")
                displayed += 1
            if symbol.kind == 2 and not any_fact:
                if displayed == _MAX_FACTS:
                    omitted = True
                else:
                    lines.append(f"- {label}: nenhuma tupla verdadeira")
                    displayed += 1
        if omitted:
            lines.append("- Apresentação limitada a 100 entradas; há valores/fatos omitidos.")
        if not symbols:
            lines.append("- Nenhum símbolo ocorre na obrigação.")
        for index, premise in enumerate(checked_o.premises):
            truth = "verdadeira" if interpreter.formula(premise, []) else "falsa"
            lines.append(f"Premissa {index + 1}: {truth} — {air.print_formula(raw_o[5][index][1])}")
        goal_true = interpreter.formula(checked_o.goal, [])
        label = "Objetivo verdadeiro" if goal_true else "Objetivo falsificado"
        lines.append(f"{label}: {air.print_formula(raw_o[6])}")
        return "\n".join(lines)
    except (canon.CanonError, air.SurfaceError, RecursionError) as error:
        raise context.ProducerError(f"invalid presentation input: {error}") from error
    except finder._Exhausted as error:
        raise context.ProducerError("presentation evaluation budget exhausted") from error
