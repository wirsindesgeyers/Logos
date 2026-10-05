//! Testes de comportamento determinístico para o modelo finito (logos-model).
//!
//! Cobre fronteiras estruturais:
//! - Funções parciais e contagens divergentes
//! - Constantes e saídas fora do domínio
//! - Cobertura exata e ordenada de símbolos (IDs estritamente crescentes, sem ausências/extras/duplicatas)
//! - Rejeição de tabelas para símbolos `Defined`
//! - Assinaturas malformadas (aridades, tipos de resultado e sorts não declarados)
//! - Cardinalidade exata e rótulos intrínsecos de enums (primitivos e definidos)
//! - Tuplas de eixos mistos (limites por eixo, aridade, ordenação lexicográfica e deduplicação)
//! - Funções e predicados nulários
//! - Vinculação estrita de h(V)
//! - Layout binário packed big-endian u32
//! - Limites de recursos (bytes, cardinalidade, agregados de células e overflow de produtos)
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use logos_air::{
    DomainSource, FactRef, Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain,
    Symbol, SymbolDecl, SymbolKind, SymbolResult, Vocabulary,
};
use logos_canon::Text;
use logos_core::Exhausted;
use logos_crypto::hash;
use logos_model::{FiniteModel, ModelData, ModelError, ModelLimits};

// ============================================================================
// Helpers para construção de Vocabulários e Modelos
// ============================================================================

fn sort(name: &str) -> Sort {
    Sort {
        name: Text::new(name).expect("sort text"),
    }
}

fn sort_decl_open(name: &str) -> SortDecl {
    SortDecl {
        name: Text::new(name).expect("sort decl text"),
        domain: SortDomain::Open,
    }
}

fn sort_decl_enum(name: &str, constants: Vec<Symbol>, fact_seed: u8) -> SortDecl {
    SortDecl {
        name: Text::new(name).expect("sort decl text"),
        domain: SortDomain::Enumerated {
            constants,
            domain_source: DomainSource::Fact(FactRef([fact_seed; 32])),
        },
    }
}

fn const_sym(name: &str, sort_name: &str) -> Symbol {
    Symbol {
        name: Text::new(name).expect("symbol text"),
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort(sort_name)),
    }
}

fn func_sym(name: &str, arity: Vec<Sort>, result_sort: &str) -> Symbol {
    Symbol {
        name: Text::new(name).expect("symbol text"),
        kind: SymbolKind::Func,
        arity,
        result: SymbolResult::Sort(sort(result_sort)),
    }
}

fn pred_sym(name: &str, arity: Vec<Sort>) -> Symbol {
    Symbol {
        name: Text::new(name).expect("symbol text"),
        kind: SymbolKind::Pred,
        arity,
        result: SymbolResult::Bool,
    }
}

fn primitive(sig: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature: sig,
        grounding: Grounding::Glossed(Text::new("primitivo").expect("gloss")),
        gloss_pt: Text::new("glosa").expect("gloss_pt"),
    }
}

fn defined(sig: Symbol) -> SymbolDecl {
    SymbolDecl {
        signature: sig,
        grounding: Grounding::Defined(
            Formula::new(vec![FormulaNode::True], Limits::default()).expect("formula"),
        ),
        gloss_pt: Text::new("definido").expect("gloss_pt"),
    }
}

// ============================================================================
// 1. Cobertura exata e ordenada de símbolos (IDs estritamente crescentes)
// ============================================================================

#[test]
fn test_sorted_exact_symbol_coverage() {
    let s = sort("S");
    let c0 = const_sym("c0", "S"); // id 0
    let c1 = const_sym("c1", "S"); // id 1
    let f = func_sym("f", vec![s.clone()], "S"); // id 2
    let p = pred_sym("p", vec![s.clone()]); // id 3

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c0), primitive(c1), primitive(f), primitive(p)],
    )
    .expect("valid vocab");
    let h_v = hash(&vocab).expect("vocab hash");
    let limits = ModelLimits::default();

    // Modelo perfeitamente válido
    let valid_data = ModelData {
        vocab: h_v,
        domains: vec![2],
        consts: vec![(0, 0), (1, 1)],
        funcs: vec![(2, vec![0, 1])],
        preds: vec![(3, vec![vec![0]])],
    };
    assert!(FiniteModel::new(valid_data.clone(), &vocab, limits).is_ok());

    // 1. Constante faltante (apenas c0 fornecido, c1 faltando)
    let mut data = valid_data.clone();
    data.consts = vec![(0, 0)];
    assert_eq!(
        FiniteModel::new(data, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "constante não-Defined faltante deve retornar InvalidInterpretation"
    );

    // 2. Função faltante
    let mut data = valid_data.clone();
    data.funcs = Vec::new();
    assert_eq!(
        FiniteModel::new(data, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "função faltante deve retornar InvalidInterpretation"
    );

    // 3. Predicado faltante
    let mut data = valid_data.clone();
    data.preds = Vec::new();
    assert_eq!(
        FiniteModel::new(data, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "predicado faltante deve retornar InvalidInterpretation"
    );

    // 4. Símbolo extra / ID inexistente
    let mut data = valid_data.clone();
    data.consts = vec![(0, 0), (1, 1), (4, 0)];
    assert_eq!(
        FiniteModel::new(data, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "ID além dos declarados deve retornar InvalidInterpretation"
    );

    // 5. IDs duplicados em consts
    let mut data = valid_data.clone();
    data.consts = vec![(0, 0), (0, 1)];
    assert_eq!(
        FiniteModel::new(data, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "ID duplicado em consts deve retornar InvalidInterpretation"
    );

    // 6. IDs fora de ordem crescente em consts
    let mut data = valid_data.clone();
    data.consts = vec![(1, 1), (0, 0)];
    assert_eq!(
        FiniteModel::new(data, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "IDs não estritamente crescentes em consts deve retornar InvalidInterpretation"
    );

    // 7. Tipo trocado: ID de função colocado em consts
    let mut data = valid_data.clone();
    data.consts = vec![(0, 0), (1, 1), (2, 0)];
    data.funcs = Vec::new();
    assert_eq!(
        FiniteModel::new(data, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "função colocada em consts deve retornar InvalidInterpretation"
    );

    // 8. Tipo trocado: ID de constante colocado em funcs
    let mut data = valid_data.clone();
    data.consts = vec![(1, 1)];
    data.funcs = vec![(0, vec![0, 1]), (2, vec![0, 1])];
    assert_eq!(
        FiniteModel::new(data, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "constante colocada em funcs deve retornar InvalidInterpretation"
    );
}

// ============================================================================
// 2. Rejeição de tabela para símbolos Defined
// ============================================================================

#[test]
fn test_defined_table_rejection() {
    let s = sort("S");
    let c_prim = const_sym("c_prim", "S"); // id 0
    let c_def = const_sym("c_def", "S"); // id 1
    let f_def = func_sym("f_def", vec![s.clone()], "S"); // id 2
    let p_def = pred_sym("p_def", vec![s.clone()]); // id 3

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![
            primitive(c_prim),
            defined(c_def),
            defined(f_def),
            defined(p_def),
        ],
    )
    .expect("valid vocab");
    let h_v = hash(&vocab).expect("vocab hash");
    let limits = ModelLimits::default();

    // Modelo válido: APENAS c_prim tem tabela; símbolos definidos NÃO têm tabelas
    let valid_data = ModelData {
        vocab: h_v,
        domains: vec![2],
        consts: vec![(0, 1)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    assert!(FiniteModel::new(valid_data.clone(), &vocab, limits).is_ok());

    // Injeção de tabela para constante Defined
    let mut bad_const = valid_data.clone();
    bad_const.consts = vec![(0, 1), (1, 0)];
    assert_eq!(
        FiniteModel::new(bad_const, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "constante Defined não deve ter tabela"
    );

    // Injeção de tabela para função Defined
    let mut bad_func = valid_data.clone();
    bad_func.funcs = vec![(2, vec![0, 1])];
    assert_eq!(
        FiniteModel::new(bad_func, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "função Defined não deve ter tabela"
    );

    // Injeção de tabela para predicado Defined
    let mut bad_pred = valid_data.clone();
    bad_pred.preds = vec![(3, vec![vec![0]])];
    assert_eq!(
        FiniteModel::new(bad_pred, &vocab, limits),
        Err(ModelError::InvalidInterpretation),
        "predicado Defined não deve ter tabela"
    );
}

// ============================================================================
// 3. Assinaturas malformadas
// ============================================================================

#[test]
fn test_malformed_signature() {
    let s = sort("S");
    let limits = ModelLimits::default();

    // 1. Constante com aridade > 0
    let bad_const = Symbol {
        name: Text::new("bad_c").expect("text"),
        kind: SymbolKind::Const,
        arity: vec![s.clone()],
        result: SymbolResult::Sort(s.clone()),
    };
    let v1 = Vocabulary::new(1, vec![sort_decl_open("S")], vec![primitive(bad_const)])
        .expect("vocabulary aceita sintaticamente");
    let h_v1 = hash(&v1).expect("hash");
    let d1 = ModelData {
        vocab: h_v1,
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(d1, &v1, limits),
        Err(ModelError::InvalidSignature),
        "constante com aridade > 0 deve retornar InvalidSignature"
    );

    // 2. Constante com resultado Bool
    let bad_const_bool = Symbol {
        name: Text::new("bad_cb").expect("text"),
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Bool,
    };
    let v2 = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(bad_const_bool)],
    )
    .expect("vocabulary aceita sintaticamente");
    let h_v2 = hash(&v2).expect("hash");
    let d2 = ModelData {
        vocab: h_v2,
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(d2, &v2, limits),
        Err(ModelError::InvalidSignature),
        "constante com resultado Bool deve retornar InvalidSignature"
    );

    // 3. Função com resultado Bool
    let bad_func_bool = Symbol {
        name: Text::new("bad_fb").expect("text"),
        kind: SymbolKind::Func,
        arity: vec![s.clone()],
        result: SymbolResult::Bool,
    };
    let v3 = Vocabulary::new(1, vec![sort_decl_open("S")], vec![primitive(bad_func_bool)])
        .expect("vocabulary aceita sintaticamente");
    let h_v3 = hash(&v3).expect("hash");
    let d3 = ModelData {
        vocab: h_v3,
        domains: vec![2],
        consts: Vec::new(),
        funcs: vec![(0, vec![0, 1])],
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(d3, &v3, limits),
        Err(ModelError::InvalidSignature),
        "função com resultado Bool deve retornar InvalidSignature"
    );

    // 4. Predicado com resultado Sort
    let bad_pred_sort = Symbol {
        name: Text::new("bad_ps").expect("text"),
        kind: SymbolKind::Pred,
        arity: vec![s.clone()],
        result: SymbolResult::Sort(s.clone()),
    };
    let v4 = Vocabulary::new(1, vec![sort_decl_open("S")], vec![primitive(bad_pred_sort)])
        .expect("vocabulary aceita sintaticamente");
    let h_v4 = hash(&v4).expect("hash");
    let d4 = ModelData {
        vocab: h_v4,
        domains: vec![2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: vec![(0, vec![vec![0]])],
    };
    assert_eq!(
        FiniteModel::new(d4, &v4, limits),
        Err(ModelError::InvalidSignature),
        "predicado com resultado Sort deve retornar InvalidSignature"
    );
}

// ============================================================================
// 4. Cardinalidade exata e rótulos de Enumeração (primitivos e Defined)
// ============================================================================

#[test]
fn test_exact_enum_cardinality_and_labels() {
    let a = const_sym("a", "Color");
    let b = const_sym("b", "Color");
    let c = const_sym("c", "Color");

    let sort_decl = sort_decl_enum("Color", vec![a.clone(), b.clone(), c.clone()], 0x42);
    // Declaramos no vocabulário em ordem permutada: b (0), a (1), c (2)
    let vocab = Vocabulary::new(
        1,
        vec![sort_decl],
        vec![primitive(b), primitive(a), primitive(c)],
    )
    .expect("vocab");
    let h_v = hash(&vocab).expect("hash");
    let limits = ModelLimits::default();

    // Na declaração do sort: a é 0, b é 1, c é 2.
    // No vocabulário: b é id 0, a é id 1, c é id 2.
    // Portanto:
    // b (id 0) deve denotar 1.
    // a (id 1) deve denotar 0.
    // c (id 2) deve denotar 2.
    let valid_data = ModelData {
        vocab: h_v,
        domains: vec![3],
        consts: vec![(0, 1), (1, 0), (2, 2)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    assert!(
        FiniteModel::new(valid_data.clone(), &vocab, limits).is_ok(),
        "rótulos mapeando corretamente para o índice da declaração do enum devem ser aceitos"
    );

    // 1. Cardinalidade do domínio menor que o número de constantes declaradas
    let mut bad_card_small = valid_data.clone();
    bad_card_small.domains = vec![2];
    assert_eq!(
        FiniteModel::new(bad_card_small, &vocab, limits),
        Err(ModelError::InvalidEnumeration),
        "cardinalidade menor deve retornar InvalidEnumeration"
    );

    // 2. Cardinalidade do domínio maior que o número de constantes declaradas
    let mut bad_card_large = valid_data.clone();
    bad_card_large.domains = vec![4];
    assert_eq!(
        FiniteModel::new(bad_card_large, &vocab, limits),
        Err(ModelError::InvalidEnumeration),
        "cardinalidade maior deve retornar InvalidEnumeration"
    );

    // 3. Rótulos errados: se a denotar 1 e b denotar 0
    let mut bad_labels = valid_data;
    bad_labels.consts = vec![(0, 0), (1, 1), (2, 2)];
    assert_eq!(
        FiniteModel::new(bad_labels, &vocab, limits),
        Err(ModelError::InvalidEnumeration),
        "constante primitiva com rótulo divergente da sua posição deve retornar InvalidEnumeration"
    );

    // 4. Sort enumerado contendo constante Defined
    let def_sym = const_sym("def_c", "EnumDef");
    let prim_sym = const_sym("prim_c", "EnumDef");
    let sort_decl_def = sort_decl_enum("EnumDef", vec![prim_sym.clone(), def_sym.clone()], 0x99);
    let vocab_def = Vocabulary::new(
        1,
        vec![sort_decl_def],
        vec![primitive(prim_sym), defined(def_sym)],
    )
    .expect("vocab with defined enum constant");
    let h_v_def = hash(&vocab_def).expect("hash");

    // prim_c é id 0, denota 0. def_c é id 1 (Defined) e NÃO deve estar em consts!
    let valid_def_data = ModelData {
        vocab: h_v_def,
        domains: vec![2],
        consts: vec![(0, 0)],
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    assert!(
        FiniteModel::new(valid_def_data, &vocab_def, limits).is_ok(),
        "constante Defined em enum deve ser aceita sem entrada na tabela consts"
    );
}

// ============================================================================
// 5. Funções parciais e contagens de saídas divergentes
// ============================================================================

#[test]
fn test_partial_functions_and_mismatch() {
    let s = sort("S");
    let t = sort("T");
    let f = func_sym("f", vec![s.clone(), t.clone()], "S");

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S"), sort_decl_open("T")],
        vec![primitive(f)],
    )
    .expect("vocab");
    let h_v = hash(&vocab).expect("hash");
    let limits = ModelLimits::default();

    // 1. Função parcial (menos saídas que o produto)
    let partial_data = ModelData {
        vocab: h_v,
        domains: vec![2, 3],
        consts: Vec::new(),
        funcs: vec![(0, vec![0, 1, 0])], // apenas 3 saídas
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(partial_data, &vocab, limits),
        Err(ModelError::NonTotalFunction),
        "menos saídas que o produto deve retornar NonTotalFunction"
    );

    // 2. Função com saídas excedentes
    let extra_data = ModelData {
        vocab: h_v,
        domains: vec![2, 3],
        consts: Vec::new(),
        funcs: vec![(0, vec![0, 1, 0, 1, 0, 1, 0])], // 7 saídas
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(extra_data, &vocab, limits),
        Err(ModelError::NonTotalFunction),
        "mais saídas que o produto deve retornar NonTotalFunction"
    );
}

// ============================================================================
// 6. Constantes e saídas fora do domínio
// ============================================================================

#[test]
fn test_out_of_range_constants_and_outputs() {
    let s = sort("S");
    let c = const_sym("c", "S");
    let f = func_sym("f", vec![s.clone()], "S");
    let p = pred_sym("p", vec![s.clone()]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(c), primitive(f), primitive(p)],
    )
    .expect("vocab");
    let h_v = hash(&vocab).expect("hash");
    let limits = ModelLimits::default();

    // |S| = 2. Valores válidos em S são {0, 1}.
    let valid_data = ModelData {
        vocab: h_v,
        domains: vec![2],
        consts: vec![(0, 1)],
        funcs: vec![(1, vec![0, 1])],
        preds: vec![(2, vec![vec![0], vec![1]])],
    };
    assert!(FiniteModel::new(valid_data.clone(), &vocab, limits).is_ok());

    // 1. Constante com valor 2 (>= 2)
    let mut bad_c = valid_data.clone();
    bad_c.consts = vec![(0, 2)];
    assert_eq!(
        FiniteModel::new(bad_c, &vocab, limits),
        Err(ModelError::ValueOutOfDomain),
        "constante com valor >= cardinalidade deve retornar ValueOutOfDomain"
    );

    // 2. Função com saída 2 (>= 2)
    let mut bad_f = valid_data.clone();
    bad_f.funcs = vec![(1, vec![0, 2])];
    assert_eq!(
        FiniteModel::new(bad_f, &vocab, limits),
        Err(ModelError::ValueOutOfDomain),
        "saída de função >= cardinalidade deve retornar ValueOutOfDomain"
    );

    // 3. Predicado com tupla contendo 2 (>= 2)
    let mut bad_p = valid_data;
    bad_p.preds = vec![(2, vec![vec![2]])];
    assert_eq!(
        FiniteModel::new(bad_p, &vocab, limits),
        Err(ModelError::ValueOutOfDomain),
        "tupla de predicado com valor >= cardinalidade deve retornar ValueOutOfDomain"
    );
}

// ============================================================================
// 7. Tuplas de eixos mistos (limites, aridade, ordenação e unicidade)
// ============================================================================

#[test]
fn test_mixed_axis_tuples() {
    let s = sort("S");
    let t = sort("T");
    let p = pred_sym("p", vec![t.clone(), s.clone()]); // aridade [T, S]

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S"), sort_decl_open("T")],
        vec![primitive(p)],
    )
    .expect("vocab");
    let h_v = hash(&vocab).expect("hash");
    let limits = ModelLimits::default();

    // domains: [|S|=2, |T|=3].
    // p tem eixos [T, S]: primeiro eixo T (0..3), segundo eixo S (0..2).
    let valid_data = ModelData {
        vocab: h_v,
        domains: vec![2, 3],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: vec![(0, vec![vec![0, 1], vec![2, 0]])],
    };
    assert!(FiniteModel::new(valid_data.clone(), &vocab, limits).is_ok());

    // 1. Eixo 0 (T) estoura o limite: [3, 0] onde |T|=3
    let mut bad_axis_0 = valid_data.clone();
    bad_axis_0.preds = vec![(0, vec![vec![3, 0]])];
    assert_eq!(
        FiniteModel::new(bad_axis_0, &vocab, limits),
        Err(ModelError::ValueOutOfDomain),
        "eixo T com valor >= 3 deve retornar ValueOutOfDomain"
    );

    // 2. Eixo 1 (S) estoura o limite: [0, 2] onde |S|=2
    let mut bad_axis_1 = valid_data.clone();
    bad_axis_1.preds = vec![(0, vec![vec![0, 2]])];
    assert_eq!(
        FiniteModel::new(bad_axis_1, &vocab, limits),
        Err(ModelError::ValueOutOfDomain),
        "eixo S com valor >= 2 deve retornar ValueOutOfDomain"
    );

    // 3. Aridade errada da tupla: tamanho 1 em vez de 2
    let mut bad_arity = valid_data.clone();
    bad_arity.preds = vec![(0, vec![vec![0]])];
    assert_eq!(
        FiniteModel::new(bad_arity, &vocab, limits),
        Err(ModelError::InvalidTuple),
        "tupla com aridade != pred.arity deve retornar InvalidTuple"
    );

    // 4. Tuplas duplicadas
    let mut bad_dup = valid_data.clone();
    bad_dup.preds = vec![(0, vec![vec![0, 1], vec![0, 1]])];
    assert_eq!(
        FiniteModel::new(bad_dup, &vocab, limits),
        Err(ModelError::InvalidTuple),
        "tuplas duplicadas devem retornar InvalidTuple"
    );

    // 5. Ordem lexicográfica invertida
    let mut bad_order = valid_data;
    bad_order.preds = vec![(0, vec![vec![2, 0], vec![0, 1]])];
    assert_eq!(
        FiniteModel::new(bad_order, &vocab, limits),
        Err(ModelError::InvalidTuple),
        "tuplas fora de ordem estritamente crescente devem retornar InvalidTuple"
    );
}

// ============================================================================
// 8. Funções e predicados nulários
// ============================================================================

#[test]
fn test_nullary_function_and_predicate() {
    let f0 = func_sym("f0", Vec::new(), "S"); // aridade []
    let p0 = pred_sym("p0", Vec::new()); // aridade []

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(f0), primitive(p0)],
    )
    .expect("vocab");
    let h_v = hash(&vocab).expect("hash");
    let limits = ModelLimits::default();

    // 1. Predicado nulário True: [[]]
    let true_data = ModelData {
        vocab: h_v,
        domains: vec![5],
        consts: Vec::new(),
        funcs: vec![(0, vec![3])],      // produto de aridade vazia = 1
        preds: vec![(1, vec![vec![]])], // true = [[]]
    };
    assert!(FiniteModel::new(true_data, &vocab, limits).is_ok());

    // 2. Predicado nulário False: []
    let false_data = ModelData {
        vocab: h_v,
        domains: vec![5],
        consts: Vec::new(),
        funcs: vec![(0, vec![3])],
        preds: vec![(1, vec![])], // false = []
    };
    assert!(FiniteModel::new(false_data, &vocab, limits).is_ok());

    // 3. Função nulária com contagem de saídas 0 (esperado 1)
    let bad_f0_empty = ModelData {
        vocab: h_v,
        domains: vec![5],
        consts: Vec::new(),
        funcs: vec![(0, vec![])],
        preds: vec![(1, vec![])],
    };
    assert_eq!(
        FiniteModel::new(bad_f0_empty, &vocab, limits),
        Err(ModelError::NonTotalFunction),
        "função nulária com 0 saídas deve retornar NonTotalFunction"
    );

    // 4. Função nulária com saída >= |S|
    let bad_f0_ood = ModelData {
        vocab: h_v,
        domains: vec![5],
        consts: Vec::new(),
        funcs: vec![(0, vec![5])], // 5 >= |S|=5
        preds: vec![(1, vec![])],
    };
    assert_eq!(
        FiniteModel::new(bad_f0_ood, &vocab, limits),
        Err(ModelError::ValueOutOfDomain),
        "função nulária com saída >= domínio deve retornar ValueOutOfDomain"
    );

    // 5. Predicado nulário com tupla não vazia
    let bad_p0_tuple = ModelData {
        vocab: h_v,
        domains: vec![5],
        consts: Vec::new(),
        funcs: vec![(0, vec![3])],
        preds: vec![(1, vec![vec![0]])], // tupla len 1 != 0
    };
    assert_eq!(
        FiniteModel::new(bad_p0_tuple, &vocab, limits),
        Err(ModelError::InvalidTuple),
        "tupla com aridade > 0 em predicado nulário deve retornar InvalidTuple"
    );

    // 6. Predicado nulário com tuplas duplicadas [ [], [] ]
    let bad_p0_dups = ModelData {
        vocab: h_v,
        domains: vec![5],
        consts: Vec::new(),
        funcs: vec![(0, vec![3])],
        preds: vec![(1, vec![vec![], vec![]])],
    };
    assert_eq!(
        FiniteModel::new(bad_p0_dups, &vocab, limits),
        Err(ModelError::InvalidTuple),
        "duplicata em predicado nulário deve retornar InvalidTuple"
    );
}

// ============================================================================
// 9. Vinculação estrita de h(V)
// ============================================================================

#[test]
fn test_hv_binding() {
    let vocab1 = Vocabulary::new(1, vec![sort_decl_open("S")], Vec::new()).expect("v1");
    let vocab2 = Vocabulary::new(2, vec![sort_decl_open("S")], Vec::new()).expect("v2");

    let h_v1 = hash(&vocab1).expect("hash1");
    let h_v2 = hash(&vocab2).expect("hash2");
    assert_ne!(h_v1, h_v2);

    let limits = ModelLimits::default();

    // 1. Constructor binds to actual crypto::hash(vocabulary), not arbitrary caller digest
    let data_with_wrong_hash = ModelData {
        vocab: h_v2,
        domains: vec![1],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(data_with_wrong_hash, &vocab1, limits),
        Err(ModelError::VocabularyMismatch),
        "h(V) divergente no construtor deve retornar VocabularyMismatch"
    );

    // 2. Decode checks wire hV == hash(vocabulary)
    let valid_data = ModelData {
        vocab: h_v1,
        domains: vec![1],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model1 = FiniteModel::new(valid_data, &vocab1, limits).expect("model1");
    let wire1 = model1.encode_with_limits(limits).expect("encode");

    // Decodificar wire1 contra vocab2 deve falhar com VocabularyMismatch
    assert_eq!(
        FiniteModel::decode_with_limits(&wire1, &vocab2, limits),
        Err(ModelError::VocabularyMismatch),
        "decodificar com vocabulário diferente do h(V) na wire deve retornar VocabularyMismatch"
    );
}

// ============================================================================
// 10. Layout packed big-endian u32 e wire canônica estrita
// ============================================================================

#[test]
fn test_strict_wire_packed_endian_layout() {
    let f = func_sym("f", Vec::new(), "S");

    let vocab = Vocabulary::new(1, vec![sort_decl_open("S")], vec![primitive(f)]).expect("vocab");
    let h_v = hash(&vocab).expect("hash");

    // Saídas contendo valores com bytes distintos em cada posição
    // Para caber em |S|, usamos cardinalidade 0x01020305
    let outputs = vec![0x01020304];
    let expected_bytes = [0x01, 0x02, 0x03, 0x04];

    let data = ModelData {
        vocab: h_v,
        domains: vec![0x01020305],
        consts: Vec::new(),
        funcs: vec![(0, outputs.clone())],
        preds: Vec::new(),
    };
    // Atenção: domínio 0x01020305 estoura o default max_domain_size (65536).
    // Usamos limites customizados para testar endianness
    let custom_limits = ModelLimits {
        max_bytes: 1_048_576,
        max_domain_size: 0x02000000,
        max_cells: 100_000,
    };
    let model = FiniteModel::new(data, &vocab, custom_limits).expect("valid model");
    let wire = model.encode_with_limits(custom_limits).expect("encode");

    // Verificar que os bytes da wire contêm exatamente a sequência big-endian
    assert!(
        wire.windows(4).any(|w| w == expected_bytes),
        "wire deve conter bytes compactados em big-endian exato"
    );

    // Decodificar e confirmar recuperação idêntica
    let decoded = FiniteModel::decode_with_limits(&wire, &vocab, custom_limits).expect("decode");
    assert_eq!(decoded.data().funcs[0].1, outputs);
}

// ============================================================================
// 11. Limites de recursos, produtos gigantes e checked arithmetic
// ============================================================================

#[test]
fn test_resource_bounds_and_overflow() {
    let s = sort("S");
    let t = sort("T");
    let f = func_sym("f", vec![s.clone(), t.clone()], "S");
    let p = pred_sym("p", vec![s.clone()]);

    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S"), sort_decl_open("T")],
        vec![primitive(f), primitive(p)],
    )
    .expect("vocab");
    let h_v = hash(&vocab).expect("hash");

    let valid_data = ModelData {
        vocab: h_v,
        domains: vec![2, 3],
        consts: Vec::new(),
        funcs: vec![(0, vec![0, 1, 0, 1, 0, 1])], // 6 células
        preds: vec![(1, vec![vec![0], vec![1]])], // 2 tuplas * max(1, 1) = 2 células
    };
    // Total de células = 6 + 2 = 8 células.

    // 1. Limite inclusivo de max_cells:
    // Com max_cells = 8: sucesso
    let lim_8 = ModelLimits {
        max_bytes: 1024,
        max_domain_size: 10,
        max_cells: 8,
    };
    assert!(FiniteModel::new(valid_data.clone(), &vocab, lim_8).is_ok());

    // Com max_cells = 7: Unknown(Size)
    let lim_7 = ModelLimits {
        max_bytes: 1024,
        max_domain_size: 10,
        max_cells: 7,
    };
    assert_eq!(
        FiniteModel::new(valid_data.clone(), &vocab, lim_7),
        Err(ModelError::Unknown(Exhausted::Size)),
        "células excedendo max_cells deve retornar Unknown(Size)"
    );

    // 2. Limite inclusivo de max_domain_size:
    // Domínios são [2, 3]. Máximo é 3.
    let lim_dom_3 = ModelLimits {
        max_bytes: 1024,
        max_domain_size: 3,
        max_cells: 10,
    };
    assert!(FiniteModel::new(valid_data.clone(), &vocab, lim_dom_3).is_ok());

    let lim_dom_2 = ModelLimits {
        max_bytes: 1024,
        max_domain_size: 2,
        max_cells: 10,
    };
    assert_eq!(
        FiniteModel::new(valid_data.clone(), &vocab, lim_dom_2),
        Err(ModelError::Unknown(Exhausted::Size)),
        "cardinalidade excedendo max_domain_size deve retornar Unknown(Size)"
    );

    // 3. Limite de max_bytes na decodificação:
    let model = FiniteModel::new(valid_data, &vocab, ModelLimits::default()).expect("model");
    let wire = model
        .encode_with_limits(ModelLimits::default())
        .expect("encode");

    let lim_wire_ok = ModelLimits {
        max_bytes: wire.len(),
        max_domain_size: 65536,
        max_cells: 100_000,
    };
    assert!(FiniteModel::decode_with_limits(&wire, &vocab, lim_wire_ok).is_ok());

    let lim_wire_tight = ModelLimits {
        max_bytes: wire.len() - 1,
        max_domain_size: 65536,
        max_cells: 100_000,
    };
    assert_eq!(
        FiniteModel::decode_with_limits(&wire, &vocab, lim_wire_tight),
        Err(ModelError::Unknown(Exhausted::Size)),
        "wire excedendo max_bytes deve retornar Unknown(Size) antes de decodificar"
    );

    // 4. Produto gigante esperado ANTES de alocar tabela tipada:
    // domínios [65536, 65536] com função f(S, T).
    // Produto = 65536 * 65536 = 4.294.967.296 células > max_cells (100.000).
    // O erro DEVE ser Unknown(Size), e NÃO NonTotalFunction, mesmo que a lista de saídas fornecida seja vazia!
    let giant_data = ModelData {
        vocab: h_v,
        domains: vec![65536, 65536],
        consts: Vec::new(),
        funcs: vec![(0, Vec::new())], // saídas vazias!
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(giant_data, &vocab, ModelLimits::default()),
        Err(ModelError::Unknown(Exhausted::Size)),
        "produto de função esperado que excede max_cells deve retornar Unknown(Size), não NonTotalFunction"
    );

    // 5. Overflow de multiplicação de cardinalidades:
    // Três eixos de tamanho 65536: 65536 * 65536 * 65536 estoura u32/usize em arquitecturas de 32 bits
    // ou domínios com u32::MAX
    let ternary_f = func_sym("f3", vec![s.clone(), s.clone(), s.clone()], "S");
    let vocab_ternary =
        Vocabulary::new(1, vec![sort_decl_open("S")], vec![primitive(ternary_f)]).expect("vocab");
    let h_v_ternary = hash(&vocab_ternary).expect("hash");

    let overflow_data = ModelData {
        vocab: h_v_ternary,
        domains: vec![u32::MAX],
        consts: Vec::new(),
        funcs: vec![(0, Vec::new())],
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(
            overflow_data,
            &vocab_ternary,
            ModelLimits {
                max_bytes: 1_048_576,
                max_domain_size: u32::MAX,
                max_cells: usize::MAX,
            }
        ),
        Err(ModelError::Unknown(Exhausted::Size)),
        "overflow na multiplicação do produto esperado deve retornar Unknown(Size)"
    );
}

// ============================================================================
// 12. Regras clássicas de FOL: domínios não vazios e contagem exata de sorts
// ============================================================================

#[test]
fn test_classical_fol_empty_domain_rejection() {
    let vocab = Vocabulary::new(
        1,
        vec![sort_decl_open("S"), sort_decl_open("T")],
        Vec::new(),
    )
    .expect("vocab");
    let h_v = hash(&vocab).expect("hash");
    let limits = ModelLimits::default();

    // 1. Domínio de cardinalidade zero: FOL clássica requer domínios não-vazios
    let empty_domain_data = ModelData {
        vocab: h_v,
        domains: vec![0, 2],
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(empty_domain_data, &vocab, limits),
        Err(ModelError::InvalidDomains),
        "domínio vazio (cardinalidade 0) deve retornar InvalidDomains"
    );

    // 2. Quantidade de domínios diferente da quantidade de sorts declarados
    let missing_sort_data = ModelData {
        vocab: h_v,
        domains: vec![2], // apenas 1 domínio, mas há 2 sorts declarados
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    assert_eq!(
        FiniteModel::new(missing_sort_data, &vocab, limits),
        Err(ModelError::InvalidDomains),
        "contagem de domínios diferente de sorts deve retornar InvalidDomains"
    );
}

// ============================================================================
// 13. Vocabulário vazio aceito e roundtrip
// ============================================================================

#[test]
fn test_empty_vocabulary_accepted() {
    let empty_vocab = Vocabulary::new(0, Vec::new(), Vec::new()).expect("empty vocab");
    let h_v = hash(&empty_vocab).expect("hash");
    let limits = ModelLimits::default();

    let data = ModelData {
        vocab: h_v,
        domains: Vec::new(),
        consts: Vec::new(),
        funcs: Vec::new(),
        preds: Vec::new(),
    };
    let model = FiniteModel::new(data.clone(), &empty_vocab, limits)
        .expect("modelo sobre vocabulário vazio deve ser aceito");

    let wire = model.encode_with_limits(limits).expect("encode");
    let decoded = FiniteModel::decode_with_limits(&wire, &empty_vocab, limits).expect("decode");

    assert_eq!(model, decoded);
    assert_eq!(decoded.data(), &data);
}

#[test]
fn encoding_respects_all_resource_limits() {
    let vocabulary = Vocabulary::new(
        1,
        vec![sort_decl_open("S")],
        vec![primitive(func_sym("f", vec![sort("S")], "S"))],
    )
    .expect("vocabulary");
    let data = ModelData {
        vocab: hash(&vocabulary).expect("hash"),
        domains: vec![2],
        consts: vec![],
        funcs: vec![(0, vec![0, 1])],
        preds: vec![],
    };
    let limits = ModelLimits::default();
    let model = FiniteModel::new(data.clone(), &vocabulary, limits).expect("model");
    let wire = model.encode_with_limits(limits).expect("wire");
    let exact = ModelLimits {
        max_bytes: wire.len(),
        max_domain_size: 2,
        max_cells: 2,
    };
    assert_eq!(
        model.encode_with_limits(exact).expect("inclusive limits"),
        wire
    );
    assert_eq!(
        FiniteModel::new(data.clone(), &vocabulary, exact)
            .expect("inclusive constructor")
            .data(),
        &data
    );
    for tight in [
        ModelLimits {
            max_cells: 1,
            ..exact
        },
        ModelLimits {
            max_domain_size: 1,
            ..exact
        },
        ModelLimits {
            max_bytes: wire.len() - 1,
            ..exact
        },
    ] {
        assert_eq!(
            model.encode_with_limits(tight),
            Err(ModelError::Unknown(Exhausted::Size))
        );
        assert_eq!(
            FiniteModel::new(data.clone(), &vocabulary, tight),
            Err(ModelError::Unknown(Exhausted::Size))
        );
    }
}
