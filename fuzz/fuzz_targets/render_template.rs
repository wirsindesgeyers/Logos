//! Templates v2 arbitrários e decoder CBOR preservam identidade.
#![no_main]
use libfuzzer_sys::fuzz_target;
use logos_air::{
    Grounding, Sort, SortDecl, SortDomain, Symbol, SymbolDecl, SymbolKind, SymbolResult, Vocabulary,
};
use logos_canon::{Text, from_canon, to_canon};

fuzz_target!(|data: &[u8]| {
    if let Ok(vocabulary) = from_canon::<Vocabulary>(data) {
        assert_eq!(to_canon(&vocabulary).unwrap(), data);
    }
    if data.is_empty() || data.len() > 8193 {
        return;
    }
    let Ok(raw) = std::str::from_utf8(&data[1..]) else {
        return;
    };
    let Ok(template) = Text::new(raw) else {
        return;
    };
    let arity = usize::from(data[0] % 4);
    let sort = Sort {
        name: Text::new("S").unwrap(),
    };
    let symbol = SymbolDecl {
        signature: Symbol {
            name: Text::new("P").unwrap(),
            kind: SymbolKind::Pred,
            arity: vec![sort.clone(); arity],
            result: SymbolResult::Bool,
        },
        grounding: Grounding::Glossed(Text::new("").unwrap()),
        gloss_pt: Text::new("").unwrap(),
    };
    if let Ok(vocabulary) = Vocabulary::new_with_render_pt(
        0,
        vec![SortDecl {
            name: sort.name,
            domain: SortDomain::Open,
        }],
        vec![symbol],
        vec![template.clone()],
    ) {
        assert_eq!(vocabulary.render_pt(), Some([template].as_slice()));
        let encoded = to_canon(&vocabulary).unwrap();
        assert_eq!(from_canon::<Vocabulary>(&encoded).unwrap(), vocabulary);
    }
});
