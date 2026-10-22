#!/usr/bin/env python3
"""Vetores paraphrase-pt/v1 com renderizador independente do runtime LOGOS.

Reimplementa a gramática de spec/paraphrase-pt.md sobre o fio air/v1 cru
(listas Python), com o CBOR mínimo de gen_vocabulary_vectors e hashlib.
Sem argumentos grava o TSV; com --check compara o arquivo inteiro.
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path

from gen_vocabulary_vectors import (
    WireArray,
    bool_result,
    encode,
    formula,
    sort,
    sort_result,
    symbol,
    term,
    term_app,
    term_var,
)

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "spec/vectors/paraphrase/vectors.tsv"
VOCAB_TAG = b"logos/vocab/v1\0"
PARAPHRASE_TAG = b"logos/paraphrase/v1\0"
GRAMMAR_ID = "paraphrase-pt/v1"
RENDERER_VERSION = "logos-seal/render_pt/1"

CONST, FUNC, PRED = 0, 1, 2


def vocabulary(sorts: list[str], decls: list[tuple[WireArray, str]]) -> WireArray:
    """vocabulary/v2 com domínios abertos e grounding Glossed."""
    return [
        2,
        1,
        [[name, [0]] for name in sorts],
        [[sig, [2, "glosa"], "glosa"] for sig, _ in decls],
        [template for _, template in decls],
    ]


def segments(template: str) -> list[str]:
    out: list[str] = []
    rest = template
    while "{" in rest:
        open_at = rest.index("{")
        out.append(rest[:open_at])
        rest = rest[rest.index("}", open_at) + 1 :]
    out.append(rest)
    return out


class Renderer:
    """Renderizador paraphrase-pt/v1 escrito a partir da spec."""

    def __init__(self, vocab: WireArray) -> None:
        pairs = zip(vocab[3], vocab[4], strict=True)
        self.templates = {decl[0][2]: segments(template) for decl, template in pairs}
        self.out: list[str] = []

    def formula(self, nodes: list[WireArray], at: int, binders: int) -> int:
        node = nodes[at]
        at += 1
        tag = node[0]
        if tag == 0:
            self.out.append("verdadeiro")
        elif tag == 1:
            self.out.append("falso")
        elif tag == 2:
            self.out.append("o valor ")
            self.slot(node[1], binders)
            self.out.append(" é igual a ")
            self.slot(node[2], binders)
        elif tag == 3:
            parts = self.templates[node[1][2]]
            self.out.append("⟦")
            for index, part in enumerate(parts):
                self.out.append(part)
                if index < len(node[2]):
                    self.slot(node[2][index], binders)
            self.out.append("⟧")
        elif tag == 4:
            self.out.append("não é o caso que (")
            at = self.group(nodes, at, binders)
        elif tag in (5, 6):
            count = node[1]
            self.out.append("vale cada uma de " if tag == 5 else "vale pelo menos uma de ")
            self.out.append(f"{count} {'condição' if count == 1 else 'condições'}")
            for index in range(count):
                self.out.append(": (" if index == 0 else "; (")
                at = self.group(nodes, at, binders)
        elif tag in (7, 8):
            self.out.append("se (" if tag == 7 else "(")
            at = self.formula(nodes, at, binders)
            self.out.append("), então (" if tag == 7 else ") se e somente se (")
            at = self.group(nodes, at, binders)
        elif tag in (9, 10):
            self.out.append("para todo " if tag == 9 else "existe ")
            self.out.append(f"x{binders + 1} do tipo ⟦{node[1][2]}")
            self.out.append("⟧, vale que (" if tag == 9 else "⟧ tal que (")
            at = self.group(nodes, at, binders + 1)
        else:
            raise ValueError(f"tag de fórmula desconhecida {tag}")
        return at

    def group(self, nodes: list[WireArray], at: int, binders: int) -> int:
        at = self.formula(nodes, at, binders)
        self.out.append(")")
        return at

    def slot(self, term_wire: WireArray, binders: int) -> None:
        self.out.append("⟦")
        if self.term(term_wire[3], 0, binders) != len(term_wire[3]):
            raise ValueError("termo com nós sobrando")
        self.out.append("⟧")

    def term(self, nodes: list[WireArray], at: int, binders: int) -> int:
        node = nodes[at]
        at += 1
        if node[0] == 0:
            self.out.append(f"x{binders - node[1]}")
            return at
        parts = self.templates[node[1][2]]
        for index, part in enumerate(parts):
            self.out.append(part)
            if index < node[2]:
                self.out.append("⟦")
                at = self.term(nodes, at, binders)
                self.out.append("⟧")
        return at


def render(vocab: WireArray, formula_wire: WireArray) -> str:
    renderer = Renderer(vocab)
    if renderer.formula(formula_wire[3], 0, 0) != len(formula_wire[3]):
        raise ValueError("fórmula com nós sobrando")
    return "".join(renderer.out)


def paraphrase_hash(vocab: WireArray, text: str) -> str:
    h_v = hashlib.sha256(VOCAB_TAG + encode(vocab)).digest()
    record = [1, GRAMMAR_ID, RENDERER_VERSION, h_v, text]
    return hashlib.sha256(PARAPHRASE_TAG + encode(record)).hexdigest()


U, D, A = sort("Usuário"), sort("Documento"), sort("Ação")
OWNS = symbol("Owns", PRED, [U, D], bool_result())
READ = symbol("CanRead", PRED, [U, D], bool_result())
ALICE = symbol("alice", CONST, [], sort_result("Usuário"))
AUTHOR = symbol("autor", FUNC, [D], sort_result("Usuário"))
BOSS = symbol("chefe", FUNC, [U], sort_result("Usuário"))
POLICY = vocabulary(
    ["Usuário", "Documento"],
    [
        (OWNS, "{0} é dono de {1}"),
        (READ, "{0} pode ler {1}"),
        (ALICE, "Alice"),
        (AUTHOR, "o autor de {0}"),
        (BOSS, "o chefe de {0}"),
    ],
)
IF_PRED = symbol("Se", PRED, [U, U], bool_result())
TRUE_PRED = symbol("Chove", PRED, [], bool_result())
RESP = symbol("Resp", PRED, [U, A], bool_result())
ACT = symbol("auditar", CONST, [], sort_result("Ação"))
ANA = symbol("ana", CONST, [], sort_result("Usuário"))
TRICKY = vocabulary(
    ["Usuário", "Ação"],
    [
        (IF_PRED, "se {0}, então {1}"),
        (TRUE_PRED, "verdadeiro"),
        (RESP, "{0} é responsável pela ação {1} (sempre)"),
        (ACT, "auditar; (verdadeiro)"),
        (ANA, "Ana"),
    ],
)

TRUE, FALSE, NOT, IMPLIES, IFF = [0], [1], [4], [7], [8]


def pred(sig: WireArray, *args: WireArray) -> WireArray:
    return [3, sig, list(args)]


def var(index: int, sort_wire: WireArray) -> WireArray:
    return term(term_var(index, sort_wire))


def const(sig: WireArray) -> WireArray:
    return term(term_app(sig, 0))


POSITIVE: list[tuple[str, WireArray, WireArray]] = [
    ("true", POLICY, formula(TRUE)),
    ("false", POLICY, formula(FALSE)),
    ("and-empty", POLICY, formula([5, 0])),
    ("or-empty", POLICY, formula([6, 0])),
    ("and-one", POLICY, formula([5, 1], TRUE)),
    ("or-two", POLICY, formula([6, 2], TRUE, FALSE)),
    ("and-ten", POLICY, formula([5, 10], *[TRUE] * 10)),
    ("not", POLICY, formula(NOT, FALSE)),
    ("iff", POLICY, formula(IFF, TRUE, NOT, FALSE)),
    ("implies-nested", POLICY, formula(IMPLIES, IMPLIES, TRUE, FALSE, TRUE)),
    (
        "owner-policy",
        POLICY,
        formula(
            [9, U],
            [9, D],
            IMPLIES,
            pred(OWNS, var(1, U), var(0, D)),
            pred(READ, var(1, U), var(0, D)),
        ),
    ),
    (
        "exists-eq-function",
        POLICY,
        formula([10, D], [2, term(term_app(AUTHOR, 1), term_var(0, D)), const(ALICE)]),
    ),
    (
        "nested-functions",
        POLICY,
        formula(
            [9, D],
            [2, term(term_app(BOSS, 1), term_app(AUTHOR, 1), term_var(0, D)), const(ALICE)],
        ),
    ),
    (
        "three-binders-same-sort",
        POLICY,
        formula(
            [9, U],
            [10, U],
            [9, U],
            [6, 2],
            [2, var(0, U), var(2, U)],
            [2, var(1, U), const(ALICE)],
        ),
    ),
    ("tricky-if-template", TRICKY, formula([9, U], pred(IF_PRED, var(0, U), const(ANA)))),
    ("tricky-true-template", TRICKY, formula([6, 2], TRUE, pred(TRUE_PRED))),
    ("tricky-accent-and-parens", TRICKY, formula([10, U], pred(RESP, var(0, U), const(ACT)))),
]

NEGATIVE: list[tuple[str, WireArray, str, str]] = [
    ("trailing-space", POLICY, "verdadeiro ", "Syntax"),
    ("capitalized", POLICY, "Verdadeiro", "Syntax"),
    ("leading-zero", POLICY, "vale cada uma de 01 condição: (verdadeiro)", "Syntax"),
    ("wrong-plural", POLICY, "vale cada uma de 1 condições: (verdadeiro)", "Syntax"),
    ("wrong-count", POLICY, "vale cada uma de 2 condições: (verdadeiro)", "Syntax"),
    ("wrong-ordinal", POLICY, "para todo x2 do tipo ⟦Usuário⟧, vale que (verdadeiro)", "Syntax"),
    ("unclosed", POLICY, "⟦Alice", "Syntax"),
    ("bare-group", POLICY, "(verdadeiro)", "Syntax"),
    ("unknown-template", POLICY, "⟦Alice é chefe⟧", "UnknownTemplate"),
    ("unknown-sort", POLICY, "existe x1 do tipo ⟦Pessoa⟧ tal que (verdadeiro)", "UnknownSort"),
    (
        "unbound-variable",
        POLICY,
        "existe x1 do tipo ⟦Usuário⟧ tal que (⟦⟦x2⟧ é dono de ⟦x1⟧⟧)",
        "UnboundVariable",
    ),
    (
        "ill-sorted",
        POLICY,
        "existe x1 do tipo ⟦Usuário⟧ tal que (⟦⟦x1⟧ é dono de ⟦x1⟧⟧)",
        "Sort",
    ),
]


def main() -> None:
    rows = [
        "# case\tvocabulary_hex\tformula_hex\ttext\toutcome — paraphrase-pt/v1",
        "# outcome = h(P) hex para render/parse, ou err:<ParaphraseError> para parse",
    ]
    for name, vocab, formula_wire in POSITIVE:
        text = render(vocab, formula_wire)
        wires = f"{encode(vocab).hex()}\t{encode(formula_wire).hex()}"
        rows.append(f"{name}\t{wires}\t{text}\t{paraphrase_hash(vocab, text)}")
    for name, vocab, text, error in NEGATIVE:
        rows.append(f"{name}\t{encode(vocab).hex()}\t-\t{text}\terr:{error}")
    contents = "\n".join(rows) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != contents:
            raise SystemExit("spec/vectors/paraphrase/vectors.tsv desatualizado")
    else:
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(contents)
    print(f"PARAPHRASE_VECTORS_OK positive={len(POSITIVE)} negative={len(NEGATIVE)}")


if __name__ == "__main__":
    main()
