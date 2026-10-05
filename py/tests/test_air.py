"""Testes da sintaxe de superfície AIR (spec/air-surface.md) e pretty-printer."""

from __future__ import annotations

import pytest

from logos_client.air import (
    SurfaceError,
    parse_document,
    parse_formula,
    print_document,
    print_formula,
)
from logos_client.canon import Value


def _recs(formula_val: Value) -> list[list[Value]]:
    assert isinstance(formula_val, list)
    records = formula_val[3]
    assert isinstance(records, list)
    out: list[list[Value]] = []
    for r in records:
        assert isinstance(r, list)
        out.append(r)
    return out


def _term_nodes(term_env: Value) -> list[list[Value]]:
    assert isinstance(term_env, list)
    records = term_env[3]
    assert isinstance(records, list)
    out: list[list[Value]] = []
    for r in records:
        assert isinstance(r, list)
        out.append(r)
    return out


def _sorts(vocab_val: Value) -> list[list[Value]]:
    assert isinstance(vocab_val, list)
    sorts = vocab_val[2]
    assert isinstance(sorts, list)
    out: list[list[Value]] = []
    for s in sorts:
        assert isinstance(s, list)
        out.append(s)
    return out


def _symbols(vocab_val: Value) -> list[list[Value]]:
    assert isinstance(vocab_val, list)
    symbols = vocab_val[3]
    assert isinstance(symbols, list)
    out: list[list[Value]] = []
    for s in symbols:
        assert isinstance(s, list)
        out.append(s)
    return out


# ---------------------------------------------------------------------------
# 1. Lexer & Validação de Texto
# ---------------------------------------------------------------------------


def test_lexer_whitespace_and_comments() -> None:
    text = "   # Comentário até o fim da linha\n  true  # outro\n"
    res = parse_formula(text)
    assert res == [1, 3, "FOL-ms-eq/v1", [[0]]]


def test_lexer_rejects_control_chars_as_whitespace() -> None:
    # U+001C..U+001F não pertencem a Unicode White_Space, mesmo se str.isspace() retornar True
    for code in (0x1C, 0x1D, 0x1E, 0x1F):
        bad_char = chr(code)
        with pytest.raises(SurfaceError) as exc_info:
            parse_formula(f"{bad_char}true")
        assert exc_info.value.kind == "Syntax"


def test_lexer_string_escapes() -> None:
    # Escapes JSON válidos: \", \\, \/, \b, \f, \n, \r, \t, \u0041
    text = r'eq(var(0, "\"\\\/\b\f\n\r\t\u0041"), var(0, "\"\\\/\b\f\n\r\t\u0041"))'
    res = parse_formula(text)
    expected_str = '"\\/\b\f\n\r\tA'
    recs = _recs(res)
    t_nodes = _term_nodes(recs[0][1])
    sort_val = t_nodes[0][2]
    assert isinstance(sort_val, list)
    assert sort_val[2] == expected_str


def test_lexer_valid_surrogate_pair() -> None:
    # \uD83D\uDE00 é o emoji 😀 (U+1F600)
    text = r'eq(var(0, "\uD83D\uDE00"), var(0, "\uD83D\uDE00"))'
    res = parse_formula(text)
    recs = _recs(res)
    t_nodes = _term_nodes(recs[0][1])
    sort_val = t_nodes[0][2]
    assert isinstance(sort_val, list)
    assert sort_val[2] == "😀"


def test_lexer_rejects_lone_surrogates() -> None:
    # High surrogate isolado
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(r'eq(var(0, "\uD800"), var(0, "S"))')
    assert exc_info.value.kind == "InvalidText"

    # Low surrogate isolado
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(r'eq(var(0, "\uDC00"), var(0, "S"))')
    assert exc_info.value.kind == "InvalidText"


def test_lexer_rejects_unescaped_control_characters_in_string() -> None:
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula('eq(var(0, "linha1\nlinha2"), var(0, "S"))')
    assert exc_info.value.kind == "InvalidText"

    with pytest.raises(SurfaceError) as exc_info:
        parse_formula('eq(var(0, "\x00"), var(0, "S"))')
    assert exc_info.value.kind == "InvalidText"


def test_lexer_rejects_non_nfc_string() -> None:
    # 'e' + combining acute (U+0301) em NFD
    nfd_str = "e\u0301"
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(f'eq(var(0, "{nfd_str}"), var(0, "S"))')
    assert exc_info.value.kind == "InvalidText"


def test_lexer_rejects_unassigned_codepoint() -> None:
    # U+0378 é não-atribuído (Cn) em Unicode 16.0
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula('eq(var(0, "\u0378"), var(0, "S"))')
    assert exc_info.value.kind == "InvalidText"


def test_lexer_rejects_unclosed_string() -> None:
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula('eq(var(0, "não fechada), var(0, "S"))')
    assert exc_info.value.kind == "Syntax"


def test_lexer_rejects_invalid_escape() -> None:
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(r'eq(var(0, "\q"), var(0, "S"))')
    assert exc_info.value.kind == "Syntax"


def test_lexer_integer_limits() -> None:
    # u64::MAX = 18446744073709551615
    ok_text = 'eq(var(18446744073709551615, "S"), var(0, "S"))'
    res = parse_formula(ok_text)
    recs = _recs(res)
    t_nodes = _term_nodes(recs[0][1])
    assert t_nodes[0][1] == 18446744073709551615

    # u64::MAX + 1
    overflow_text = 'eq(var(18446744073709551616, "S"), var(0, "S"))'
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(overflow_text)
    assert exc_info.value.kind == "LimitExceeded"


def test_lexer_version_tokens() -> None:
    doc_ok = "vocab v1 {}"
    doc = parse_document(doc_ok)
    assert isinstance(doc.vocabulary, list)
    assert doc.vocabulary[1] == 1

    doc_v0 = "vocab v0 {}"
    doc0 = parse_document(doc_v0)
    assert isinstance(doc0.vocabulary, list)
    assert doc0.vocabulary[1] == 0

    doc_overflow = "vocab v18446744073709551616 {}"
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_overflow)
    assert exc_info.value.kind == "LimitExceeded"


def test_v_names_as_identifiers() -> None:
    # v1, v01, v12name são identificadores válidos preservando grafia exata
    doc_text = "vocab v1 { sort v01; const v1: v01; } claim v12name: forall v1:v01. v1 = v1"
    doc = parse_document(doc_text)
    sorts = _sorts(doc.vocabulary)
    assert sorts[0][0] == "v01"
    symbols = _symbols(doc.vocabulary)
    sig = symbols[0][0]
    assert isinstance(sig, list)
    assert sig[2] == "v1"
    assert doc.claims[0].name == "v12name"


def test_lexer_strict_eof() -> None:
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula("true extra_garbage")
    assert exc_info.value.kind == "Syntax"


# ---------------------------------------------------------------------------
# 2. Precedência de Operadores e Átomos
# ---------------------------------------------------------------------------


def test_precedence_and_associativity() -> None:
    # <-> associativo à esquerda: (A <-> B) <-> C
    f_iff = "true <-> false <-> true"
    res_iff = parse_formula(f_iff)
    recs_iff = _recs(res_iff)
    assert recs_iff[0] == [8]
    assert recs_iff[1] == [8]
    assert recs_iff[2] == [0]
    assert recs_iff[3] == [1]
    assert recs_iff[4] == [0]

    # -> associativo à direita: A -> (B -> C)
    f_imp = "true -> false -> true"
    res_imp = parse_formula(f_imp)
    recs_imp = _recs(res_imp)
    assert recs_imp[0] == [7]
    assert recs_imp[1] == [0]
    assert recs_imp[2] == [7]
    assert recs_imp[3] == [1]
    assert recs_imp[4] == [0]

    # or associativo à esquerda, menor precedência que and: (A and B) or C
    f_and_or = "true and false or true"
    res_and_or = parse_formula(f_and_or)
    recs_and_or = _recs(res_and_or)
    assert recs_and_or[0] == [6, 2]
    assert recs_and_or[1] == [5, 2]

    # and tem precedência maior que or: A or (B and C)
    f_or_and = "true or false and true"
    res_or_and = parse_formula(f_or_and)
    recs_or_and = _recs(res_or_and)
    assert recs_or_and[0] == [6, 2]
    assert recs_or_and[1] == [0]
    assert recs_or_and[2] == [5, 2]

    # not tem precedência maior que and: (not true) and false
    f_not_and = "not true and false"
    res_not_and = parse_formula(f_not_and)
    recs_not_and = _recs(res_not_and)
    assert recs_not_and[0] == [5, 2]
    assert recs_not_and[1] == [4]


# ---------------------------------------------------------------------------
# 3. Quantificadores, Binders, Shadowing e Escopo
# ---------------------------------------------------------------------------


def test_quantifiers_nominal_binders_and_scopes() -> None:
    # forall x:S. x = x
    text = "forall x:S. x = x"
    res = parse_formula(text)
    recs = _recs(res)
    assert recs[0] == [9, [1, 0, "S"]]
    eq_rec = recs[1]
    assert eq_rec[0] == 2
    lt = _term_nodes(eq_rec[1])
    rt = _term_nodes(eq_rec[2])
    assert lt[0] == [0, 0, [1, 0, "S"]]
    assert rt[0] == [0, 0, [1, 0, "S"]]


def test_quantifier_shadowing() -> None:
    # forall x:S1. forall x:S2. x = x
    # Binder mais interno x tem sort S2 e índice 0
    text = "forall x:S1. forall x:S2. x = x"
    res = parse_formula(text)
    recs = _recs(res)
    assert recs[0] == [9, [1, 0, "S1"]]
    assert recs[1] == [9, [1, 0, "S2"]]
    eq_rec = recs[2]
    lt = _term_nodes(eq_rec[1])
    rt = _term_nodes(eq_rec[2])
    assert lt[0] == [0, 0, [1, 0, "S2"]]
    assert rt[0] == [0, 0, [1, 0, "S2"]]


def test_sibling_quantifier_scope_isolation() -> None:
    # Escopo não deve vazar para irmãos
    text = "(forall x:S. x = x) and (forall y:S. x = y)"
    # No segundo ramo, 'x' não está em escopo e não é símbolo declarado
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(text)
    assert exc_info.value.kind == "UnknownName"


def test_binder_does_not_intercept_application() -> None:
    # x() é sempre aplicação de símbolo x, mesmo existindo binder x
    vocab_text = "vocab v1 { sort S; func x(): S; }"
    doc = parse_document(vocab_text)
    text = "forall x:S. x() = x"
    res = parse_formula(text, doc.vocabulary)
    recs = _recs(res)
    eq_rec = recs[1]
    lt = _term_nodes(eq_rec[1])
    rt = _term_nodes(eq_rec[2])
    # LHS é App do símbolo x
    assert lt[0][0] == 1
    sig = lt[0][1]
    assert isinstance(sig, list)
    assert sig[2] == "x"
    # RHS é Var 0 do binder x
    assert rt[0][0] == 0
    assert rt[0][1] == 0


def test_quantifier_sort_tokens_nominal_without_declared_sort() -> None:
    # Quantificador constrói Sort nominal mesmo se não declarado no vocabulário
    text = "forall u:UndeclaredSort. u = u"
    res = parse_formula(text, None)
    recs = _recs(res)
    assert recs[0] == [9, [1, 0, "UndeclaredSort"]]


# ---------------------------------------------------------------------------
# 4. Construtores Explícitos e Sintaxe Bruta
# ---------------------------------------------------------------------------


def test_explicit_constructors() -> None:
    # and e or com 0 filhos e 3 filhos
    f_and0 = "and([])"
    assert _recs(parse_formula(f_and0))[0] == [5, 0]

    f_or0 = "or([])"
    assert _recs(parse_formula(f_or0))[0] == [6, 0]

    f_and3 = "and([true, false, true])"
    assert _recs(parse_formula(f_and3))[0] == [5, 3]

    f_implies = "implies(true, false)"
    assert _recs(parse_formula(f_implies))[0] == [7]

    f_iff = "iff(true, false)"
    assert _recs(parse_formula(f_iff))[0] == [8]


def test_raw_terms_and_odd_signatures() -> None:
    # var com índice livre arbitrário e sort não declarado
    f_var = 'eq(var(42, "AlienSort"), var(99, "AlienSort"))'
    res_var = parse_formula(f_var)
    recs_var = _recs(res_var)
    lt = _term_nodes(recs_var[0][1])
    rt = _term_nodes(recs_var[0][2])
    assert lt[0] == [0, 42, [1, 0, "AlienSort"]]
    assert rt[0] == [0, 99, [1, 0, "AlienSort"]]

    # app e pred com symbol explícito não consultam vocabulário
    # Symbol com assinatura incomum: kind Func com resultado bool!
    f_odd = 'pred(symbol("weird", func, ["S1", "S2"], bool), [var(0, "S1"), var(1, "S2")])'
    res_odd = parse_formula(f_odd)
    recs_odd = _recs(res_odd)
    assert recs_odd[0][0] == 3
    sig = recs_odd[0][1]
    assert isinstance(sig, list)
    assert sig[2] == "weird"
    assert sig[3] == 1  # Func
    assert sig[5] == [1]  # bool


def test_symbol_bool_keyword_vs_string() -> None:
    # bool sem aspas significa Bool ([1])
    f_bool_res = 'pred(symbol("p", pred, [], bool), [])'
    res_bool = parse_formula(f_bool_res)
    recs_bool = _recs(res_bool)
    sig = recs_bool[0][1]
    assert isinstance(sig, list)
    assert sig[5] == [1]

    # "bool" com aspas significa Sort de nome bool ([0, [1, 0, "bool"]])
    f_sort_bool = 'app(symbol("f", func, [], "bool"), []) = app(symbol("g", func, [], "bool"), [])'
    res_sort = parse_formula(f_sort_bool)
    recs_sort = _recs(res_sort)
    lt = _term_nodes(recs_sort[0][1])
    sig_lt = lt[0][1]
    assert isinstance(sig_lt, list)
    assert sig_lt[5] == [0, [1, 0, "bool"]]


# ---------------------------------------------------------------------------
# 5. Deterministic Property Test: parse(print(f)) == f
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "formula_text",
    [
        "true",
        "false",
        "not(true)",
        "not(false)",
        "and([])",
        "or([])",
        "and([true, false])",
        "or([false, true, false])",
        "implies(true, false)",
        "iff(false, true)",
        'eq(var(0, "User"), var(0, "User"))',
        'eq(var(5, "FreeSort"), var(10, "OtherSort"))',
        'pred(symbol("P", pred, [], bool), [])',
        'pred(symbol("Q", pred, ["A", "B"], bool), [var(0, "A"), var(1, "B")])',
        'forall "_q0": "User". eq(var(0, "User"), var(0, "User"))',
        'exists "_q0": "Db". forall "_q1": "User". eq(var(0, "User"), var(1, "Db"))',
        # Odd signatures:
        'pred(symbol("oddConst", const, ["S"], "ResultSort"), [var(0, "S")])',
        'pred(symbol("oddPred", pred, [], "NotABool"), [])',
    ],
)
def test_property_parse_print_roundtrip(formula_text: str) -> None:
    f1 = parse_formula(formula_text)
    printed = print_formula(f1)
    f2 = parse_formula(printed)
    assert f1 == f2, f"Roundtrip falhou para {formula_text}: impresso como {printed}"


def test_property_depth128_roundtrip() -> None:
    # Constrói fórmula com profundidade lógica exatamente 128 (127 Not aninhados sobre true)
    # Profundidade da raiz é 1, cada Not adiciona 1 -> 127 + 1 = 128
    current = "true"
    for _ in range(127):
        current = f"not({current})"
    f1 = parse_formula(current)
    recs = _recs(f1)
    assert len(recs) == 128
    printed = print_formula(f1)
    f2 = parse_formula(printed)
    assert f1 == f2


# ---------------------------------------------------------------------------
# 6. Vocabulário, Documentos, Referências e Invariantes
# ---------------------------------------------------------------------------


def test_document_roundtrip_with_all_variants() -> None:
    hex1 = "0" * 63 + "1"
    doc_text = f"""
    vocab v1 {{
      sort "User" = enum ["alice"] from fact "{hex1}"
      sort "Db" = enum [] from policy "ref_policy_alias"
      sort "Action"
      const alice: "User" = glossed("usuário Alice") gloss "Alice"
      func get_db("User"): "Db" = anchored("attestor1", "proc1") gloss "pega banco"
      pred Admin("User")
      pred Owns("User", "Db") = defined(forall u:"User". Admin(u)) gloss "admin é dono"
      decl symbol("extra", const, [], "Action") = glossed("") gloss ""
    }}
    claim "C1": forall u:"User". Admin(u) -> exists d:"Db". Owns(u, d) cites ["F3", "P2"]
    claim "C2": true
    """
    refs = {
        "ref_policy_alias": bytes.fromhex(
            "0000000000000000000000000000000000000000000000000000000000000002"
        )
    }
    doc1 = parse_document(doc_text, refs)
    printed = print_document(doc1)
    doc2 = parse_document(printed, refs)
    assert doc1 == doc2


def test_vocabulary_forward_defined_references() -> None:
    # Símbolo A é definido usando símbolo B, que foi declarado após A
    doc_text = """
    vocab v1 {
      sort S
      pred A(S) = defined(forall s:S. B(s))
      pred B(S)
    }
    """
    doc = parse_document(doc_text)
    symbols = _symbols(doc.vocabulary)
    assert len(symbols) == 2


def test_vocabulary_rejects_duplicate_sort_name() -> None:
    doc_text = "vocab v1 { sort S; sort S; }"
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_text)
    assert exc_info.value.kind == "DuplicateName"


def test_vocabulary_rejects_duplicate_symbol_name() -> None:
    doc_text = "vocab v1 { sort S; pred P(S); pred P(S); }"
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_text)
    assert exc_info.value.kind == "DuplicateName"


def test_vocabulary_rejects_duplicate_claim_name() -> None:
    doc_text = "vocab v1 {} claim C1: true; claim C1: false;"
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_text)
    assert exc_info.value.kind == "DuplicateName"


def test_vocabulary_rejects_undeclared_sort_in_symbol() -> None:
    doc_text = "vocab v1 { pred P(UndeclaredSort); }"
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_text)
    assert exc_info.value.kind == "UnknownName"


def test_vocabulary_rejects_undeclared_sort_in_defined() -> None:
    doc_text = """
    vocab v1 {
      sort S
      pred P(S) = defined(forall x:UndeclaredSort. true)
    }
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_text)
    assert exc_info.value.kind == "UnknownName"


def test_vocabulary_rejects_undeclared_symbol_in_defined() -> None:
    doc_text = """
    vocab v1 {
      sort S
      pred P(S) = defined(forall s:S. UndeclaredSym(s))
    }
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_text)
    assert exc_info.value.kind == "UnknownName"


def test_vocabulary_rejects_signature_mismatch_in_defined() -> None:
    # Declara P com aridade [S], mas usa explicitamente com aridade diferente no corpo
    doc_text = """
    vocab v1 {
      sort S
      pred P(S)
      pred Q(S) = defined(pred(symbol("P", pred, [], bool), []))
    }
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_text)
    assert exc_info.value.kind == "InvalidVocabulary"


def test_vocabulary_rejects_circular_definition() -> None:
    # Auto-referência direta
    doc_self = """
    vocab v1 {
      sort S
      pred P(S) = defined(forall s:S. P(s))
    }
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_self)
    assert exc_info.value.kind == "InvalidVocabulary"

    # Ciclo mútuo: A -> B -> A
    doc_cycle = """
    vocab v1 {
      sort S
      pred A(S) = defined(forall s:S. B(s))
      pred B(S) = defined(forall s:S. A(s))
    }
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_cycle)
    assert exc_info.value.kind == "InvalidVocabulary"


def test_vocabulary_rejects_invalid_enum_constant() -> None:
    # Constante não declarada
    hex1 = "0" * 63 + "1"
    doc_unknown = f"""
    vocab v1 {{
      sort User = enum [alice] from fact "{hex1}"
    }}
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_unknown)
    assert exc_info.value.kind == "UnknownName"

    # Constante com sort diferente
    hex1 = "0" * 63 + "1"
    doc_wrong_sort = f"""
    vocab v1 {{
      sort User = enum [item] from fact "{hex1}"
      sort Other
      const item: Other
    }}
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_wrong_sort)
    assert exc_info.value.kind == "InvalidVocabulary"

    # Constante duplicada no enum
    hex1 = "0" * 63 + "1"
    doc_dup_const = f"""
    vocab v1 {{
      sort User = enum [alice, alice] from fact "{hex1}"
      const alice: User
    }}
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_dup_const)
    assert exc_info.value.kind == "InvalidVocabulary"


def test_vocabulary_rejects_missing_reference_alias() -> None:
    doc_text = """
    vocab v1 {
      sort User = enum [] from fact missing_alias
    }
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(doc_text, references={})
    assert exc_info.value.kind == "UnknownReference"


# ---------------------------------------------------------------------------
# 7. Limites Estruturais e Fronteiras de Erro
# ---------------------------------------------------------------------------


def test_limits_depth_exceeded() -> None:
    # 128 Not aninhados sobre true -> profundidade 1 + 128 = 129 (> 128)
    current = "true"
    for _ in range(128):
        current = f"not({current})"
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(current)
    assert exc_info.value.kind == "LimitExceeded"


def test_transparent_parentheses_success() -> None:
    # 200 parênteses transparentes devem ser aceitos sob a especificação
    nested = "(" * 200 + "true" + ")" * 200
    res = parse_formula(nested)
    assert _recs(res)[0] == [0]


def test_limits_excessive_textual_nesting_no_recursion_error() -> None:
    # 10.000 parênteses deve falhar com LimitExceeded, nunca RecursionError
    nested = "(" * 10000 + "true" + ")" * 10000
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(nested)
    assert exc_info.value.kind == "LimitExceeded"


def test_printer_rejects_invalid_ast() -> None:
    # Envelope inválido passado a print_formula
    with pytest.raises(SurfaceError) as exc_info:
        bad_envelope: Value = [1, 2, "WrongTag", []]
        print_formula(bad_envelope)
    assert exc_info.value.kind == "InvalidAst"

    # Prefixo incompleto (And com 2 filhos prometidos mas apenas 1 fornecido)
    incomplete_formula: Value = [1, 3, "FOL-ms-eq/v1", [[5, 2], [0]]]
    with pytest.raises(SurfaceError) as exc_info:
        print_formula(incomplete_formula)
    assert exc_info.value.kind == "InvalidAst"


def test_nested_grouping_and_infix_semantics() -> None:
    # ((true) and false) produz and(true, false) com nó binário exato
    f1 = "((true) and false)"
    res1 = parse_formula(f1)
    assert res1 == [1, 3, "FOL-ms-eq/v1", [[5, 2], [0], [1]]]

    # ((true -> false) <-> true) -> <->(->(true, false), true)
    f2 = "((true -> false) <-> true)"
    res2 = parse_formula(f2)
    assert res2 == [1, 3, "FOL-ms-eq/v1", [[8], [7], [0], [1], [0]]]

    # (true <-> (false <-> true))
    f3 = "(true <-> (false <-> true))"
    res3 = parse_formula(f3)
    assert res3 == [1, 3, "FOL-ms-eq/v1", [[8], [0], [8], [1], [0]]]

    # ((true and false) or (false and true))
    f4 = "((true and false) or (false and true))"
    res4 = parse_formula(f4)
    assert res4 == [1, 3, "FOL-ms-eq/v1", [[6, 2], [5, 2], [0], [1], [5, 2], [1], [0]]]

    # (((true)))
    f5 = "(((true)))"
    res5 = parse_formula(f5)
    assert res5 == [1, 3, "FOL-ms-eq/v1", [[0]]]


def test_nominal_sibling_and_quantifier_scope() -> None:
    # Vocabulário com sort S e predicado P(S)
    doc_src = """
    vocab v1 {
      sort S
      pred P(S)
    }
    claim C1: and([forall x:S. P(x), forall y:S. P(y)])
    claim C2: and([forall x:S. P(x), forall x:S. P(x)])
    claim C3: forall x:S. exists x:S. P(x)
    claim C4: forall x:S. exists y:S. P(x)
    """
    doc = parse_document(doc_src)
    sort_s: Value = [1, 0, "S"]
    sym_p: Value = [1, 1, "P", 2, [sort_s], [1]]

    # C1: x no primeiro forall é índice 0, y no segundo forall é índice 0 (escopo restaurado)
    f1 = doc.claims[0].formula
    t0: Value = [1, 2, "FOL-ms-eq/v1", [[0, 0, sort_s]]]
    atom0: Value = [3, sym_p, [t0]]
    expected_c1: Value = [
        1,
        3,
        "FOL-ms-eq/v1",
        [[5, 2], [9, sort_s], atom0, [9, sort_s], atom0],
    ]
    assert f1 == expected_c1

    # C2: ambos usam nome x, ambos resolvem para índice 0
    f2 = doc.claims[1].formula
    assert f2 == expected_c1

    # C3: shadowing - x interno resolve para índice 0
    f3 = doc.claims[2].formula
    expected_c3: Value = [1, 3, "FOL-ms-eq/v1", [[9, sort_s], [10, sort_s], atom0]]
    assert f3 == expected_c3

    # C4: sem shadowing - x externo resolve para índice 1
    t1: Value = [1, 2, "FOL-ms-eq/v1", [[0, 1, sort_s]]]
    atom1: Value = [3, sym_p, [t1]]
    f4 = doc.claims[3].formula
    expected_c4: Value = [1, 3, "FOL-ms-eq/v1", [[9, sort_s], [10, sort_s], atom1]]
    assert f4 == expected_c4

    # Variável fora de escopo após fechamento do grupo do quantifier
    bad_scope = """
    vocab v1 {
      sort S
      pred P(S)
    }
    claim Bad: (forall x:S. P(x)) and P(x)
    """
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(bad_scope)
    assert exc_info.value.kind == "UnknownName"


def test_exact_logical_depth_128_pass_129_reject() -> None:
    # 1. Not chain: 127 'not' sobre true -> profundidade 1 + 127 = 128 (passa!)
    not127 = "not(" * 127 + "true" + ")" * 127
    res_not = parse_formula(not127)
    assert len(_recs(res_not)) == 128

    # 128 'not' sobre true -> profundidade 1 + 128 = 129 (rejeita!)
    not128 = "not(" * 128 + "true" + ")" * 128
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(not128)
    assert exc_info.value.kind == "LimitExceeded"

    # 2. Quantifier chain: 127 forall sobre true -> profundidade 128 (passa!)
    q127 = "".join(f'forall "x{i}": "S". ' for i in range(127)) + "true"
    res_q = parse_formula(q127)
    assert len(_recs(res_q)) == 128

    # 128 forall sobre true -> profundidade 129 (rejeita!)
    q128 = "".join(f'forall "x{i}": "S". ' for i in range(128)) + "true"
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(q128)
    assert exc_info.value.kind == "LimitExceeded"

    # 3. Embedded App chain em vocabulário:
    # c é const (depth 1). f é func(S): S. P é pred(S).
    # P(f(...(c)...)):
    # 126 f's -> term depth 127, P formula depth 128 (passa!)
    doc_prefix = """
    vocab v1 {
      sort S
      const c: S
      func f(S): S
      pred P(S)
    }
    """
    app126_term = "f(" * 126 + "c" + ")" * 126
    claim_pass = doc_prefix + f"claim C: P({app126_term})"
    doc_p = parse_document(claim_pass)
    assert len(doc_p.claims) == 1

    # 127 f's -> term depth 128, P formula depth 129 (rejeita!)
    app127_term = "f(" * 127 + "c" + ")" * 127
    claim_fail = doc_prefix + f"claim C: P({app127_term})"
    with pytest.raises(SurfaceError) as exc_info:
        parse_document(claim_fail)
    assert exc_info.value.kind == "LimitExceeded"


def test_transparent_groups_200_and_500_pass() -> None:
    # 200 parênteses transparentes
    res200 = parse_formula("(" * 200 + "true" + ")" * 200)
    assert _recs(res200)[0] == [0]

    # 500 parênteses transparentes
    res500 = parse_formula("(" * 500 + "true" + ")" * 500)
    assert _recs(res500)[0] == [0]


def test_adversarial_nesting_10000_controlled_limit_exceeded() -> None:
    # 10.000 agrupamentos: LimitExceeded controlado, sem RecursionError
    nested10k = "(" * 10000 + "true" + ")" * 10000
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(nested10k)
    assert exc_info.value.kind == "LimitExceeded"

    # 10.000 cadeia right-associative (->): LimitExceeded controlado
    right_imp10k = "true -> " * 10000 + "true"
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(right_imp10k)
    assert exc_info.value.kind == "LimitExceeded"

    # 10.000 cadeia not: LimitExceeded controlado
    not10k = "not(" * 10000 + "true" + ")" * 10000
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(not10k)
    assert exc_info.value.kind == "LimitExceeded"

    # 10.000 prefix not: LimitExceeded controlado
    prefix_not10k = "not " * 10000 + "true"
    with pytest.raises(SurfaceError) as exc_info:
        parse_formula(prefix_not10k)
    assert exc_info.value.kind == "LimitExceeded"
