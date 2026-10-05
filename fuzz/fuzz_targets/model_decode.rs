//! Fuzz target para FiniteModel sob limites estritos e vocabulários fixos representativos.
//!
//! Contrato:
//! - Vocabulários fixos representativos cobrindo funções, predicados e tipos enumerados
//! - Limites estritos de bytes, cardinalidade e agregado de células
//! - Idempotência para toda entrada aceita: decode(bytes) -> encode(model) -> decode(encoded) == model
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_air::{
    DomainSource, FactRef, Formula, FormulaNode, Grounding, Limits, Sort, SortDecl, SortDomain,
    Symbol, SymbolDecl, SymbolKind, SymbolResult, Vocabulary,
};
use logos_canon::Text;
use logos_model::{FiniteModel, ModelLimits};
use std::sync::LazyLock;

/// Vocabulário exato do vetor canônico `mixed_axes`:
/// Versão 7, sorts S Open, T Open; c(Const->S), f(Func S,T->S), P(Pred T,S->Bool);
/// cada grounding Glossed("primitivo"), gloss_pt "glosa".
static MIXED_VOCAB: LazyLock<Vocabulary> = LazyLock::new(|| {
    let s = Sort {
        name: Text::new("S").expect("text S"),
    };
    let t = Sort {
        name: Text::new("T").expect("text T"),
    };
    let c = Symbol {
        name: Text::new("c").expect("text c"),
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(s.clone()),
    };
    let f = Symbol {
        name: Text::new("f").expect("text f"),
        kind: SymbolKind::Func,
        arity: vec![s.clone(), t.clone()],
        result: SymbolResult::Sort(s.clone()),
    };
    let p = Symbol {
        name: Text::new("P").expect("text P"),
        kind: SymbolKind::Pred,
        arity: vec![t.clone(), s.clone()],
        result: SymbolResult::Bool,
    };
    let symbols = vec![
        SymbolDecl {
            signature: c,
            grounding: Grounding::Glossed(Text::new("primitivo").expect("gloss")),
            gloss_pt: Text::new("glosa").expect("gloss_pt"),
        },
        SymbolDecl {
            signature: f,
            grounding: Grounding::Glossed(Text::new("primitivo").expect("gloss")),
            gloss_pt: Text::new("glosa").expect("gloss_pt"),
        },
        SymbolDecl {
            signature: p,
            grounding: Grounding::Glossed(Text::new("primitivo").expect("gloss")),
            gloss_pt: Text::new("glosa").expect("gloss_pt"),
        },
    ];
    let sorts = vec![
        SortDecl {
            name: Text::new("S").expect("text S"),
            domain: SortDomain::Open,
        },
        SortDecl {
            name: Text::new("T").expect("text T"),
            domain: SortDomain::Open,
        },
    ];
    Vocabulary::new(7, sorts, symbols).expect("valid mixed_axes vocabulary")
});

/// Vocabulário do vetor canônico com sort enumerado e símbolo Defined:
/// Versão 1, sort S Enumerated [a, b], símbolos b (primitivo), D (definido), a (primitivo).
static ENUM_VOCAB: LazyLock<Vocabulary> = LazyLock::new(|| {
    let s = Sort {
        name: Text::new("S").expect("text S"),
    };
    let a = Symbol {
        name: Text::new("a").expect("text a"),
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(s.clone()),
    };
    let b = Symbol {
        name: Text::new("b").expect("text b"),
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(s.clone()),
    };
    let d = Symbol {
        name: Text::new("D").expect("text D"),
        kind: SymbolKind::Pred,
        arity: Vec::new(),
        result: SymbolResult::Bool,
    };
    let sort_decl = SortDecl {
        name: Text::new("S").expect("text S"),
        domain: SortDomain::Enumerated {
            constants: vec![a.clone(), b.clone()],
            domain_source: DomainSource::Fact(FactRef([0xab; 32])),
        },
    };
    let defined_formula =
        Formula::new(vec![FormulaNode::True], Limits::default()).expect("formula");
    let symbols = vec![
        SymbolDecl {
            signature: b,
            grounding: Grounding::Glossed(Text::new("primitivo").expect("gloss")),
            gloss_pt: Text::new("glosa").expect("gloss_pt"),
        },
        SymbolDecl {
            signature: d,
            grounding: Grounding::Defined(defined_formula),
            gloss_pt: Text::new("definido").expect("gloss_pt"),
        },
        SymbolDecl {
            signature: a,
            grounding: Grounding::Glossed(Text::new("primitivo").expect("gloss")),
            gloss_pt: Text::new("glosa").expect("gloss_pt"),
        },
    ];
    Vocabulary::new(1, vec![sort_decl], symbols).expect("valid enum vocabulary")
});

fn check_model(data: &[u8], vocab: &Vocabulary, limits: ModelLimits) {
    if let Ok(model) = FiniteModel::decode_with_limits(data, vocab, limits) {
        let encoded = model
            .encode_with_limits(limits)
            .expect("accepted model must encode");
        assert_eq!(
            encoded, data,
            "accepted model wire must match canonical input"
        );
        let decoded = FiniteModel::decode_with_limits(&encoded, vocab, limits)
            .expect("re-decoding canonical wire must succeed");
        assert_eq!(decoded, model, "encode->decode must be idempotent");
    }
}

fuzz_target!(|data: &[u8]| {
    // Limites estritos para fuzzer
    let tight_limits = ModelLimits {
        max_bytes: 4096,
        max_domain_size: 16,
        max_cells: 64,
    };

    check_model(data, &MIXED_VOCAB, tight_limits);
    check_model(data, &ENUM_VOCAB, tight_limits);

    // Também testar sob limites padrão para entradas curtas
    if data.len() <= 1024 {
        let default_limits = ModelLimits::default();
        check_model(data, &MIXED_VOCAB, default_limits);
    }
});
