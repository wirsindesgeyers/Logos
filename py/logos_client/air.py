"""Sintaxe de superfície AIR (air-text/v1) e pretty-printer determinístico.

Implementação independente da Etapa 1.4 conforme spec/air-surface.md.
Fora da TCB. Saídas são compatíveis com air/v1 e vocabulary/v1 em canon/v1.
"""

from __future__ import annotations

import json
import unicodedata
from collections import deque
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any, cast

from logos_client.canon import (
    CanonError,
    Value,
    check_text,
)

__all__ = [
    "Claim",
    "Document",
    "SurfaceError",
    "parse_document",
    "parse_formula",
    "print_document",
    "print_formula",
]

LOGIC_ID = "FOL-ms-eq/v1"
AIR_FORMAT_VERSION = 1
VOCABULARY_FORMAT_VERSION = 1

MAX_AIR_DEPTH = 128
MAX_AIR_NODES = 100_000
MAX_TEXT_NESTING = 4096
U64_MAX = 18_446_744_073_709_551_615

KEYWORDS: frozenset[str] = frozenset(
    {
        "vocab",
        "sort",
        "const",
        "func",
        "pred",
        "decl",
        "symbol",
        "bool",
        "enum",
        "from",
        "fact",
        "policy",
        "glossed",
        "anchored",
        "defined",
        "gloss",
        "claim",
        "cites",
        "true",
        "false",
        "eq",
        "not",
        "and",
        "or",
        "implies",
        "iff",
        "forall",
        "exists",
        "var",
        "app",
    }
)


class SurfaceError(Exception):
    """Erro de parsing ou validação da sintaxe de superfície AIR."""

    def __init__(self, kind: str, line: int, column: int, message: str) -> None:
        super().__init__(f"[{line}:{column}] {kind}: {message}")
        self.kind = kind
        self.line = line
        self.column = column
        self.message = message


@dataclass(frozen=True)
class Claim:
    """Afirmação (claim) em memória: nome, fórmula canônica e lista ordenada de citações."""

    name: str
    formula: Value
    cites: list[str]


@dataclass(frozen=True)
class Document:
    """Documento AIR completo: vocabulário canônico e lista ordenada de claims."""

    vocabulary: Value
    claims: list[Claim]


def _is_int(v: Any) -> bool:
    """Verifica se ``v`` é inteiro unsigned canônico e NÃO booleano (identidade CBOR)."""
    return type(v) is int


def _as_list(v: Any) -> list[Value]:
    if not isinstance(v, list):
        raise SurfaceError("InvalidAst", 1, 1, "expected list in canon value")
    return cast(list[Value], v)


def _as_str(v: Any) -> str:
    if not isinstance(v, str):
        raise SurfaceError("InvalidAst", 1, 1, "expected string in canon value")
    return v


def _as_int(v: Any) -> int:
    if type(v) is not int:
        raise SurfaceError("InvalidAst", 1, 1, "expected integer in canon value")
    return v


def _as_bytes(v: Any) -> bytes:
    if not isinstance(v, bytes):
        raise SurfaceError("InvalidAst", 1, 1, "expected bytes in canon value")
    return v


def _check_canon_text(s: str) -> None:
    try:
        check_text(s)
    except CanonError as e:
        raise SurfaceError("InvalidText", 1, 1, f"invalid canon text: {e.kind}") from None


# ---------------------------------------------------------------------------
# Lexer
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class _Token:
    kind: str
    value: Any
    line: int
    column: int


def _is_whitespace(c: str) -> bool:
    """Verifica se ``c`` possui a propriedade Unicode White_Space.

    Equivalente a char::is_whitespace em Rust. Python str.isspace() inclui
    indevidamente U+001C..U+001F (ASCII 28..31), que não são White_Space.
    """
    return c.isspace() and not ("\x1c" <= c <= "\x1f")


def _json_str(s: str) -> str:
    """Renderiza string JSON determinística em UTF-8 NFC."""
    return json.dumps(s, ensure_ascii=False)


class _Lexer:
    def __init__(self, text: str) -> None:
        self.text = text
        self.pos = 0
        self.line = 1
        self.col = 1
        self.length = len(text)

    def _advance_char(self) -> str:
        c = self.text[self.pos]
        self.pos += 1
        if c == "\n":
            self.line += 1
            self.col = 1
        else:
            self.col += 1
        return c

    def _skip_whitespace_and_comments(self) -> None:
        while self.pos < self.length:
            c = self.text[self.pos]
            if _is_whitespace(c):
                self._advance_char()
            elif c == "#":
                # Comentário até LF (\n) ou EOF fora de strings.
                while self.pos < self.length and self.text[self.pos] != "\n":
                    self._advance_char()
            else:
                break

    def tokenize(self) -> list[_Token]:
        tokens: list[_Token] = []
        while True:
            self._skip_whitespace_and_comments()
            if self.pos >= self.length:
                tokens.append(_Token("EOF", None, self.line, self.col))
                break

            tok_line = self.line
            tok_col = self.col
            c = self.text[self.pos]

            if ("a" <= c <= "z") or ("A" <= c <= "Z") or c == "_":
                tokens.append(self._lex_identifier())

            elif "0" <= c <= "9":
                tokens.append(self._lex_number())

            elif c == '"':
                tokens.append(self._lex_string())

            elif self.text.startswith("<->", self.pos):
                self._advance_char()
                self._advance_char()
                self._advance_char()
                tokens.append(_Token("<->", "<->", tok_line, tok_col))

            elif self.text.startswith("->", self.pos):
                self._advance_char()
                self._advance_char()
                tokens.append(_Token("->", "->", tok_line, tok_col))

            elif c in "{}()[],:.;=":
                ch = self._advance_char()
                tokens.append(_Token(ch, ch, tok_line, tok_col))

            else:
                raise SurfaceError("Syntax", tok_line, tok_col, f"unrecognized character {c!r}")

        return tokens

    def _lex_identifier(self) -> _Token:
        tok_line = self.line
        tok_col = self.col
        chars: list[str] = []
        while self.pos < self.length:
            c = self.text[self.pos]
            if ("a" <= c <= "z") or ("A" <= c <= "Z") or ("0" <= c <= "9") or c == "_":
                chars.append(self._advance_char())
            else:
                break
        name = "".join(chars)
        if name in KEYWORDS:
            return _Token(name, name, tok_line, tok_col)
        return _Token("IDENT", name, tok_line, tok_col)

    def _lex_number(self) -> _Token:
        tok_line = self.line
        tok_col = self.col
        digits: list[str] = []
        while self.pos < self.length and "0" <= self.text[self.pos] <= "9":
            digits.append(self._advance_char())
        if self.pos < self.length:
            c = self.text[self.pos]
            if ("a" <= c <= "z") or ("A" <= c <= "Z") or c == "_":
                raise SurfaceError(
                    "Syntax", self.line, self.col, f"unexpected character {c!r} in integer"
                )
        val = int("".join(digits))
        if val > U64_MAX:
            raise SurfaceError("LimitExceeded", tok_line, tok_col, "integer exceeds u64 limit")
        return _Token("UINT", val, tok_line, tok_col)

    def _lex_string(self) -> _Token:
        tok_line = self.line
        tok_col = self.col
        self._advance_char()  # consome aspas inicial
        chars: list[str] = []

        while self.pos < self.length:
            c = self.text[self.pos]
            if c == '"':
                self._advance_char()
                break
            if c == "\n":
                raise SurfaceError(
                    "InvalidText", self.line, self.col, "unescaped newline in string literal"
                )
            if ord(c) < 0x20:
                raise SurfaceError(
                    "InvalidText",
                    self.line,
                    self.col,
                    f"unescaped control character U+{ord(c):04X} in string literal",
                )
            if c == "\\":
                esc_line = self.line
                esc_col = self.col
                self._advance_char()  # consome '\\'
                if self.pos >= self.length:
                    raise SurfaceError(
                        "Syntax", tok_line, tok_col, "unclosed string literal at escape"
                    )
                esc = self._advance_char()
                if esc == '"':
                    chars.append('"')
                elif esc == "\\":
                    chars.append("\\")
                elif esc == "/":
                    chars.append("/")
                elif esc == "b":
                    chars.append("\b")
                elif esc == "f":
                    chars.append("\f")
                elif esc == "n":
                    chars.append("\n")
                elif esc == "r":
                    chars.append("\r")
                elif esc == "t":
                    chars.append("\t")
                elif esc == "u":
                    cp = self._read_hex4(esc_line, esc_col)
                    if 0xD800 <= cp <= 0xDBFF:
                        # High surrogate: requer par surrogate \uXXXX com low surrogate
                        if (
                            self.pos + 2 <= self.length
                            and self.text[self.pos : self.pos + 2] == "\\u"
                        ):
                            self._advance_char()
                            self._advance_char()
                            cp2 = self._read_hex4(self.line, self.col)
                            if 0xDC00 <= cp2 <= 0xDFFF:
                                scalar = 0x10000 + ((cp - 0xD800) << 10) + (cp2 - 0xDC00)
                                chars.append(chr(scalar))
                                continue
                        raise SurfaceError(
                            "InvalidText",
                            esc_line,
                            esc_col,
                            f"lone high surrogate escape \\u{cp:04X}",
                        )
                    elif 0xDC00 <= cp <= 0xDFFF:
                        raise SurfaceError(
                            "InvalidText",
                            esc_line,
                            esc_col,
                            f"lone low surrogate escape \\u{cp:04X}",
                        )
                    else:
                        chars.append(chr(cp))
                else:
                    raise SurfaceError(
                        "Syntax", esc_line, esc_col, f"invalid escape sequence \\{esc}"
                    )
            else:
                chars.append(self._advance_char())
        else:
            raise SurfaceError("Syntax", tok_line, tok_col, "unclosed string literal")

        assembled = "".join(chars)
        # Validação de texto canônico (ordem: code points atribuídos em Unicode 16.0.0, depois NFC)
        for ch in assembled:
            if unicodedata.category(ch) == "Cn":
                raise SurfaceError(
                    "InvalidText",
                    tok_line,
                    tok_col,
                    f"unassigned Unicode 16.0 codepoint U+{ord(ch):04X}",
                )
        if unicodedata.normalize("NFC", assembled) != assembled:
            raise SurfaceError(
                "InvalidText",
                tok_line,
                tok_col,
                "string literal is not normalized in NFC",
            )
        return _Token("STRING", assembled, tok_line, tok_col)

    def _read_hex4(self, err_line: int, err_col: int) -> int:
        if self.pos + 4 > self.length:
            raise SurfaceError("Syntax", err_line, err_col, "incomplete \\u unicode escape")
        h_chars = self.text[self.pos : self.pos + 4]
        if not all(c in "0123456789abcdefABCDEF" for c in h_chars):
            raise SurfaceError("Syntax", err_line, err_col, f"invalid hex in \\u escape: {h_chars}")
        for _ in range(4):
            self._advance_char()
        return int(h_chars, 16)


# ---------------------------------------------------------------------------
# Canon Value Helpers
# ---------------------------------------------------------------------------


def _make_sort(name: str) -> list[Value]:
    _check_canon_text(name)
    tag_ver: Value = AIR_FORMAT_VERSION
    tag_sort: Value = 0
    val_name: Value = name
    return [tag_ver, tag_sort, val_name]


def _make_symbol(name: str, kind: int, arity: list[str], result: str | None) -> list[Value]:
    _check_canon_text(name)
    arity_sorts: list[Value] = [_make_sort(s) for s in arity]
    result_val: list[Value] = [1] if result is None else [0, _make_sort(result)]
    tag_ver: Value = AIR_FORMAT_VERSION
    tag_sym: Value = 1
    val_name: Value = name
    val_kind: Value = kind
    val_arity: Value = arity_sorts
    val_res: Value = result_val
    return [tag_ver, tag_sym, val_name, val_kind, val_arity, val_res]


def _make_term_envelope(nodes: list[Value]) -> list[Value]:
    tag_ver: Value = AIR_FORMAT_VERSION
    tag_term: Value = 2
    tag_logic: Value = LOGIC_ID
    val_nodes: Value = nodes
    return [tag_ver, tag_term, tag_logic, val_nodes]


def _make_formula_envelope(nodes: list[Value]) -> list[Value]:
    tag_ver: Value = AIR_FORMAT_VERSION
    tag_form: Value = 3
    tag_logic: Value = LOGIC_ID
    val_nodes: Value = nodes
    return [tag_ver, tag_form, tag_logic, val_nodes]


def _formula_node(*items: Value) -> Value:
    return list(items)


# ---------------------------------------------------------------------------
# Flat Intermediate Results
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class _TermResult:
    records: list[Value]
    depth: int
    nodes: int
    line: int
    column: int


@dataclass(frozen=True)
class _FormulaResult:
    records: list[Value]
    depth: int
    nodes: int
    line: int
    column: int


# ---------------------------------------------------------------------------
# Formula & Term Parser Frames
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class _InfixFrame:
    lhs: _FormulaResult
    op_tok: _Token
    min_prec: int


@dataclass(frozen=True)
class _GroupFrame:
    min_prec: int


@dataclass(frozen=True)
class _NotFrame:
    op_tok: _Token
    is_paren: bool
    min_prec: int


@dataclass(frozen=True)
class _QuantifierFrame:
    q_tok: _Token
    sort_name: str
    min_prec: int


@dataclass
class _NaryFrame:
    op_tok: _Token
    items: list[_FormulaResult]
    min_prec: int


@dataclass
class _BinaryConstructorFrame:
    op_tok: _Token
    f1: _FormulaResult | None
    min_prec: int


_FormulaFrame = (
    _InfixFrame | _GroupFrame | _NotFrame | _QuantifierFrame | _NaryFrame | _BinaryConstructorFrame
)


@dataclass
class _AppFrame:
    sym: Value
    expected_close: str
    is_raw_app: bool
    child_count: int
    max_child_depth: int
    total_nodes: int
    app_idx: int
    line: int
    column: int


# ---------------------------------------------------------------------------
# Formula & Term Parser
# ---------------------------------------------------------------------------


class _FormulaParser:
    def __init__(
        self,
        tokens: list[_Token],
        vocab_symbols: Mapping[str, Value],
        vocab_sorts: set[str],
    ) -> None:
        self.tokens = tokens
        self.pos = 0
        self.vocab_symbols = vocab_symbols
        self.vocab_sorts = vocab_sorts
        # Pilha de escopos de binders nominais: cada item é lista de (nome, sort_nome)
        self.binders: list[list[tuple[str, str]]] = []
        self.stack: list[_FormulaFrame] = []

    def _current(self) -> _Token:
        if self.pos < len(self.tokens):
            return self.tokens[self.pos]
        return self.tokens[-1]

    def _peek(self, offset: int = 1) -> _Token:
        idx = self.pos + offset
        if idx < len(self.tokens):
            return self.tokens[idx]
        return self.tokens[-1]

    def _advance(self) -> _Token:
        tok = self._current()
        if self.pos < len(self.tokens):
            self.pos += 1
        return tok

    def _expect(self, kind: str) -> _Token:
        tok = self._current()
        if tok.kind != kind:
            raise SurfaceError(
                "Syntax", tok.line, tok.column, f"expected {kind!r}, got {tok.kind!r}"
            )
        return self._advance()

    def _parse_name(self) -> str:
        tok = self._current()
        if tok.kind in ("IDENT", "STRING"):
            self._advance()
            return str(tok.value)
        if tok.kind in KEYWORDS:
            raise SurfaceError(
                "Syntax",
                tok.line,
                tok.column,
                f"reserved keyword {tok.kind!r} used as name must be quoted as string",
            )
        raise SurfaceError(
            "Syntax", tok.line, tok.column, f"expected identifier or string, got {tok.kind!r}"
        )

    def _resolve_binder(self, name: str) -> tuple[int, str] | None:
        """Resolve variável nominal pelo binder mais próximo.

        Retorna (De Bruijn index, sort_name) ou None.
        """
        dist = 0
        for scope in reversed(self.binders):
            for b_name, b_sort in reversed(scope):
                if b_name == name:
                    return (dist, b_sort)
                dist += 1
        return None

    def _precedence(self, op: str) -> int:
        if op == "<->":
            return 0
        if op == "->":
            return 1
        if op == "or":
            return 2
        if op == "and":
            return 3
        return -1

    def parse_term(self) -> _TermResult:
        records: list[Value] = []
        term_stack: list[_AppFrame] = []
        need_term = True
        root_line = 0
        root_col = 0
        term_depth = 0
        term_nodes = 0

        while True:
            if need_term:
                if len(self.stack) + len(term_stack) >= MAX_TEXT_NESTING:
                    tok = self._current()
                    raise SurfaceError(
                        "LimitExceeded",
                        tok.line,
                        tok.column,
                        "textual nesting limit exceeded",
                    )

                tok = self._current()

                if tok.kind == "var":
                    v_tok = self._advance()
                    if not term_stack:
                        root_line, root_col = v_tok.line, v_tok.column
                    self._expect("(")
                    idx_tok = self._expect("UINT")
                    self._expect(",")
                    sort_name = self._parse_name()
                    self._expect(")")
                    node: Value = [0, int(idx_tok.value), _make_sort(sort_name)]
                    records.append(node)
                    term_depth = 1
                    term_nodes = 1
                    need_term = False

                elif tok.kind == "app":
                    a_tok = self._advance()
                    if not term_stack:
                        root_line, root_col = a_tok.line, a_tok.column
                    self._expect("(")
                    sym = self._parse_symbol()
                    self._expect(",")
                    self._expect("[")
                    if self._current().kind == "]":
                        self._advance()
                        self._expect(")")
                        records.append([1, sym, 0])
                        term_depth = 1
                        term_nodes = 1
                        need_term = False
                    else:
                        app_idx = len(records)
                        records.append([1, sym, 0])
                        term_stack.append(
                            _AppFrame(
                                sym=sym,
                                expected_close="]",
                                is_raw_app=True,
                                child_count=0,
                                max_child_depth=0,
                                total_nodes=1,
                                app_idx=app_idx,
                                line=a_tok.line,
                                column=a_tok.column,
                            )
                        )
                        need_term = True

                elif tok.kind in ("IDENT", "STRING"):
                    name_tok = self._current()
                    name = self._parse_name()
                    if not term_stack:
                        root_line, root_col = name_tok.line, name_tok.column
                    if self._current().kind == "(":
                        self._advance()
                        if name not in self.vocab_symbols:
                            raise SurfaceError(
                                "UnknownName",
                                name_tok.line,
                                name_tok.column,
                                f"unknown symbol {name!r}",
                            )
                        sym = self.vocab_symbols[name]
                        if self._current().kind == ")":
                            self._advance()
                            records.append([1, sym, 0])
                            term_depth = 1
                            term_nodes = 1
                            need_term = False
                        else:
                            app_idx = len(records)
                            records.append([1, sym, 0])
                            term_stack.append(
                                _AppFrame(
                                    sym=sym,
                                    expected_close=")",
                                    is_raw_app=False,
                                    child_count=0,
                                    max_child_depth=0,
                                    total_nodes=1,
                                    app_idx=app_idx,
                                    line=name_tok.line,
                                    column=name_tok.column,
                                )
                            )
                            need_term = True
                    else:
                        resolved = self._resolve_binder(name)
                        if resolved is not None:
                            idx, sort_name = resolved
                            node = [0, idx, _make_sort(sort_name)]
                            records.append(node)
                            term_depth = 1
                            term_nodes = 1
                            need_term = False
                        elif name in self.vocab_symbols:
                            node = [1, self.vocab_symbols[name], 0]
                            records.append(node)
                            term_depth = 1
                            term_nodes = 1
                            need_term = False
                        else:
                            raise SurfaceError(
                                "UnknownName",
                                name_tok.line,
                                name_tok.column,
                                f"unknown variable or symbol {name!r}",
                            )

                else:
                    raise SurfaceError(
                        "Syntax",
                        tok.line,
                        tok.column,
                        f"unexpected token in term: {tok.kind!r}",
                    )

            else:
                if not term_stack:
                    return _TermResult(
                        records,
                        term_depth,
                        term_nodes,
                        root_line,
                        root_col,
                    )

                parent = term_stack[-1]
                parent.child_count += 1
                if term_depth > parent.max_child_depth:
                    parent.max_child_depth = term_depth
                parent.total_nodes += term_nodes

                if self._current().kind == ",":
                    self._advance()
                    need_term = True
                elif self._current().kind == parent.expected_close:
                    self._advance()
                    if parent.is_raw_app:
                        self._expect(")")
                    frame = term_stack.pop()
                    records[frame.app_idx] = [1, frame.sym, frame.child_count]
                    term_depth = 1 + frame.max_child_depth
                    term_nodes = frame.total_nodes
                    if term_depth > MAX_AIR_DEPTH:
                        raise SurfaceError(
                            "LimitExceeded",
                            frame.line,
                            frame.column,
                            f"AIR depth {term_depth} exceeds limit 128",
                        )
                    if term_nodes > MAX_AIR_NODES:
                        raise SurfaceError(
                            "LimitExceeded",
                            frame.line,
                            frame.column,
                            f"AIR nodes {term_nodes} exceeds limit 100000",
                        )
                    need_term = False
                else:
                    tok = self._current()
                    raise SurfaceError(
                        "Syntax",
                        tok.line,
                        tok.column,
                        f"expected ',' or {parent.expected_close!r}, got {tok.kind!r}",
                    )

    def _build_app_term(
        self, sym: Value, terms: list[_TermResult], line: int, col: int
    ) -> _TermResult:
        child_recs: list[Value] = []
        max_d = 0
        total_n = 1
        for t in terms:
            child_recs.extend(t.records)
            if t.depth > max_d:
                max_d = t.depth
            total_n += t.nodes
        head: Value = [1, sym, len(terms)]
        return _TermResult([head, *child_recs], 1 + max_d, total_n, line, col)

    def _build_eq_formula(
        self, t1: _TermResult, t2: _TermResult, line: int, col: int
    ) -> _FormulaResult:
        lt_env: Value = _make_term_envelope(t1.records)
        rt_env: Value = _make_term_envelope(t2.records)
        depth = 1 + max(t1.depth, t2.depth)
        nodes = 1 + t1.nodes + t2.nodes
        if depth > MAX_AIR_DEPTH:
            raise SurfaceError("LimitExceeded", line, col, f"AIR depth {depth} exceeds limit 128")
        if nodes > MAX_AIR_NODES:
            raise SurfaceError(
                "LimitExceeded", line, col, f"AIR nodes {nodes} exceeds limit 100000"
            )
        node: Value = [2, lt_env, rt_env]
        return _FormulaResult([node], depth, nodes, line, col)

    def _build_pred_formula(
        self, sym: Value, terms: list[_TermResult], line: int, col: int
    ) -> _FormulaResult:
        term_envs: list[Value] = [_make_term_envelope(t.records) for t in terms]
        max_t_d = max((t.depth for t in terms), default=0)
        depth = 1 + max_t_d
        nodes = 1 + sum(t.nodes for t in terms)
        if depth > MAX_AIR_DEPTH:
            raise SurfaceError("LimitExceeded", line, col, f"AIR depth {depth} exceeds limit 128")
        if nodes > MAX_AIR_NODES:
            raise SurfaceError(
                "LimitExceeded", line, col, f"AIR nodes {nodes} exceeds limit 100000"
            )
        node: Value = [3, sym, term_envs]
        return _FormulaResult([node], depth, nodes, line, col)

    def _parse_symbol(self) -> Value:
        self._expect("symbol")
        self._expect("(")
        name = self._parse_name()
        self._expect(",")
        kind_tok = self._current()
        if kind_tok.kind in ("const", "func", "pred"):
            kind_str = kind_tok.kind
            self._advance()
        elif kind_tok.kind == "STRING" and kind_tok.value in ("const", "func", "pred"):
            kind_str = str(kind_tok.value)
            self._advance()
        else:
            raise SurfaceError(
                "Syntax",
                kind_tok.line,
                kind_tok.column,
                f"expected 'const', 'func', or 'pred', got {kind_tok.kind!r}",
            )
        kind_int = 0 if kind_str == "const" else (1 if kind_str == "func" else 2)
        self._expect(",")
        self._expect("[")
        arity: list[str] = []
        if self._current().kind != "]":
            arity.append(self._parse_name())
            while self._current().kind == ",":
                self._advance()
                arity.append(self._parse_name())
        self._expect("]")
        self._expect(",")

        res_tok = self._current()
        if res_tok.kind == "bool":
            self._advance()
            result: str | None = None
        else:
            result = self._parse_name()

        self._expect(")")
        return _make_symbol(name, kind_int, arity, result)

    def parse_formula(self, initial_min_prec: int = 0) -> _FormulaResult:
        self.stack.clear()
        need_expr = True
        current_min_prec = initial_min_prec
        current_result: _FormulaResult | None = None

        while True:
            if need_expr:
                if len(self.stack) >= MAX_TEXT_NESTING:
                    tok = self._current()
                    raise SurfaceError(
                        "LimitExceeded",
                        tok.line,
                        tok.column,
                        "textual nesting limit exceeded",
                    )

                tok = self._current()

                if tok.kind == "true":
                    self._advance()
                    current_result = _FormulaResult([_formula_node(0)], 1, 1, tok.line, tok.column)
                    need_expr = False

                elif tok.kind == "false":
                    self._advance()
                    current_result = _FormulaResult([_formula_node(1)], 1, 1, tok.line, tok.column)
                    need_expr = False

                elif tok.kind == "(":
                    self._advance()
                    self.stack.append(_GroupFrame(min_prec=current_min_prec))
                    current_min_prec = 0
                    need_expr = True

                elif tok.kind == "not":
                    op_tok = self._advance()
                    if self._current().kind == "(":
                        self._advance()
                        self.stack.append(
                            _NotFrame(op_tok, is_paren=True, min_prec=current_min_prec)
                        )
                        current_min_prec = 0
                    else:
                        self.stack.append(
                            _NotFrame(op_tok, is_paren=False, min_prec=current_min_prec)
                        )
                        current_min_prec = 4
                    need_expr = True

                elif tok.kind in ("forall", "exists"):
                    q_tok = self._advance()
                    var_name = self._parse_name()
                    self._expect(":")
                    sort_name = self._parse_name()
                    self._expect(".")
                    self.binders.append([(var_name, sort_name)])
                    self.stack.append(_QuantifierFrame(q_tok, sort_name, min_prec=current_min_prec))
                    current_min_prec = 0
                    need_expr = True

                elif tok.kind in ("and", "or"):
                    if self._peek(1).kind == "(" and self._peek(2).kind == "[":
                        op_tok = self._advance()
                        self._expect("(")
                        self._expect("[")
                        if self._current().kind == "]":
                            self._advance()
                            self._expect(")")
                            tag_val: Value = 5 if op_tok.kind == "and" else 6
                            current_result = _FormulaResult(
                                [[tag_val, 0]], 1, 1, op_tok.line, op_tok.column
                            )
                            need_expr = False
                        else:
                            self.stack.append(
                                _NaryFrame(op_tok, items=[], min_prec=current_min_prec)
                            )
                            current_min_prec = 0
                            need_expr = True
                    else:
                        raise SurfaceError(
                            "Syntax",
                            tok.line,
                            tok.column,
                            f"unexpected token in atom: {tok.kind!r}",
                        )

                elif tok.kind in ("implies", "iff"):
                    if self._peek(1).kind == "(":
                        op_tok = self._advance()
                        self._expect("(")
                        self.stack.append(
                            _BinaryConstructorFrame(op_tok, f1=None, min_prec=current_min_prec)
                        )
                        current_min_prec = 0
                        need_expr = True
                    else:
                        raise SurfaceError(
                            "Syntax",
                            tok.line,
                            tok.column,
                            f"unexpected token in atom: {tok.kind!r}",
                        )

                elif tok.kind == "eq":
                    eq_tok = self._advance()
                    self._expect("(")
                    t1 = self.parse_term()
                    self._expect(",")
                    t2 = self.parse_term()
                    self._expect(")")
                    current_result = self._build_eq_formula(t1, t2, eq_tok.line, eq_tok.column)
                    need_expr = False

                elif tok.kind == "pred":
                    p_tok = self._advance()
                    self._expect("(")
                    sym = self._parse_symbol()
                    self._expect(",")
                    self._expect("[")
                    terms: list[_TermResult] = []
                    if self._current().kind != "]":
                        terms.append(self.parse_term())
                        while self._current().kind == ",":
                            self._advance()
                            terms.append(self.parse_term())
                    self._expect("]")
                    self._expect(")")
                    current_result = self._build_pred_formula(sym, terms, p_tok.line, p_tok.column)
                    need_expr = False

                elif tok.kind in ("var", "app"):
                    lhs_term = self.parse_term()
                    eq_tok = self._expect("=")
                    rhs_term = self.parse_term()
                    current_result = self._build_eq_formula(
                        lhs_term, rhs_term, eq_tok.line, eq_tok.column
                    )
                    need_expr = False

                elif tok.kind in ("IDENT", "STRING"):
                    name_tok = self._current()
                    name = self._parse_name()
                    if self._current().kind == "(":
                        self._advance()
                        terms = []
                        if self._current().kind != ")":
                            terms.append(self.parse_term())
                            while self._current().kind == ",":
                                self._advance()
                                terms.append(self.parse_term())
                        self._expect(")")
                        if self._current().kind == "=":
                            eq_tok = self._advance()
                            if name not in self.vocab_symbols:
                                raise SurfaceError(
                                    "UnknownName",
                                    name_tok.line,
                                    name_tok.column,
                                    f"unknown symbol {name!r}",
                                )
                            lhs_term = self._build_app_term(
                                self.vocab_symbols[name],
                                terms,
                                name_tok.line,
                                name_tok.column,
                            )
                            rhs_term = self.parse_term()
                            current_result = self._build_eq_formula(
                                lhs_term, rhs_term, eq_tok.line, eq_tok.column
                            )
                        else:
                            if name not in self.vocab_symbols:
                                raise SurfaceError(
                                    "UnknownName",
                                    name_tok.line,
                                    name_tok.column,
                                    f"unknown symbol {name!r}",
                                )
                            current_result = self._build_pred_formula(
                                self.vocab_symbols[name],
                                terms,
                                name_tok.line,
                                name_tok.column,
                            )
                        need_expr = False
                    elif self._current().kind == "=":
                        eq_tok = self._advance()
                        resolved = self._resolve_binder(name)
                        if resolved is not None:
                            idx, sort_name = resolved
                            lhs_term = _TermResult(
                                [[0, idx, _make_sort(sort_name)]],
                                1,
                                1,
                                name_tok.line,
                                name_tok.column,
                            )
                        elif name in self.vocab_symbols:
                            lhs_term = self._build_app_term(
                                self.vocab_symbols[name],
                                [],
                                name_tok.line,
                                name_tok.column,
                            )
                        else:
                            raise SurfaceError(
                                "UnknownName",
                                name_tok.line,
                                name_tok.column,
                                f"unknown variable or symbol {name!r}",
                            )
                        rhs_term = self.parse_term()
                        current_result = self._build_eq_formula(
                            lhs_term, rhs_term, eq_tok.line, eq_tok.column
                        )
                        need_expr = False
                    else:
                        if name not in self.vocab_symbols:
                            raise SurfaceError(
                                "UnknownName",
                                name_tok.line,
                                name_tok.column,
                                f"unknown symbol {name!r}",
                            )
                        current_result = self._build_pred_formula(
                            self.vocab_symbols[name],
                            [],
                            name_tok.line,
                            name_tok.column,
                        )
                        need_expr = False

                else:
                    raise SurfaceError(
                        "Syntax",
                        tok.line,
                        tok.column,
                        f"unexpected token in atom: {tok.kind!r}",
                    )

            else:
                assert current_result is not None
                tok = self._current()
                prec = self._precedence(tok.kind)
                if prec >= current_min_prec:
                    op_tok = self._advance()
                    next_min_prec = prec if op_tok.kind == "->" else prec + 1
                    self.stack.append(
                        _InfixFrame(
                            lhs=current_result,
                            op_tok=op_tok,
                            min_prec=current_min_prec,
                        )
                    )
                    current_min_prec = next_min_prec
                    need_expr = True
                    continue

                if not self.stack:
                    return current_result

                top_frame = self.stack[-1]

                if isinstance(top_frame, _InfixFrame):
                    infix_frame = cast(_InfixFrame, self.stack.pop())
                    depth = 1 + max(infix_frame.lhs.depth, current_result.depth)
                    nodes = 1 + infix_frame.lhs.nodes + current_result.nodes
                    if depth > MAX_AIR_DEPTH:
                        raise SurfaceError(
                            "LimitExceeded",
                            infix_frame.op_tok.line,
                            infix_frame.op_tok.column,
                            f"AIR depth {depth} exceeds limit 128",
                        )
                    if nodes > MAX_AIR_NODES:
                        raise SurfaceError(
                            "LimitExceeded",
                            infix_frame.op_tok.line,
                            infix_frame.op_tok.column,
                            f"AIR nodes {nodes} exceeds limit 100000",
                        )
                    if infix_frame.op_tok.kind == "<->":
                        recs = [_formula_node(8), *infix_frame.lhs.records, *current_result.records]
                    elif infix_frame.op_tok.kind == "->":
                        recs = [_formula_node(7), *infix_frame.lhs.records, *current_result.records]
                    elif infix_frame.op_tok.kind == "or":
                        recs = [
                            _formula_node(6, 2),
                            *infix_frame.lhs.records,
                            *current_result.records,
                        ]
                    elif infix_frame.op_tok.kind == "and":
                        recs = [
                            _formula_node(5, 2),
                            *infix_frame.lhs.records,
                            *current_result.records,
                        ]
                    else:
                        raise SurfaceError(
                            "Syntax",
                            infix_frame.op_tok.line,
                            infix_frame.op_tok.column,
                            f"unknown operator {infix_frame.op_tok.kind!r}",
                        )
                    current_result = _FormulaResult(
                        recs,
                        depth,
                        nodes,
                        infix_frame.op_tok.line,
                        infix_frame.op_tok.column,
                    )
                    current_min_prec = infix_frame.min_prec
                    need_expr = False

                elif isinstance(top_frame, _GroupFrame):
                    group_frame = cast(_GroupFrame, self.stack.pop())
                    self._expect(")")
                    current_min_prec = group_frame.min_prec
                    need_expr = False

                elif isinstance(top_frame, _NotFrame):
                    not_frame = cast(_NotFrame, self.stack.pop())
                    if not_frame.is_paren:
                        self._expect(")")
                    depth = 1 + current_result.depth
                    nodes = 1 + current_result.nodes
                    if depth > MAX_AIR_DEPTH:
                        raise SurfaceError(
                            "LimitExceeded",
                            not_frame.op_tok.line,
                            not_frame.op_tok.column,
                            f"AIR depth {depth} exceeds limit 128",
                        )
                    if nodes > MAX_AIR_NODES:
                        raise SurfaceError(
                            "LimitExceeded",
                            not_frame.op_tok.line,
                            not_frame.op_tok.column,
                            f"AIR nodes {nodes} exceeds limit 100000",
                        )
                    current_result = _FormulaResult(
                        [_formula_node(4), *current_result.records],
                        depth,
                        nodes,
                        not_frame.op_tok.line,
                        not_frame.op_tok.column,
                    )
                    current_min_prec = not_frame.min_prec
                    need_expr = False

                elif isinstance(top_frame, _QuantifierFrame):
                    q_frame = cast(_QuantifierFrame, self.stack.pop())
                    self.binders.pop()
                    depth = 1 + current_result.depth
                    nodes = 1 + current_result.nodes
                    if depth > MAX_AIR_DEPTH:
                        raise SurfaceError(
                            "LimitExceeded",
                            q_frame.q_tok.line,
                            q_frame.q_tok.column,
                            f"AIR depth {depth} exceeds limit 128",
                        )
                    if nodes > MAX_AIR_NODES:
                        raise SurfaceError(
                            "LimitExceeded",
                            q_frame.q_tok.line,
                            q_frame.q_tok.column,
                            f"AIR nodes {nodes} exceeds limit 100000",
                        )
                    tag = 9 if q_frame.q_tok.kind == "forall" else 10
                    sort_val = _make_sort(q_frame.sort_name)
                    node_val: Value = [tag, sort_val]
                    current_result = _FormulaResult(
                        [node_val, *current_result.records],
                        depth,
                        nodes,
                        q_frame.q_tok.line,
                        q_frame.q_tok.column,
                    )
                    current_min_prec = q_frame.min_prec
                    need_expr = False

                elif isinstance(top_frame, _NaryFrame):
                    nary_frame = top_frame
                    nary_frame.items.append(current_result)
                    if self._current().kind == ",":
                        self._advance()
                        current_min_prec = 0
                        need_expr = True
                    elif self._current().kind == "]":
                        self._advance()
                        self._expect(")")
                        self.stack.pop()
                        tag_val = 5 if nary_frame.op_tok.kind == "and" else 6
                        count_val = len(nary_frame.items)
                        depth = 1 + max((it.depth for it in nary_frame.items), default=0)
                        nodes = 1 + sum(it.nodes for it in nary_frame.items)
                        if depth > MAX_AIR_DEPTH:
                            raise SurfaceError(
                                "LimitExceeded",
                                nary_frame.op_tok.line,
                                nary_frame.op_tok.column,
                                f"AIR depth {depth} exceeds limit 128",
                            )
                        if nodes > MAX_AIR_NODES:
                            raise SurfaceError(
                                "LimitExceeded",
                                nary_frame.op_tok.line,
                                nary_frame.op_tok.column,
                                f"AIR nodes {nodes} exceeds limit 100000",
                            )
                        nary_head: Value = [tag_val, count_val]
                        recs = [nary_head]
                        for it in nary_frame.items:
                            recs.extend(it.records)
                        current_result = _FormulaResult(
                            recs,
                            depth,
                            nodes,
                            nary_frame.op_tok.line,
                            nary_frame.op_tok.column,
                        )
                        current_min_prec = nary_frame.min_prec
                        need_expr = False
                    else:
                        tok = self._current()
                        raise SurfaceError(
                            "Syntax",
                            tok.line,
                            tok.column,
                            f"expected ',' or ']', got {tok.kind!r}",
                        )

                elif isinstance(top_frame, _BinaryConstructorFrame):
                    bin_frame = top_frame
                    if bin_frame.f1 is None:
                        bin_frame.f1 = current_result
                        self._expect(",")
                        current_min_prec = 0
                        need_expr = True
                    else:
                        self._expect(")")
                        self.stack.pop()
                        tag_val = 7 if bin_frame.op_tok.kind == "implies" else 8
                        depth = 1 + max(bin_frame.f1.depth, current_result.depth)
                        nodes = 1 + bin_frame.f1.nodes + current_result.nodes
                        if depth > MAX_AIR_DEPTH:
                            raise SurfaceError(
                                "LimitExceeded",
                                bin_frame.op_tok.line,
                                bin_frame.op_tok.column,
                                f"AIR depth {depth} exceeds limit 128",
                            )
                        if nodes > MAX_AIR_NODES:
                            raise SurfaceError(
                                "LimitExceeded",
                                bin_frame.op_tok.line,
                                bin_frame.op_tok.column,
                                f"AIR nodes {nodes} exceeds limit 100000",
                            )
                        binary_head: Value = [tag_val]
                        current_result = _FormulaResult(
                            [binary_head, *bin_frame.f1.records, *current_result.records],
                            depth,
                            nodes,
                            bin_frame.op_tok.line,
                            bin_frame.op_tok.column,
                        )
                        current_min_prec = bin_frame.min_prec
                        need_expr = False


# ---------------------------------------------------------------------------
# AST Validation
# ---------------------------------------------------------------------------


def _validate_sort(val: Any) -> None:
    v_list = _as_list(val)
    if len(v_list) != 3 or _as_int(v_list[0]) != AIR_FORMAT_VERSION or _as_int(v_list[1]) != 0:
        raise SurfaceError("InvalidAst", 1, 1, "invalid Sort envelope")
    name = _as_str(v_list[2])
    _check_canon_text(name)


def _validate_symbol(val: Any) -> None:
    v_list = _as_list(val)
    if (
        len(v_list) != 6
        or _as_int(v_list[0]) != AIR_FORMAT_VERSION
        or _as_int(v_list[1]) != 1
        or _as_int(v_list[3]) not in (0, 1, 2)
    ):
        raise SurfaceError("InvalidAst", 1, 1, "invalid Symbol envelope")
    name = _as_str(v_list[2])
    _check_canon_text(name)
    arity = _as_list(v_list[4])
    for s in arity:
        _validate_sort(s)
    result = _as_list(v_list[5])
    if len(result) == 1 and _as_int(result[0]) == 1:
        pass
    elif len(result) == 2 and _as_int(result[0]) == 0:
        _validate_sort(result[1])
    else:
        raise SurfaceError("InvalidAst", 1, 1, "invalid Symbol result")


def _validate_term(term_env: Any) -> tuple[int, int]:
    """Valida envelope de Term sob limites de AIR. Retorna (depth, nodes)."""
    v_list = _as_list(term_env)
    if (
        len(v_list) != 4
        or _as_int(v_list[0]) != AIR_FORMAT_VERSION
        or _as_int(v_list[1]) != 2
        or _as_str(v_list[2]) != LOGIC_ID
    ):
        raise SurfaceError("InvalidAst", 1, 1, "invalid Term envelope")

    records = _as_list(v_list[3])
    if len(records) == 0:
        raise SurfaceError("InvalidAst", 1, 1, "empty term prefix tree")

    stack: list[int] = []
    nodes = 0
    max_depth = 0

    for rec_val in records:
        while stack and stack[-1] == 0:
            stack.pop()
        if nodes > 0 and not stack:
            raise SurfaceError("InvalidAst", 1, 1, "extra roots in term prefix tree")
        depth = 1 + len(stack)
        if depth > MAX_AIR_DEPTH:
            raise SurfaceError("LimitExceeded", 1, 1, f"AIR depth {depth} exceeds limit 128")
        nodes += 1
        if nodes > MAX_AIR_NODES:
            raise SurfaceError("LimitExceeded", 1, 1, f"AIR nodes {nodes} exceeds limit 100000")
        if depth > max_depth:
            max_depth = depth
        if stack:
            stack[-1] -= 1

        rec = _as_list(rec_val)
        if len(rec) < 3:
            raise SurfaceError("InvalidAst", 1, 1, "malformed term record")

        tag = _as_int(rec[0])
        if tag == 0:  # Var: [0, index, sort]
            idx = _as_int(rec[1])
            if idx < 0 or idx > U64_MAX:
                raise SurfaceError("InvalidAst", 1, 1, "invalid Var index")
            _validate_sort(rec[2])
        elif tag == 1:  # App: [1, symbol, child_count]
            _validate_symbol(rec[1])
            count = _as_int(rec[2])
            if count < 0:
                raise SurfaceError("InvalidAst", 1, 1, "invalid App child count")
            if count > 0:
                stack.append(count)
        else:
            raise SurfaceError("InvalidAst", 1, 1, f"unknown term record tag {tag}")

    while stack and stack[-1] == 0:
        stack.pop()
    if stack:
        raise SurfaceError("InvalidAst", 1, 1, "incomplete term prefix tree")

    return (max_depth, nodes)


def _validate_formula(formula_env: Any) -> tuple[int, int]:
    """Valida envelope de Formula e limites agregados. Retorna (depth, nodes)."""
    v_list = _as_list(formula_env)
    if (
        len(v_list) != 4
        or _as_int(v_list[0]) != AIR_FORMAT_VERSION
        or _as_int(v_list[1]) != 3
        or _as_str(v_list[2]) != LOGIC_ID
    ):
        raise SurfaceError("InvalidAst", 1, 1, "invalid Formula envelope")

    records = _as_list(v_list[3])
    if len(records) == 0:
        raise SurfaceError("InvalidAst", 1, 1, "empty formula prefix tree")

    stack: list[int] = []
    nodes = 0
    max_depth = 0

    for rec_val in records:
        while stack and stack[-1] == 0:
            stack.pop()
        if nodes > 0 and not stack:
            raise SurfaceError("InvalidAst", 1, 1, "extra roots in formula prefix tree")
        depth = 1 + len(stack)
        if depth > MAX_AIR_DEPTH:
            raise SurfaceError("LimitExceeded", 1, 1, f"AIR depth {depth} exceeds limit 128")
        nodes += 1
        if nodes > MAX_AIR_NODES:
            raise SurfaceError("LimitExceeded", 1, 1, f"AIR nodes {nodes} exceeds limit 100000")
        if depth > max_depth:
            max_depth = depth
        if stack:
            stack[-1] -= 1

        rec = _as_list(rec_val)
        if len(rec) == 0:
            raise SurfaceError("InvalidAst", 1, 1, "malformed formula record")

        tag = _as_int(rec[0])
        if tag in (0, 1):  # True, False
            pass
        elif tag == 2:  # Eq: [2, left_term, right_term]
            if len(rec) != 3:
                raise SurfaceError("InvalidAst", 1, 1, "malformed Eq record")
            lt_depth, lt_nodes = _validate_term(rec[1])
            rt_depth, rt_nodes = _validate_term(rec[2])
            total_d = max(depth + lt_depth, depth + rt_depth)
            if total_d > MAX_AIR_DEPTH:
                raise SurfaceError("LimitExceeded", 1, 1, f"AIR depth {total_d} exceeds limit 128")
            nodes += lt_nodes + rt_nodes
            if nodes > MAX_AIR_NODES:
                raise SurfaceError("LimitExceeded", 1, 1, f"AIR nodes {nodes} exceeds limit 100000")
            if total_d > max_depth:
                max_depth = total_d
        elif tag == 3:  # Pred: [3, symbol, args]
            if len(rec) != 3:
                raise SurfaceError("InvalidAst", 1, 1, "malformed Pred record")
            _validate_symbol(rec[1])
            args_list = _as_list(rec[2])
            for arg in args_list:
                t_depth, t_nodes = _validate_term(arg)
                total_d = depth + t_depth
                if total_d > MAX_AIR_DEPTH:
                    raise SurfaceError(
                        "LimitExceeded", 1, 1, f"AIR depth {total_d} exceeds limit 128"
                    )
                nodes += t_nodes
                if nodes > MAX_AIR_NODES:
                    raise SurfaceError(
                        "LimitExceeded", 1, 1, f"AIR nodes {nodes} exceeds limit 100000"
                    )
                if total_d > max_depth:
                    max_depth = total_d
        elif tag == 4:  # Not
            stack.append(1)
        elif tag in (5, 6):  # And, Or: [tag, count]
            if len(rec) != 2:
                raise SurfaceError("InvalidAst", 1, 1, "malformed And/Or record")
            count = _as_int(rec[1])
            if count < 0:
                raise SurfaceError("InvalidAst", 1, 1, "negative And/Or count")
            if count > 0:
                stack.append(count)
        elif tag in (7, 8):  # Implies, Iff
            stack.append(2)
        elif tag in (9, 10):  # Forall, Exists: [tag, sort]
            if len(rec) != 2:
                raise SurfaceError("InvalidAst", 1, 1, "malformed Forall/Exists record")
            _validate_sort(rec[1])
            stack.append(1)
        else:
            raise SurfaceError("InvalidAst", 1, 1, f"unknown formula record tag {tag}")

    while stack and stack[-1] == 0:
        stack.pop()
    if stack:
        raise SurfaceError("InvalidAst", 1, 1, "incomplete formula prefix tree")

    return (max_depth, nodes)


def _validate_vocabulary(vocab_val: Any) -> None:
    v_list = _as_list(vocab_val)
    if len(v_list) != 4 or _as_int(v_list[0]) != VOCABULARY_FORMAT_VERSION:
        raise SurfaceError("InvalidVocabulary", 1, 1, "invalid Vocabulary envelope")

    version = _as_int(v_list[1])
    if version < 0 or version > U64_MAX:
        raise SurfaceError("InvalidVocabulary", 1, 1, "invalid version")

    sorts = _as_list(v_list[2])
    symbols = _as_list(v_list[3])

    sort_names: set[str] = set()
    for s_decl_val in sorts:
        s_decl = _as_list(s_decl_val)
        if len(s_decl) != 2:
            raise SurfaceError("InvalidVocabulary", 1, 1, "invalid SortDecl")
        s_name = _as_str(s_decl[0])
        _check_canon_text(s_name)
        if s_name in sort_names:
            raise SurfaceError("DuplicateName", 1, 1, f"duplicate sort {s_name!r}")
        sort_names.add(s_name)

    symbol_names: dict[str, Value] = {}
    for sym_decl_val in symbols:
        sym_decl = _as_list(sym_decl_val)
        if len(sym_decl) != 3:
            raise SurfaceError("InvalidVocabulary", 1, 1, "invalid SymbolDecl")
        sig = sym_decl[0]
        _validate_symbol(sig)
        name = _as_str(_as_list(sig)[2])
        if name in symbol_names:
            raise SurfaceError("DuplicateName", 1, 1, f"duplicate symbol {name!r}")
        symbol_names[name] = sig
        gloss_pt = _as_str(sym_decl[2])
        _check_canon_text(gloss_pt)

    # Valida sorts usados em assinaturas
    for sym_decl_val in symbols:
        sym_decl = _as_list(sym_decl_val)
        sig = _as_list(sym_decl[0])
        for s in _as_list(sig[4]):
            s_name = _as_str(_as_list(s)[2])
            if s_name not in sort_names:
                raise SurfaceError("UnknownName", 1, 1, f"undeclared sort {s_name!r}")
        res_list = _as_list(sig[5])
        if not (len(res_list) == 1 and _as_int(res_list[0]) == 1):
            s_name = _as_str(_as_list(res_list[1])[2])
            if s_name not in sort_names:
                raise SurfaceError("UnknownName", 1, 1, f"undeclared sort {s_name!r}")

    # Valida domínios enumerados
    for s_decl_val in sorts:
        s_decl = _as_list(s_decl_val)
        domain = _as_list(s_decl[1])
        if len(domain) == 0:
            raise SurfaceError("InvalidVocabulary", 1, 1, "invalid sort domain")
        dom_tag = _as_int(domain[0])
        if len(domain) == 1 and dom_tag == 0:
            continue
        if dom_tag == 1 and len(domain) == 3:
            consts = _as_list(domain[1])
            src = _as_list(domain[2])
            if len(src) != 2:
                raise SurfaceError("InvalidVocabulary", 1, 1, "invalid domain source")
            src_tag = _as_int(src[0])
            if src_tag not in (0, 1):
                raise SurfaceError("InvalidVocabulary", 1, 1, "invalid domain source tag")
            src_bytes = _as_bytes(src[1])
            if len(src_bytes) != 32:
                raise SurfaceError("InvalidVocabulary", 1, 1, "invalid domain source length")
            seen_consts: set[str] = set()
            for c in consts:
                _validate_symbol(c)
                c_sig = _as_list(c)
                c_name = _as_str(c_sig[2])
                if c_name not in symbol_names:
                    raise SurfaceError("UnknownName", 1, 1, f"unknown enum constant {c_name!r}")
                if symbol_names[c_name] != c:
                    raise SurfaceError("InvalidVocabulary", 1, 1, "signature mismatch in enum")
                c_kind = _as_int(c_sig[3])
                c_arity = _as_list(c_sig[4])
                c_res = _as_list(c_sig[5])
                s_decl_name = _as_str(s_decl[0])
                is_correct_sort = (
                    len(c_res) == 2
                    and _as_int(c_res[0]) == 0
                    and _as_str(_as_list(c_res[1])[2]) == s_decl_name
                )
                if c_kind != 0 or len(c_arity) != 0 or not is_correct_sort or c_name in seen_consts:
                    raise SurfaceError(
                        "InvalidVocabulary", 1, 1, f"invalid domain constant {c_name!r}"
                    )
                seen_consts.add(c_name)
        else:
            raise SurfaceError("InvalidVocabulary", 1, 1, "invalid sort domain tag")

    # Valida corpos de símbolos Defined e aciclicidade
    defined_indices: dict[str, int] = {}
    for idx, sym_decl_val in enumerate(symbols):
        sym_decl = _as_list(sym_decl_val)
        gr = _as_list(sym_decl[1])
        if len(gr) == 0:
            raise SurfaceError("InvalidVocabulary", 1, 1, "invalid grounding")
        gr_tag = _as_int(gr[0])
        sig_val = _as_list(sym_decl[0])
        sym_name = _as_str(sig_val[2])
        if gr_tag == 0:
            if len(gr) != 2:
                raise SurfaceError("InvalidVocabulary", 1, 1, "invalid Defined grounding")
            _validate_formula(gr[1])
            defined_indices[sym_name] = idx
        elif gr_tag == 1:
            if len(gr) != 3:
                raise SurfaceError("InvalidVocabulary", 1, 1, "invalid Anchored grounding")
            _check_canon_text(_as_str(gr[1]))
            _check_canon_text(_as_str(gr[2]))
        elif gr_tag == 2:
            if len(gr) != 2:
                raise SurfaceError("InvalidVocabulary", 1, 1, "invalid Glossed grounding")
            _check_canon_text(_as_str(gr[1]))
        else:
            raise SurfaceError("InvalidVocabulary", 1, 1, f"invalid grounding tag {gr_tag}")

    # Checa dependências e ciclos entre Defined
    deps: dict[int, set[int]] = {i: set() for i in defined_indices.values()}
    for sym_name, idx in defined_indices.items():
        sym_decl_item = _as_list(symbols[idx])
        gr_item = _as_list(sym_decl_item[1])
        formula_val = _as_list(gr_item[1])
        f_recs = _as_list(formula_val[3])
        for rec_val in f_recs:
            rec = _as_list(rec_val)
            tag = _as_int(rec[0])
            if tag in (9, 10):
                s_name = _as_str(_as_list(rec[1])[2])
                if s_name not in sort_names:
                    raise SurfaceError("UnknownName", 1, 1, f"undeclared sort {s_name!r}")
            elif tag == 3:
                s_sig = rec[1]
                s_sig_list = _as_list(s_sig)
                s_name = _as_str(s_sig_list[2])
                if s_name not in symbol_names:
                    raise SurfaceError("UnknownName", 1, 1, f"undeclared symbol {s_name!r}")
                if symbol_names[s_name] != s_sig:
                    raise SurfaceError("InvalidVocabulary", 1, 1, "signature mismatch in formula")
                if s_name == sym_name:
                    raise SurfaceError(
                        "InvalidVocabulary", 1, 1, f"self-referential defined symbol {s_name!r}"
                    )
                if s_name in defined_indices:
                    deps[idx].add(defined_indices[s_name])
                args_terms = _as_list(rec[2])
                for term in args_terms:
                    _collect_term_deps(
                        term, sym_name, idx, sort_names, symbol_names, defined_indices, deps
                    )
            elif tag == 2:  # Eq
                _collect_term_deps(
                    rec[1], sym_name, idx, sort_names, symbol_names, defined_indices, deps
                )
                _collect_term_deps(
                    rec[2], sym_name, idx, sort_names, symbol_names, defined_indices, deps
                )

    # Ordenação topológica (Kahn) para verificar ausência de ciclos
    in_degree = {i: 0 for i in defined_indices.values()}
    dependents: dict[int, list[int]] = {i: [] for i in defined_indices.values()}
    for src_idx, targets in deps.items():
        for tgt_idx in targets:
            dependents[tgt_idx].append(src_idx)
            in_degree[src_idx] += 1

    ready = deque([i for i, deg in in_degree.items() if deg == 0])
    visited = 0
    while ready:
        node = ready.popleft()
        visited += 1
        for dep in dependents[node]:
            in_degree[dep] -= 1
            if in_degree[dep] == 0:
                ready.append(dep)

    if visited != len(defined_indices):
        raise SurfaceError("InvalidVocabulary", 1, 1, "circular vocabulary definition")


def _collect_term_deps(
    term_env: Value,
    current_sym: str,
    current_idx: int,
    sort_names: set[str],
    symbol_names: dict[str, Value],
    defined_indices: dict[str, int],
    deps: dict[int, set[int]],
) -> None:
    t_list = _as_list(term_env)
    t_recs = _as_list(t_list[3])
    for rec_val in t_recs:
        rec = _as_list(rec_val)
        tag = _as_int(rec[0])
        if tag == 0:  # Var: [0, index, sort]
            s_name = _as_str(_as_list(rec[2])[2])
            if s_name not in sort_names:
                raise SurfaceError("UnknownName", 1, 1, f"undeclared sort {s_name!r}")
        elif tag == 1:  # App: [1, symbol, count]
            s_sig = rec[1]
            s_name = _as_str(_as_list(s_sig)[2])
            if s_name not in symbol_names:
                raise SurfaceError("UnknownName", 1, 1, f"undeclared symbol {s_name!r}")
            if symbol_names[s_name] != s_sig:
                raise SurfaceError("InvalidVocabulary", 1, 1, "signature mismatch in term")
            if s_name == current_sym:
                raise SurfaceError(
                    "InvalidVocabulary", 1, 1, f"self-referential defined symbol {s_name!r}"
                )
            if s_name in defined_indices:
                deps[current_idx].add(defined_indices[s_name])


# ---------------------------------------------------------------------------
# Pretty-Printer
# ---------------------------------------------------------------------------


def _print_symbol(sym: Value) -> str:
    s_list = _as_list(sym)
    name = _as_str(s_list[2])
    kind_int = _as_int(s_list[3])
    arity = _as_list(s_list[4])
    result = _as_list(s_list[5])
    if kind_int == 0:
        kind_str = "const"
    elif kind_int == 1:
        kind_str = "func"
    elif kind_int == 2:
        kind_str = "pred"
    else:
        raise SurfaceError("InvalidAst", 1, 1, f"invalid symbol kind {kind_int}")
    arity_items = [_json_str(_as_str(_as_list(s)[2])) for s in arity]
    arity_str = "[" + ", ".join(arity_items) + "]"
    res_str = (
        "bool"
        if len(result) == 1 and _as_int(result[0]) == 1
        else _json_str(_as_str(_as_list(result[1])[2]))
    )
    return f"symbol({_json_str(name)}, {kind_str}, {arity_str}, {res_str})"


def _print_term(term_env: Value) -> str:
    """Pretty-printer iterativo e plano para Term."""
    t_list = _as_list(term_env)
    records = _as_list(t_list[3])
    stack: list[str] = []
    for rec_val in reversed(records):
        rec = _as_list(rec_val)
        tag = _as_int(rec[0])
        if tag == 0:  # Var: [0, index, sort]
            idx = _as_int(rec[1])
            sort_val = _as_list(rec[2])
            sort_name = _as_str(sort_val[2])
            stack.append(f"var({idx}, {_json_str(sort_name)})")
        elif tag == 1:  # App: [1, symbol, child_count]
            count = _as_int(rec[2])
            children = [stack.pop() for _ in range(count)]
            sym_str = _print_symbol(rec[1])
            stack.append(f"app({sym_str}, [{', '.join(children)}])")
        else:
            raise SurfaceError("InvalidAst", 1, 1, f"invalid term tag {tag}")
    return stack[0]


def print_formula(formula: Value) -> str:
    """Pretty-printer determinístico e iterativo (flat) para AIR Formula.

    Preserva toda AST sintaticamente válida sob os limites normativos.
    """
    _validate_formula(formula)
    f_list = _as_list(formula)
    records = _as_list(f_list[3])
    stack: list[str] = []

    for i in range(len(records) - 1, -1, -1):
        rec = _as_list(records[i])
        tag = _as_int(rec[0])

        if tag == 0:
            stack.append("true")
        elif tag == 1:
            stack.append("false")
        elif tag == 2:  # Eq: [2, left, right]
            l_str = _print_term(rec[1])
            r_str = _print_term(rec[2])
            stack.append(f"eq({l_str}, {r_str})")
        elif tag == 3:  # Pred: [3, symbol, args]
            sym_str = _print_symbol(rec[1])
            args_list = _as_list(rec[2])
            args_str = [_print_term(arg) for arg in args_list]
            stack.append(f"pred({sym_str}, [{', '.join(args_str)}])")
        elif tag == 4:  # Not
            child = stack.pop()
            stack.append(f"not({child})")
        elif tag == 5:  # And: [5, count]
            count = _as_int(rec[1])
            children = [stack.pop() for _ in range(count)]
            stack.append(f"and([{', '.join(children)}])")
        elif tag == 6:  # Or: [6, count]
            count = _as_int(rec[1])
            children = [stack.pop() for _ in range(count)]
            stack.append(f"or([{', '.join(children)}])")
        elif tag == 7:  # Implies
            left_str = stack.pop()
            right_str = stack.pop()
            stack.append(f"implies({left_str}, {right_str})")
        elif tag == 8:  # Iff
            left_str = stack.pop()
            right_str = stack.pop()
            stack.append(f"iff({left_str}, {right_str})")
        elif tag == 9:  # Forall: [9, sort]
            child = stack.pop()
            sort_val = _as_list(rec[1])
            s_name = _as_str(sort_val[2])
            stack.append(f'forall "_q{i}": {_json_str(s_name)}. {child}')
        elif tag == 10:  # Exists: [10, sort]
            child = stack.pop()
            sort_val = _as_list(rec[1])
            s_name = _as_str(sort_val[2])
            stack.append(f'exists "_q{i}": {_json_str(s_name)}. {child}')
        else:
            raise SurfaceError("InvalidAst", 1, 1, f"invalid formula tag {tag}")

    return stack[0]


def _print_grounding(grounding: Value) -> str:
    gr = _as_list(grounding)
    tag = _as_int(gr[0])
    if tag == 0:
        f_str = print_formula(gr[1])
        return f"= defined({f_str})"
    elif tag == 1:
        att = _json_str(_as_str(gr[1]))
        proc = _json_str(_as_str(gr[2]))
        return f"= anchored({att}, {proc})"
    elif tag == 2:
        txt = _json_str(_as_str(gr[1]))
        return f"= glossed({txt})"
    raise SurfaceError("InvalidVocabulary", 1, 1, f"invalid grounding tag {tag}")


def print_document(document: Document) -> str:
    """Pretty-printer determinístico de documento AIR."""
    _validate_vocabulary(document.vocabulary)
    for claim in document.claims:
        _validate_formula(claim.formula)

    vocab = _as_list(document.vocabulary)
    version = _as_int(vocab[1])
    sorts = _as_list(vocab[2])
    symbols = _as_list(vocab[3])

    lines = [f"vocab v{version} {{\n"]
    for s_decl_val in sorts:
        s_decl = _as_list(s_decl_val)
        s_name = _as_str(s_decl[0])
        domain = _as_list(s_decl[1])
        if len(domain) == 1 and _as_int(domain[0]) == 0:
            lines.append(f"  sort {_json_str(s_name)}\n")
        elif len(domain) == 3 and _as_int(domain[0]) == 1:
            consts = _as_list(domain[1])
            src = _as_list(domain[2])
            c_names = [_json_str(_as_str(_as_list(c)[2])) for c in consts]
            src_kind = "fact" if _as_int(src[0]) == 0 else "policy"
            hex_str = _as_bytes(src[1]).hex().lower()
            c_part = ", ".join(c_names)
            s_json = _json_str(s_name)
            h_json = _json_str(hex_str)
            lines.append(f"  sort {s_json} = enum [{c_part}] from {src_kind} {h_json}\n")
        else:
            raise SurfaceError("InvalidVocabulary", 1, 1, "invalid sort domain")

    for sym_decl_val in symbols:
        sym_decl = _as_list(sym_decl_val)
        sig = sym_decl[0]
        gr = sym_decl[1]
        gloss = _as_str(sym_decl[2])
        sym_p = _print_symbol(sig)
        gr_p = _print_grounding(gr)
        gl_p = _json_str(gloss)
        lines.append(f"  decl {sym_p} {gr_p} gloss {gl_p}\n")

    lines.append("}\n")

    for claim in document.claims:
        c_name = _json_str(claim.name)
        f_str = print_formula(claim.formula)
        if claim.cites:
            cites_str = ", ".join(_json_str(c) for c in claim.cites)
            lines.append(f"claim {c_name}: {f_str} cites [{cites_str}]\n")
        else:
            lines.append(f"claim {c_name}: {f_str}\n")

    return "".join(lines)


# ---------------------------------------------------------------------------
# Public Parsing APIs
# ---------------------------------------------------------------------------


def parse_formula(text: str, vocabulary: Value = None) -> Value:
    """Faz o parsing de uma fórmula textual AIR e retorna o envelope canônico Formula.

    O parâmetro ``vocabulary`` fornece símbolos e sorts declarados para resolução nominal.
    """
    vocab_symbols: dict[str, Value] = {}
    vocab_sorts: set[str] = set()

    if vocabulary is not None:
        _validate_vocabulary(vocabulary)
        v_list = _as_list(vocabulary)
        sorts_list = _as_list(v_list[2])
        for s_decl_val in sorts_list:
            s_decl = _as_list(s_decl_val)
            vocab_sorts.add(_as_str(s_decl[0]))
        sym_list = _as_list(v_list[3])
        for sym_decl_val in sym_list:
            sym_decl = _as_list(sym_decl_val)
            sig = sym_decl[0]
            sig_list = _as_list(sig)
            vocab_symbols[_as_str(sig_list[2])] = sig

    lexer = _Lexer(text)
    tokens = lexer.tokenize()

    parser = _FormulaParser(tokens, vocab_symbols, vocab_sorts)
    res = parser.parse_formula(0)
    eof_tok = parser._current()
    if eof_tok.kind != "EOF":
        raise SurfaceError(
            "Syntax",
            eof_tok.line,
            eof_tok.column,
            f"unexpected token after formula: {eof_tok.kind!r}",
        )

    if res.depth > MAX_AIR_DEPTH:
        raise SurfaceError("LimitExceeded", 1, 1, f"AIR depth {res.depth} exceeds limit 128")
    if res.nodes > MAX_AIR_NODES:
        raise SurfaceError("LimitExceeded", 1, 1, f"AIR nodes {res.nodes} exceeds limit 100000")

    return _make_formula_envelope(res.records)


@dataclass
class _SortDeclIntermediate:
    name: str
    is_enum: bool
    const_names: list[str]
    source_kind: str
    reference_name: str
    line: int
    column: int


@dataclass
class _SymbolDeclIntermediate:
    name: str
    kind: int
    arity: list[str]
    result: str | None
    grounding_type: str  # "glossed", "anchored", "defined"
    grounding_text: str = ""
    grounding_attestor: str = ""
    grounding_proc: str = ""
    defined_tokens: list[_Token] | None = None
    gloss: str = ""
    line: int = 1
    column: int = 1


class _DocumentParser:
    def __init__(self, tokens: list[_Token], references: Mapping[str, bytes]) -> None:
        self.tokens = tokens
        self.pos = 0
        self.references = references

    def _current(self) -> _Token:
        return self.tokens[self.pos] if self.pos < len(self.tokens) else self.tokens[-1]

    def _advance(self) -> _Token:
        t = self._current()
        if self.pos < len(self.tokens):
            self.pos += 1
        return t

    def _expect(self, kind: str) -> _Token:
        t = self._current()
        if t.kind != kind:
            raise SurfaceError("Syntax", t.line, t.column, f"expected {kind!r}, got {t.kind!r}")
        return self._advance()

    def _parse_name(self) -> str:
        t = self._current()
        if t.kind in ("IDENT", "STRING"):
            self._advance()
            return str(t.value)
        if t.kind in KEYWORDS:
            raise SurfaceError(
                "Syntax",
                t.line,
                t.column,
                f"reserved keyword {t.kind!r} used as name must be quoted as string",
            )
        raise SurfaceError(
            "Syntax", t.line, t.column, f"expected identifier or string, got {t.kind!r}"
        )

    def _parse_string_val(self) -> str:
        t = self._current()
        if t.kind != "STRING":
            raise SurfaceError(
                "Syntax", t.line, t.column, f"expected string literal, got {t.kind!r}"
            )
        self._advance()
        return str(t.value)

    def _parse_optional_grounding(
        self,
    ) -> tuple[str, str, str, str, list[_Token] | None]:
        if self._current().kind == "=":
            self._advance()
            return self._parse_grounding_body()
        return ("glossed", "", "", "", None)

    def _parse_grounding_body(
        self,
    ) -> tuple[str, str, str, str, list[_Token] | None]:
        tok = self._current()
        if tok.kind == "glossed":
            self._advance()
            self._expect("(")
            txt = self._parse_string_val()
            self._expect(")")
            return ("glossed", txt, "", "", None)
        elif tok.kind == "anchored":
            self._advance()
            self._expect("(")
            att = self._parse_string_val()
            self._expect(",")
            proc = self._parse_string_val()
            self._expect(")")
            return ("anchored", "", att, proc, None)
        elif tok.kind == "defined":
            self._advance()
            self._expect("(")
            paren_depth = 1
            formula_tokens: list[_Token] = []
            while paren_depth > 0:
                t = self._current()
                if t.kind == "EOF":
                    raise SurfaceError(
                        "Syntax", t.line, t.column, "unclosed defined(...) grounding"
                    )
                if t.kind == "(":
                    paren_depth += 1
                elif t.kind == ")":
                    paren_depth -= 1
                    if paren_depth == 0:
                        self._advance()
                        break
                formula_tokens.append(self._advance())
            formula_tokens.append(_Token("EOF", None, self._current().line, self._current().column))
            return ("defined", "", "", "", formula_tokens)
        else:
            raise SurfaceError(
                "Syntax",
                tok.line,
                tok.column,
                f"expected 'glossed', 'anchored', or 'defined', got {tok.kind!r}",
            )

    def _parse_optional_gloss(self) -> str:
        if self._current().kind == "gloss":
            self._advance()
            return self._parse_string_val()
        return ""

    def parse(self) -> Document:
        # 1. Cabeçalho vocab v<N> {
        self._expect("vocab")
        v_tok = self._current()
        if (
            v_tok.kind != "IDENT"
            or not isinstance(v_tok.value, str)
            or not v_tok.value.startswith("v")
            or len(v_tok.value) < 2
            or not v_tok.value[1:].isdigit()
        ):
            raise SurfaceError(
                "Syntax",
                v_tok.line,
                v_tok.column,
                f"expected vocabulary version 'v<digits>', got {v_tok.value!r}",
            )
        self._advance()
        version = int(v_tok.value[1:])
        if version > U64_MAX:
            raise SurfaceError(
                "LimitExceeded",
                v_tok.line,
                v_tok.column,
                "version number exceeds u64 limit",
            )
        self._expect("{")

        sorts_order: list[_SortDeclIntermediate] = []
        symbols_order: list[_SymbolDeclIntermediate] = []
        seen_sort_names: set[str] = set()
        seen_symbol_names: set[str] = set()

        # Pass 1: Coleta de declarações
        while self._current().kind != "}":
            tok = self._current()
            if tok.kind == "EOF":
                raise SurfaceError("Syntax", tok.line, tok.column, "unclosed vocab block")

            if tok.kind == "sort":
                s_tok = self._advance()
                s_name = self._parse_name()
                if s_name in seen_sort_names:
                    raise SurfaceError(
                        "DuplicateName", s_tok.line, s_tok.column, f"duplicate sort name {s_name!r}"
                    )
                seen_sort_names.add(s_name)

                if self._current().kind == "=":
                    self._advance()
                    self._expect("enum")
                    self._expect("[")
                    members: list[str] = []
                    if self._current().kind != "]":
                        members.append(self._parse_name())
                        while self._current().kind == ",":
                            self._advance()
                            members.append(self._parse_name())
                    self._expect("]")
                    self._expect("from")
                    src_tok = self._current()
                    if src_tok.kind in ("fact", "policy"):
                        src_kind = src_tok.kind
                        self._advance()
                    else:
                        raise SurfaceError(
                            "Syntax",
                            src_tok.line,
                            src_tok.column,
                            f"expected 'fact' or 'policy', got {src_tok.kind!r}",
                        )
                    ref_name = self._parse_name()
                    sorts_order.append(
                        _SortDeclIntermediate(
                            name=s_name,
                            is_enum=True,
                            const_names=members,
                            source_kind=src_kind,
                            reference_name=ref_name,
                            line=s_tok.line,
                            column=s_tok.column,
                        )
                    )
                else:
                    sorts_order.append(
                        _SortDeclIntermediate(
                            name=s_name,
                            is_enum=False,
                            const_names=[],
                            source_kind="",
                            reference_name="",
                            line=s_tok.line,
                            column=s_tok.column,
                        )
                    )

            elif tok.kind == "const":
                c_tok = self._advance()
                name = self._parse_name()
                if name in seen_symbol_names:
                    raise SurfaceError(
                        "DuplicateName", c_tok.line, c_tok.column, f"duplicate symbol name {name!r}"
                    )
                seen_symbol_names.add(name)
                self._expect(":")
                sort_name = self._parse_name()
                gr_type, gr_txt, gr_att, gr_proc, gr_toks = self._parse_optional_grounding()
                gloss = self._parse_optional_gloss()
                symbols_order.append(
                    _SymbolDeclIntermediate(
                        name=name,
                        kind=0,
                        arity=[],
                        result=sort_name,
                        grounding_type=gr_type,
                        grounding_text=gr_txt,
                        grounding_attestor=gr_att,
                        grounding_proc=gr_proc,
                        defined_tokens=gr_toks,
                        gloss=gloss,
                        line=c_tok.line,
                        column=c_tok.column,
                    )
                )

            elif tok.kind == "func":
                f_tok = self._advance()
                name = self._parse_name()
                if name in seen_symbol_names:
                    raise SurfaceError(
                        "DuplicateName", f_tok.line, f_tok.column, f"duplicate symbol name {name!r}"
                    )
                seen_symbol_names.add(name)
                self._expect("(")
                arity: list[str] = []
                if self._current().kind != ")":
                    arity.append(self._parse_name())
                    while self._current().kind == ",":
                        self._advance()
                        arity.append(self._parse_name())
                self._expect(")")
                self._expect(":")
                res_sort = self._parse_name()
                gr_type, gr_txt, gr_att, gr_proc, gr_toks = self._parse_optional_grounding()
                gloss = self._parse_optional_gloss()
                symbols_order.append(
                    _SymbolDeclIntermediate(
                        name=name,
                        kind=1,
                        arity=arity,
                        result=res_sort,
                        grounding_type=gr_type,
                        grounding_text=gr_txt,
                        grounding_attestor=gr_att,
                        grounding_proc=gr_proc,
                        defined_tokens=gr_toks,
                        gloss=gloss,
                        line=f_tok.line,
                        column=f_tok.column,
                    )
                )

            elif tok.kind == "pred":
                p_tok = self._advance()
                name = self._parse_name()
                if name in seen_symbol_names:
                    raise SurfaceError(
                        "DuplicateName", p_tok.line, p_tok.column, f"duplicate symbol name {name!r}"
                    )
                seen_symbol_names.add(name)
                self._expect("(")
                arity = []
                if self._current().kind != ")":
                    arity.append(self._parse_name())
                    while self._current().kind == ",":
                        self._advance()
                        arity.append(self._parse_name())
                self._expect(")")
                gr_type, gr_txt, gr_att, gr_proc, gr_toks = self._parse_optional_grounding()
                gloss = self._parse_optional_gloss()
                symbols_order.append(
                    _SymbolDeclIntermediate(
                        name=name,
                        kind=2,
                        arity=arity,
                        result=None,
                        grounding_type=gr_type,
                        grounding_text=gr_txt,
                        grounding_attestor=gr_att,
                        grounding_proc=gr_proc,
                        defined_tokens=gr_toks,
                        gloss=gloss,
                        line=p_tok.line,
                        column=p_tok.column,
                    )
                )

            elif tok.kind == "decl":
                d_tok = self._advance()
                self._expect("symbol")
                self._expect("(")
                name = self._parse_name()
                if name in seen_symbol_names:
                    raise SurfaceError(
                        "DuplicateName", d_tok.line, d_tok.column, f"duplicate symbol name {name!r}"
                    )
                seen_symbol_names.add(name)
                self._expect(",")
                k_tok = self._current()
                if k_tok.kind in ("const", "func", "pred"):
                    kind_str = k_tok.kind
                    self._advance()
                elif k_tok.kind == "STRING" and k_tok.value in ("const", "func", "pred"):
                    kind_str = str(k_tok.value)
                    self._advance()
                else:
                    raise SurfaceError(
                        "Syntax",
                        k_tok.line,
                        k_tok.column,
                        f"expected 'const', 'func', or 'pred', got {k_tok.kind!r}",
                    )
                kind_int = 0 if kind_str == "const" else (1 if kind_str == "func" else 2)
                self._expect(",")
                self._expect("[")
                arity = []
                if self._current().kind != "]":
                    arity.append(self._parse_name())
                    while self._current().kind == ",":
                        self._advance()
                        arity.append(self._parse_name())
                self._expect("]")
                self._expect(",")
                res_tok = self._current()
                if res_tok.kind == "bool":
                    self._advance()
                    res_sort_opt = None
                else:
                    res_sort_opt = self._parse_name()
                self._expect(")")

                self._expect("=")
                gr_type, gr_txt, gr_att, gr_proc, gr_toks = self._parse_grounding_body()
                self._expect("gloss")
                gloss_str = self._parse_string_val()
                symbols_order.append(
                    _SymbolDeclIntermediate(
                        name=name,
                        kind=kind_int,
                        arity=arity,
                        result=res_sort_opt,
                        grounding_type=gr_type,
                        grounding_text=gr_txt,
                        grounding_attestor=gr_att,
                        grounding_proc=gr_proc,
                        defined_tokens=gr_toks,
                        gloss=gloss_str,
                        line=d_tok.line,
                        column=d_tok.column,
                    )
                )

            else:
                raise SurfaceError(
                    "Syntax", tok.line, tok.column, f"unexpected declaration {tok.kind!r}"
                )

            if self._current().kind == ";":
                self._advance()

        self._expect("}")

        # Pass 2: Validação de assinaturas e resolução de Defined e Enums
        declared_sort_names = seen_sort_names
        signatures_by_name: dict[str, list[Value]] = {}
        for s_inter in symbols_order:
            for s in s_inter.arity:
                if s not in declared_sort_names:
                    raise SurfaceError(
                        "UnknownName", s_inter.line, s_inter.column, f"undeclared sort {s!r}"
                    )
            if s_inter.result is not None and s_inter.result not in declared_sort_names:
                raise SurfaceError(
                    "UnknownName",
                    s_inter.line,
                    s_inter.column,
                    f"undeclared sort {s_inter.result!r}",
                )
            sig = _make_symbol(s_inter.name, s_inter.kind, s_inter.arity, s_inter.result)
            signatures_by_name[s_inter.name] = sig

        # Resolução de groundings
        symbol_decls: list[Value] = []
        for s_inter in symbols_order:
            sig = signatures_by_name[s_inter.name]
            gr_val: list[Value]
            if s_inter.grounding_type == "glossed":
                tag_g: Value = 2
                txt_g: Value = s_inter.grounding_text
                gr_val = [tag_g, txt_g]
            elif s_inter.grounding_type == "anchored":
                tag_a: Value = 1
                att_a: Value = s_inter.grounding_attestor
                proc_a: Value = s_inter.grounding_proc
                gr_val = [tag_a, att_a, proc_a]
            elif s_inter.grounding_type == "defined":
                assert s_inter.defined_tokens is not None
                f_parser = _FormulaParser(
                    s_inter.defined_tokens, signatures_by_name, declared_sort_names
                )
                f_res = f_parser.parse_formula(0)
                if f_parser._current().kind != "EOF":
                    eof_t = f_parser._current()
                    raise SurfaceError(
                        "Syntax",
                        eof_t.line,
                        eof_t.column,
                        f"unexpected token in defined formula: {eof_t.kind!r}",
                    )
                if f_res.depth > MAX_AIR_DEPTH:
                    raise SurfaceError(
                        "LimitExceeded",
                        s_inter.line,
                        s_inter.column,
                        f"AIR depth {f_res.depth} exceeds limit 128",
                    )
                if f_res.nodes > MAX_AIR_NODES:
                    raise SurfaceError(
                        "LimitExceeded",
                        s_inter.line,
                        s_inter.column,
                        f"AIR nodes {f_res.nodes} exceeds limit 100000",
                    )
                f_env = _make_formula_envelope(f_res.records)
                tag_d: Value = 0
                val_env: Value = f_env
                gr_val = [tag_d, val_env]
            else:
                raise SurfaceError(
                    "InvalidVocabulary", s_inter.line, s_inter.column, "invalid grounding"
                )

            val_sig: Value = sig
            val_gr: Value = gr_val
            val_gloss: Value = s_inter.gloss
            symbol_decls.append([val_sig, val_gr, val_gloss])

        # Resolução de domínios dos sorts
        sort_decls: list[Value] = []
        for s_inter_sort in sorts_order:
            if not s_inter_sort.is_enum:
                dom_open: Value = [0]
                sort_decls.append([s_inter_sort.name, dom_open])
            else:
                resolved_consts: list[Value] = []
                seen_c: set[str] = set()
                for c_name in s_inter_sort.const_names:
                    if c_name not in signatures_by_name:
                        raise SurfaceError(
                            "UnknownName",
                            s_inter_sort.line,
                            s_inter_sort.column,
                            f"unknown enum constant {c_name!r}",
                        )
                    c_sig = signatures_by_name[c_name]
                    c_sig_list = _as_list(c_sig)
                    c_kind = _as_int(c_sig_list[3])
                    c_arity = _as_list(c_sig_list[4])
                    c_res = _as_list(c_sig_list[5])
                    is_correct_sort = (
                        len(c_res) == 2
                        and _as_int(c_res[0]) == 0
                        and _as_str(_as_list(c_res[1])[2]) == s_inter_sort.name
                    )
                    if c_kind != 0 or len(c_arity) != 0 or not is_correct_sort or c_name in seen_c:
                        raise SurfaceError(
                            "InvalidVocabulary",
                            s_inter_sort.line,
                            s_inter_sort.column,
                            f"invalid domain constant {c_name!r}",
                        )
                    seen_c.add(c_name)
                    resolved_consts.append(c_sig)

                ref = s_inter_sort.reference_name
                ref_bytes: bytes
                if ref in self.references:
                    ref_bytes = self.references[ref]
                    if len(ref_bytes) != 32:
                        raise SurfaceError(
                            "InvalidVocabulary",
                            s_inter_sort.line,
                            s_inter_sort.column,
                            f"reference {ref!r} must be exactly 32 bytes",
                        )
                elif len(ref) == 64 and all(c in "0123456789abcdefABCDEF" for c in ref):
                    ref_bytes = bytes.fromhex(ref)
                else:
                    raise SurfaceError(
                        "UnknownReference",
                        s_inter_sort.line,
                        s_inter_sort.column,
                        f"unknown reference alias {ref!r}",
                    )

                src_tag: Value = 0 if s_inter_sort.source_kind == "fact" else 1
                src_bytes_val: Value = ref_bytes
                src_val: Value = [src_tag, src_bytes_val]
                domain_val: Value = [1, resolved_consts, src_val]
                sort_decls.append([s_inter_sort.name, domain_val])

        ver_tag: Value = VOCABULARY_FORMAT_VERSION
        version_val: Value = version
        sorts_val: Value = sort_decls
        symbols_val: Value = symbol_decls
        vocab_val: list[Value] = [
            ver_tag,
            version_val,
            sorts_val,
            symbols_val,
        ]
        _validate_vocabulary(vocab_val)

        # 3. Claims
        claims: list[Claim] = []
        seen_claim_names: set[str] = set()

        while self._current().kind != "EOF":
            c_tok = self._expect("claim")
            claim_name = self._parse_name()
            if claim_name in seen_claim_names:
                raise SurfaceError(
                    "DuplicateName",
                    c_tok.line,
                    c_tok.column,
                    f"duplicate claim name {claim_name!r}",
                )
            seen_claim_names.add(claim_name)
            self._expect(":")

            f_parser = _FormulaParser(
                self.tokens[self.pos :], signatures_by_name, declared_sort_names
            )
            f_res = f_parser.parse_formula(0)
            self.pos += f_parser.pos

            cites: list[str] = []
            if self._current().kind == "cites":
                self._advance()
                self._expect("[")
                if self._current().kind != "]":
                    cites.append(self._parse_name())
                    while self._current().kind == ",":
                        self._advance()
                        cites.append(self._parse_name())
                self._expect("]")

            if self._current().kind == ";":
                self._advance()

            if self._current().kind not in ("claim", "EOF"):
                tok = self._current()
                raise SurfaceError(
                    "Syntax",
                    tok.line,
                    tok.column,
                    f"unexpected token after claim: {tok.kind!r}",
                )

            if f_res.depth > MAX_AIR_DEPTH:
                raise SurfaceError(
                    "LimitExceeded",
                    c_tok.line,
                    c_tok.column,
                    f"AIR depth {f_res.depth} exceeds limit 128",
                )
            if f_res.nodes > MAX_AIR_NODES:
                raise SurfaceError(
                    "LimitExceeded",
                    c_tok.line,
                    c_tok.column,
                    f"AIR nodes {f_res.nodes} exceeds limit 100000",
                )

            f_env = _make_formula_envelope(f_res.records)
            claims.append(Claim(name=claim_name, formula=f_env, cites=cites))

        self._expect("EOF")
        return Document(vocabulary=vocab_val, claims=claims)


def parse_document(text: str, references: Mapping[str, bytes] | None = None) -> Document:
    """Faz o parsing de um documento AIR completo (vocabulário + claims).

    O mapa ``references`` fornece correspondências alias -> digest de 32 bytes.
    """
    if references is None:
        references = {}
    lexer = _Lexer(text)
    tokens = lexer.tokenize()
    parser = _DocumentParser(tokens, references)
    return parser.parse()
