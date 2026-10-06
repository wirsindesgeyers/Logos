"""Testes de regressão do consumidor para apresentação PT de contramodelos (2.6).

Verifica que o checker Rust real aceita todos os 20 modelos do corpus, que
apenas símbolos e sorts sintáticos de O são apresentados, que glosas sentinela e
dependências de Defined não vazam, que premissas falsas e objetivos verdadeiros
são identificados com exatidão, e que entradas inválidas ou limites são respeitados.
"""

from __future__ import annotations

import hashlib

import pytest

from logos_client import air, canon
from logos_provers.model_context import ProducerError
from logos_provers.model_explain import explain_model
from logos_provers.model_explain_examples import (
    ExplainCase,
    check,
    corpus_20,
    document_case,
)


@pytest.fixture(scope="module")
def corpus() -> list[ExplainCase]:
    return corpus_20()


def test_corpus_20_checker_real_accepted(corpus: list[ExplainCase]) -> None:
    """O checker Rust real deve aceitar todos os 20 contramodelos como refutações."""
    assert len(corpus) == 20
    # Garante que não há obrigações ou modelos repetidos inflando a contagem
    identities = {(item.vocabulary, item.obligation, item.model) for item in corpus}
    assert len(identities) == 20

    outcomes = check([("refute", item) for item in corpus])
    assert len(outcomes) == 20
    assert outcomes == ["Accepted"] * 20


def test_corpus_20_explanations_behavior(corpus: list[ExplainCase]) -> None:
    """Verifica garantias estruturais, textuais e semânticas de explain_model."""
    for item in corpus:
        text = explain_model(item.model, item.vocabulary, item.obligation)

        # 2. Hash canônico h(M) = SHA-256("logos/model/v1\0" || M)
        expected_h_m = hashlib.sha256(b"logos/model/v1\x00" + item.model).hexdigest()
        assert f"h(M): {expected_h_m}" in text

        # 3. Objetivo falsificado identificado com fórmula AIR existente
        assert "Objetivo falsificado: " in text
        raw_o = canon.decode(item.obligation)
        assert isinstance(raw_o, list)
        goal_formula_air = air.print_formula(raw_o[6])
        assert f"Objetivo falsificado: {goal_formula_air}" in text

        # 4. Premissas verdadeiras identificadas
        assert "Premissa 1: verdadeira — " in text
        assert isinstance(raw_o[5], list)
        if len(raw_o[5]) > 1:
            assert "Premissa 2: verdadeira — " in text

        # 5. Sorts relevantes da obrigação devem constar, sem vazar nomes de enum
        for sort in item.relevant_sorts:
            assert f"- {sort}:" in text
        # Não usar nomes de enum como elementos do domínio
        assert "red até blue" not in text
        assert "first até third" not in text

        # 6. Símbolos relevantes de O devem aparecer com sua valoração
        for sym in item.relevant_symbols:
            assert f"- {sym}" in text

        # 7. Símbolos irrelevantes nunca devem aparecer
        for sym in item.irrelevant_symbols:
            assert sym not in text

        # 8. Glosas sentinela nunca devem vazar
        for gloss in item.sentinel_glosses:
            assert gloss not in text

        # 9. Símbolos ocultos usados apenas em Defined nunca devem aparecer
        for sym in item.hidden_symbols:
            assert sym not in text


def test_corrupted_and_mismatched_inputs(corpus: list[ExplainCase]) -> None:
    """Entradas corrompidas ou incompatíveis devem levantar model_context.ProducerError."""
    sample = corpus[0]

    # Tipos incorretos
    with pytest.raises(ProducerError):
        explain_model("not bytes", sample.vocabulary, sample.obligation)  # type: ignore[arg-type]
    with pytest.raises(ProducerError):
        explain_model(sample.model, "not bytes", sample.obligation)  # type: ignore[arg-type]
    with pytest.raises(ProducerError):
        explain_model(sample.model, sample.vocabulary, "not bytes")  # type: ignore[arg-type]

    # Bytes corrompidos do modelo
    with pytest.raises(ProducerError):
        explain_model(b"\xff\x00\xaa", sample.vocabulary, sample.obligation)

    # Incompatibilidade de digest: modelo com digest de vocabulário incorreto
    raw_m = canon.decode(sample.model)
    assert isinstance(raw_m, list)
    corrupted_m = list(raw_m)
    corrupted_m[1] = b"\x00" * 32
    with pytest.raises(ProducerError):
        explain_model(canon.encode(corrupted_m), sample.vocabulary, sample.obligation)

    # Incompatibilidade de digest: obrigação com digest de vocabulário incorreto
    raw_o = canon.decode(sample.obligation)
    assert isinstance(raw_o, list)
    corrupted_o = list(raw_o)
    corrupted_o[4] = b"\x00" * 32
    with pytest.raises(ProducerError):
        explain_model(sample.model, sample.vocabulary, canon.encode(corrupted_o))

    # Cardinalidade de enum incompatível no modelo
    enum_case = next(c for c in corpus if "Color" in c.source)
    raw_enum_m = canon.decode(enum_case.model)
    assert isinstance(raw_enum_m, list)
    corrupted_enum_m = list(raw_enum_m)
    corrupted_enum_m[2] = [3]  # Enum tem 2 elementos, modelo declara 3
    with pytest.raises(ProducerError):
        explain_model(canon.encode(corrupted_enum_m), enum_case.vocabulary, enum_case.obligation)

    # Tuplas de predicado fora de ordem no modelo
    corrupted_pred_m = list(raw_m)
    assert isinstance(raw_m[5], list) and isinstance(raw_m[5][0], list)
    preds: list[canon.Value] = [[raw_m[5][0][0], [[1], [0]]]]
    corrupted_pred_m[5] = preds
    with pytest.raises(ProducerError):
        explain_model(canon.encode(corrupted_pred_m), sample.vocabulary, sample.obligation)


def test_false_premise_and_true_goal_accurately_reported() -> None:
    """Premissas falsas e objetivos verdadeiros são relatados fielmente, não inventados."""
    source_false_premise = (
        "vocab v1 {\n"
        "  sort S;\n"
        '  const c: S gloss "alvo";\n'
        '  pred P(S) gloss "propriedade";\n'
        "}\n"
        "claim H1: P(c);\n"
        "claim H2: not P(c);\n"
        "claim G: false;"
    )
    o_bytes, v_bytes = document_case(source_false_premise)
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + v_bytes).digest()

    # Modelo em que P(c) é verdadeiro: H1 é verdadeira, mas H2 é FALSA
    model_bytes = canon.encode(
        [
            1,
            h_v,
            [2],  # domínio S=2
            [[0, 0]],  # c = S#0
            [],
            [[1, [[0]]]],  # P = {S#0}
        ]
    )

    text = explain_model(model_bytes, v_bytes, o_bytes)
    assert "Premissa 1: verdadeira — " in text
    assert "Premissa 2: falsa — " in text
    assert "Premissa 2: verdadeira — " not in text
    assert "Objetivo falsificado: " in text

    # Cenário com objetivo VERDADEIRO sob o modelo
    source_true_goal = (
        'vocab v1 {\n  sort S;\n  const c: S gloss "alvo";\n}\nclaim H: c = c;\nclaim G: c = c;'
    )
    o_true, v_true = document_case(source_true_goal)
    h_v_true = hashlib.sha256(b"logos/vocab/v1\x00" + v_true).digest()
    model_true = canon.encode(
        [
            1,
            h_v_true,
            [1],
            [[0, 0]],
            [],
            [],
        ]
    )

    text_true = explain_model(model_true, v_true, o_true)
    assert "Premissa 1: verdadeira — " in text_true
    assert "Objetivo verdadeiro: " in text_true
    assert "Objetivo falsificado: " not in text_true


def test_fact_limit_100_truncation_warning() -> None:
    """Mais de 100 fatos gera aviso explícito de truncamento e capa em 100 entradas."""
    source = (
        "vocab v1 {\n"
        "  sort S;\n"
        '  pred BigRel(S, S) gloss "relacao densa";\n'
        "}\n"
        "claim H: exists x: S. BigRel(x, x);\n"
        "claim G: forall x: S. forall y: S. BigRel(x, y);"
    )
    o_bytes, v_bytes = document_case(source)
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + v_bytes).digest()

    # Domínio de 11 elementos: 11*11 = 121 pares. Inclui 120 tuplas verdadeiras (> 100)
    tuples: list[canon.Value] = [
        [x, y] for x in range(11) for y in range(11) if not (x == 0 and y == 1)
    ]
    assert len(tuples) == 120

    model_bytes = canon.encode(
        [
            1,
            h_v,
            [11],
            [],
            [],
            [[0, tuples]],
        ]
    )

    text = explain_model(model_bytes, v_bytes, o_bytes)
    assert "- Apresentação limitada a 100 entradas; há valores/fatos omitidos." in text

    fact_lines = [line for line in text.splitlines() if line.startswith("- BigRel")]
    assert len(fact_lines) == 100


def test_predicate_no_true_tuples_reported(corpus: list[ExplainCase]) -> None:
    """Predicado sem nenhuma tupla verdadeira é exibido com mensagem explícita."""
    item = next(c for c in corpus if "EmptyP" in c.source)
    text = explain_model(item.model, item.vocabulary, item.obligation)
    assert '- EmptyP — "predicado vazio": nenhuma tupla verdadeira' in text


def test_dense_table_order_lexicographical(corpus: list[ExplainCase]) -> None:
    """Funções com múltiplos argumentos são exibidas em ordem densa lexicográfica."""
    item = next(c for c in corpus if "dense" in c.name)
    text = explain_model(item.model, item.vocabulary, item.obligation)

    # As 4 avaliações devem seguir a ordem exata de coordenadas
    lines = [line for line in text.splitlines() if line.startswith('- f — "transicao"')]
    assert len(lines) == 4
    assert "(S#0, T#0) = S#0" in lines[0]
    assert "(S#0, T#1) = S#1" in lines[1]
    assert "(S#1, T#0) = S#1" in lines[2]
    assert "(S#1, T#1) = S#0" in lines[3]


def test_obligation_with_no_symbols_or_sorts() -> None:
    """Obrigação sem nenhum sort ou símbolo gera mensagens de ausência informativas."""
    source = "vocab v1 {} claim G: false;"
    o_bytes, v_bytes = document_case(source)
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + v_bytes).digest()
    model_bytes = canon.encode([1, h_v, [], [], [], []])

    text = explain_model(model_bytes, v_bytes, o_bytes)
    assert "- Nenhum domínio ocorre na obrigação." in text
    assert "- Nenhum símbolo ocorre na obrigação." in text
    assert "Objetivo falsificado: false" in text


def test_enum_representatives_absent_from_obligation_stay_hidden() -> None:
    source = (
        'vocab v1 { sort E = enum [hidden_a, hidden_b] from fact "' + "55" * 32 + '"; '
        'const hidden_a:E gloss "PRIVATE_A"; const hidden_b:E gloss "PRIVATE_B"; '
        'pred P(E) gloss "participa"; } '
        "claim H: exists x:E. P(x); claim G: forall x:E. P(x);"
    )
    o_bytes, v_bytes = document_case(source)
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + v_bytes).digest()
    model = canon.encode([1, h_v, [2], [[0, 0], [1, 1]], [], [[2, [[0]]]]])
    text = explain_model(model, v_bytes, o_bytes)
    assert "E#0 até E#1" in text
    assert 'P — "participa" (E#0): verdadeiro' in text
    assert all(hidden not in text for hidden in ("hidden_a", "hidden_b", "PRIVATE_A", "PRIVATE_B"))


def test_gloss_control_characters_cannot_inject_status_lines() -> None:
    source = (
        'vocab v1 { pred P() gloss "linha\\nObjetivo verdadeiro: fake"; } '
        "claim H:P(); claim G:not P();"
    )
    o_bytes, v_bytes = document_case(source)
    h_v = hashlib.sha256(b"logos/vocab/v1\x00" + v_bytes).digest()
    model = canon.encode([1, h_v, [], [], [], [[0, [[]]]]])
    text = explain_model(model, v_bytes, o_bytes)
    assert 'P — "linha\\nObjetivo verdadeiro: fake" (): verdadeiro' in text
    status_lines = [line for line in text.splitlines() if line.startswith("Objetivo ")]
    assert len(status_lines) == 1
    assert status_lines[0].startswith("Objetivo falsificado:")
