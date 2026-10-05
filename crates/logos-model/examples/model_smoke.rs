//! Exercício real da API pública de logos-model: `cargo run -p logos-model --example model_smoke`.

use std::error::Error;

use logos_air::{
    DomainSource, FactRef, Sort, SortDecl, SortDomain, Symbol, SymbolDecl, SymbolKind,
    SymbolResult, Vocabulary,
};
use logos_canon::Text;
use logos_core::Exhausted;
use logos_crypto::hash;
use logos_model::{FiniteModel, ModelData, ModelError, ModelLimits};

fn require(condition: bool, message: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Construir vocabulário real com domínio misto (Open e Enumerated)
    let sort_s = Sort {
        name: Text::new("S")?,
    };
    let decl_sort_s = SortDecl {
        name: Text::new("S")?,
        domain: SortDomain::Open,
    };

    let sort_color = Sort {
        name: Text::new("Color")?,
    };
    let sym_red = Symbol {
        name: Text::new("Red")?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort_color.clone()),
    };
    let sym_green = Symbol {
        name: Text::new("Green")?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort_color.clone()),
    };
    let decl_sort_color = SortDecl {
        name: Text::new("Color")?,
        domain: SortDomain::Enumerated {
            constants: vec![sym_red.clone(), sym_green.clone()],
            domain_source: DomainSource::Fact(FactRef([0x11; 32])),
        },
    };

    let sym_base = Symbol {
        name: Text::new("base")?,
        kind: SymbolKind::Const,
        arity: Vec::new(),
        result: SymbolResult::Sort(sort_s.clone()),
    };
    let sym_step = Symbol {
        name: Text::new("step")?,
        kind: SymbolKind::Func,
        arity: vec![sort_s.clone(), sort_color.clone()],
        result: SymbolResult::Sort(sort_s.clone()),
    };
    let sym_valid = Symbol {
        name: Text::new("valid")?,
        kind: SymbolKind::Pred,
        arity: vec![sort_color.clone(), sort_s.clone()],
        result: SymbolResult::Bool,
    };
    let sym_active = Symbol {
        name: Text::new("active")?,
        kind: SymbolKind::Pred,
        arity: Vec::new(),
        result: SymbolResult::Bool,
    };

    let decls = vec![
        SymbolDecl {
            signature: sym_red,
            grounding: logos_air::Grounding::Glossed(Text::new("vermelho")?),
            gloss_pt: Text::new("cor vermelha")?,
        },
        SymbolDecl {
            signature: sym_green,
            grounding: logos_air::Grounding::Glossed(Text::new("verde")?),
            gloss_pt: Text::new("cor verde")?,
        },
        SymbolDecl {
            signature: sym_base,
            grounding: logos_air::Grounding::Glossed(Text::new("base")?),
            gloss_pt: Text::new("elemento base")?,
        },
        SymbolDecl {
            signature: sym_step,
            grounding: logos_air::Grounding::Glossed(Text::new("passo")?),
            gloss_pt: Text::new("funcao de transicao")?,
        },
        SymbolDecl {
            signature: sym_valid,
            grounding: logos_air::Grounding::Glossed(Text::new("valido")?),
            gloss_pt: Text::new("predicado de validade")?,
        },
        SymbolDecl {
            signature: sym_active,
            grounding: logos_air::Grounding::Glossed(Text::new("ativo")?),
            gloss_pt: Text::new("predicado nulo ativo")?,
        },
    ];

    let vocab = Vocabulary::new(1, vec![decl_sort_s, decl_sort_color], decls)?;
    let vocab_hash = hash(&vocab)?;

    // 2. Construir ModelData e FiniteModel válidos
    // |S| = 3, |Color| = 2
    // Constantes:
    // Red (id 0) -> 0
    // Green (id 1) -> 1
    // base (id 2) -> 2
    // Função step (id 3): S x Color -> S. 3 x 2 = 6 saídas no formato row-major
    // Predicado valid (id 4): Color x S. Tuplas lexicograficamente ordenadas
    // Predicado active (id 5): nulário true = [[]]
    let valid_data = ModelData {
        vocab: vocab_hash,
        domains: vec![3, 2],
        consts: vec![(0, 0), (1, 1), (2, 2)],
        funcs: vec![(3, vec![1, 2, 0, 1, 2, 0])],
        preds: vec![(4, vec![vec![0, 1], vec![1, 2]]), (5, vec![vec![]])],
    };

    let limits = ModelLimits::default();
    let model = FiniteModel::new(valid_data.clone(), &vocab, limits)?;

    // 3. Roundtrip de codificação e decodificação
    let wire = model.encode_with_limits(limits)?;
    let decoded = FiniteModel::decode_with_limits(&wire, &vocab, limits)?;
    require(
        model == decoded,
        "modelo decodificado deve ser identico ao original",
    )?;
    require(
        decoded.data() == &valid_data,
        "dados internos devem coincidir",
    )?;

    // 4. Exercitar erro de função parcial / divergência de tamanho
    let partial_data = ModelData {
        vocab: vocab_hash,
        domains: vec![3, 2],
        consts: vec![(0, 0), (1, 1), (2, 2)],
        funcs: vec![(3, vec![1, 2, 0])], // apenas 3 saídas em vez de 6
        preds: vec![(4, vec![vec![0, 1]]), (5, vec![vec![]])],
    };
    let partial_err = FiniteModel::new(partial_data, &vocab, limits);
    require(
        partial_err == Err(ModelError::NonTotalFunction),
        "funcao parcial deve retornar NonTotalFunction",
    )?;

    // 5. Exercitar erro de saída fora do domínio
    let ood_data = ModelData {
        vocab: vocab_hash,
        domains: vec![3, 2],
        consts: vec![(0, 0), (1, 1), (2, 2)],
        funcs: vec![(3, vec![1, 2, 0, 1, 2, 3])], // saída 3 >= |S|=3
        preds: vec![(4, vec![vec![0, 1]]), (5, vec![vec![]])],
    };
    let ood_err = FiniteModel::new(ood_data, &vocab, limits);
    require(
        ood_err == Err(ModelError::ValueOutOfDomain),
        "saida >= cardinalidade deve retornar ValueOutOfDomain",
    )?;

    // 6. Exercitar limites de recursos com Unknown(Size)
    // Limite estrito de max_bytes
    let tight_bytes_limits = ModelLimits {
        max_bytes: 10,
        max_domain_size: 65_536,
        max_cells: 100_000,
    };
    let size_err = FiniteModel::decode_with_limits(&wire, &vocab, tight_bytes_limits);
    require(
        size_err == Err(ModelError::Unknown(Exhausted::Size)),
        "exceder max_bytes deve retornar Unknown(Size)",
    )?;

    // Limite estrito de max_domain_size
    let tight_domain_limits = ModelLimits {
        max_bytes: 1_048_576,
        max_domain_size: 1, // domínios são 3 e 2
        max_cells: 100_000,
    };
    let domain_size_err = FiniteModel::new(valid_data.clone(), &vocab, tight_domain_limits);
    require(
        domain_size_err == Err(ModelError::Unknown(Exhausted::Size)),
        "exceder max_domain_size deve retornar Unknown(Size)",
    )?;

    // Limite estrito de max_cells
    // Total de células: 3 (consts) + 6 (funcs) + 4 (pred valid) + 1 (pred active) = 14
    let tight_cells_limits = ModelLimits {
        max_bytes: 1_048_576,
        max_domain_size: 65_536,
        max_cells: 10,
    };
    let cells_err = FiniteModel::new(valid_data, &vocab, tight_cells_limits);
    require(
        cells_err == Err(ModelError::Unknown(Exhausted::Size)),
        "exceder max_cells deve retornar Unknown(Size)",
    )?;

    println!("MODEL_SMOKE_OK");
    Ok(())
}
