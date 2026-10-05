"""Independent surface/AST corpus oracle for Etapa 1.4; no parser/printer calls."""

from __future__ import annotations

import json
import random
from dataclasses import dataclass

from logos_client import canon

LOGIC = "FOL-ms-eq/v1"


def quoted(name: str) -> str:
    return json.dumps(name, ensure_ascii=False)


def sort_value(name: str) -> canon.Value:
    return [1, 0, name]


@dataclass(frozen=True)
class Signature:
    name: str
    kind: int
    arity: tuple[str, ...]
    result: str | None

    def value(self) -> canon.Value:
        result: canon.Value = [1] if self.result is None else [0, sort_value(self.result)]
        return [1, 1, self.name, self.kind, [sort_value(s) for s in self.arity], result]

    def text(self) -> str:
        kind = ("const", "func", "pred")[self.kind]
        args = ",".join(quoted(s) for s in self.arity)
        result = "bool" if self.result is None else quoted(self.result)
        return f"symbol({quoted(self.name)},{kind},[{args}],{result})"


U0 = Signature("u0", 0, (), "U")
T0 = Signature("t0", 0, (), "T")
T1 = Signature("t1", 0, (), "T")
B0 = Signature("b0", 0, (), "bool")
H = Signature("h", 1, ("U", "T"), "T")
P = Signature("P", 2, ("U",), None)
G = Signature("G", 2, ("T",), None)
DERIVED = Signature("Derived", 2, ("U",), None)
ODD = Signature("odd", 0, ("U",), None)
CONSTANTS = (U0, T0, T1, B0)
SIGS = (U0, T0, T1, B0, H, P, G, ODD)
NAMES = ("", "bool", "😀", "not", 'line\nquote\\"', "é", "Ω")


def term_value(records: list[canon.Value]) -> canon.Value:
    return [1, 2, LOGIC, records]


def formula_value(records: list[canon.Value]) -> canon.Value:
    return [1, 3, LOGIC, records]


@dataclass(frozen=True)
class Tree:
    text: str
    records: list[canon.Value]


@dataclass(frozen=True)
class Case:
    source: str
    vocabulary: canon.Value
    formula: canon.Value


def vocabulary_value() -> canon.Value:
    definition = formula_value([[3, P.value(), [term_value([[0, 0, sort_value("U")]])]]])
    sorts: list[canon.Value] = [
        ["U", [0]],
        ["bool", [0]],
        ["T", [1, [T1.value(), T0.value()], [1, b"\x44" * 32]]],
    ]
    symbols: list[canon.Value] = [
        [DERIVED.value(), [0, definition], "derivado"],
        [U0.value(), [2, "usuário"], "primeiro"],
        [T0.value(), [2, ""], ""],
        [T1.value(), [2, ""], ""],
        [B0.value(), [2, "bool é sort"], "não é Bool"],
        [H.value(), [2, "função h"], 'linha\ncom aspas " e barra \\'],
        [P.value(), [1, "atestador Ω", "procedimento 😀"], "predicado P"],
        [G.value(), [2, ""], ""],
        [ODD.value(), [2, "assinatura sintática"], ""],
    ]
    return [1, 7, sorts, symbols]


def vocabulary_source() -> str:
    gloss = quoted('linha\ncom aspas " e barra \\')
    return f'''vocab v7 {{
  # Defined resolves forward P, with a free parameter, not a closed claim.
  sort U
  pred Derived(U) = defined(P(var(0,U))) gloss "derivado"
  sort "bool"
  sort T = enum [t1,t0] from policy "{"44" * 32}"
  const u0: U = glossed("usuário") gloss "primeiro"
  const t0: T
  const t1: T
  const b0: "bool" = glossed("bool é sort") gloss "não é Bool"
  func h(U,T): T = glossed("função h") gloss {gloss}
  pred P(U) = anchored("atestador Ω","procedimento 😀") gloss "predicado P"
  pred G(T)
  decl {ODD.text()} = glossed("assinatura sintática") gloss ""
}}'''


def _raw_signature(rng: random.Random) -> Signature:
    return Signature(
        rng.choice(NAMES),
        rng.randrange(3),
        tuple(rng.choice(NAMES) for _ in range(rng.randrange(4))),
        rng.choice((*NAMES, None)),
    )


def _term(rng: random.Random, depth: int, raw: bool, binders: tuple[tuple[str, str], ...]) -> Tree:
    if raw and (depth == 0 or rng.randrange(3) == 0):
        index = rng.choice((0, 1, 2, 2**64 - 1))
        sort = rng.choice(NAMES)
        return Tree(f"var({index},{quoted(sort)})", [[0, index, sort_value(sort)]])
    if not raw and binders and rng.randrange(3) == 0:
        name = rng.choice(binders)[0]
        index, sort = next((i, s) for i, (n, s) in enumerate(reversed(binders)) if n == name)
        return Tree(quoted(name), [[0, index, sort_value(sort)]])
    signature = _raw_signature(rng) if raw else rng.choice(CONSTANTS if depth == 0 else SIGS)
    count = 0 if depth == 0 else rng.randrange(4)
    children = [_term(rng, depth - 1, raw, binders) for _ in range(count)]
    records: list[canon.Value] = [[1, signature.value(), count]]
    for child in children:
        records.extend(child.records)
    args = ",".join(child.text for child in children)
    text = f"app({signature.text()},[{args}])" if raw else f"{signature.name}({args})"
    return Tree(text, records)


def _formula(
    rng: random.Random,
    depth: int,
    raw: bool,
    binders: tuple[tuple[str, str], ...] = (),
    tag: int | None = None,
) -> Tree:
    if tag is None:
        tag = rng.randrange(11 if depth else 4)
    if tag in (0, 1):
        return Tree("true" if tag == 0 else "false", [[tag]])
    if tag == 2:
        left = _term(rng, 2, raw, binders)
        right = _term(rng, 2, raw, binders)
        text = f"eq({left.text},{right.text})" if raw else f"({left.text} = {right.text})"
        return Tree(text, [[2, term_value(left.records), term_value(right.records)]])
    if tag == 3:
        signature = _raw_signature(rng) if raw else rng.choice(SIGS)
        terms = [_term(rng, 2, raw, binders) for _ in range(rng.randrange(4))]
        args = ",".join(term.text for term in terms)
        text = f"pred({signature.text()},[{args}])" if raw else f"{signature.name}({args})"
        return Tree(text, [[3, signature.value(), [term_value(term.records) for term in terms]]])
    if tag == 4:
        child = _formula(rng, depth - 1, raw, binders)
        return Tree(f"not({child.text})", [[4], *child.records])
    if tag in (5, 6):
        count = rng.randrange(4)
        children = [_formula(rng, depth - 1, raw, binders) for _ in range(count)]
        records: list[canon.Value] = [[tag, count]]
        for child in children:
            records.extend(child.records)
        name = "and" if tag == 5 else "or"
        # Keep n-arity/zero explicit; for 2 children exercise human infix too.
        if not raw and count == 2:
            text = f"({children[0].text} {name} {children[1].text})"
        else:
            text = f"{name}([{','.join(child.text for child in children)}])"
        return Tree(text, records)
    if tag in (7, 8):
        left = _formula(rng, depth - 1, raw, binders)
        right = _formula(rng, depth - 1, raw, binders)
        if raw:
            text = f"{'implies' if tag == 7 else 'iff'}({left.text},{right.text})"
        else:
            text = f"({left.text} {'->' if tag == 7 else '<->'} {right.text})"
        return Tree(text, [[tag], *left.records, *right.records])
    name = rng.choice(("x", "y", "true", ""))
    sort = rng.choice(NAMES) if raw else rng.choice(("U", "T", "bool"))
    child = _formula(rng, depth - 1, raw, (*binders, (name, sort)))
    keyword = "forall" if tag == 9 else "exists"
    return Tree(
        f"({keyword} {quoted(name)}:{quoted(sort)}. {child.text})",
        [[tag, sort_value(sort)], *child.records],
    )


def corpus(count: int = 500, seed: int = 20261005) -> list[Case]:
    rng = random.Random(seed)
    cases = []
    for index in range(count):
        tree = _formula(rng, 4, raw=index % 2 == 1, tag=index % 11)
        marker = term_value([[0, index, sort_value("U")]])
        records: list[canon.Value] = [[5, 2], *tree.records, [2, marker, marker]]
        text = f"and([{tree.text},eq(var({index},U),var({index},U))])"
        source = f"{vocabulary_source()}\nclaim C{index}: {text} cites [F3,P2,C5,F3]"
        cases.append(Case(source, vocabulary_value(), formula_value(records)))
    return cases
