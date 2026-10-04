//! Exercício real da API pública: `cargo run -p logos-air --example air_smoke`.

use logos_air::{
    AirError, Formula, FormulaNode, Limits, Sort, Symbol, SymbolKind, SymbolResult, Term, TermNode,
};
use logos_canon::{Text, from_canon, to_canon};
use logos_crypto::hash;
use std::error::Error;

fn require(condition: bool, message: &str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let sort = Sort {
        name: Text::new("person")?,
    };
    let symbol = Symbol {
        name: Text::new("P")?,
        kind: SymbolKind::Pred,
        arity: vec![sort.clone()],
        result: SymbolResult::Bool,
    };
    let term = Term::new(
        vec![TermNode::Var {
            index: 0,
            sort: sort.clone(),
        }],
        Limits::default(),
    )?;
    let formula = Formula::new(
        vec![
            FormulaNode::Forall(sort),
            FormulaNode::Pred {
                symbol,
                args: vec![term],
            },
        ],
        Limits {
            max_depth: 3,
            max_nodes: 3,
        },
    )?;
    // CBOR independente do encoder Rust; SHA-256 calculado com hashlib.
    const EXPECTED: &[u8] = &[
        0x84, 0x01, 0x03, 0x6c, 0x46, 0x4f, 0x4c, 0x2d, 0x6d, 0x73, 0x2d, 0x65, 0x71, 0x2f, 0x76,
        0x31, 0x82, 0x82, 0x09, 0x83, 0x01, 0x00, 0x66, 0x70, 0x65, 0x72, 0x73, 0x6f, 0x6e, 0x83,
        0x03, 0x86, 0x01, 0x01, 0x61, 0x50, 0x02, 0x81, 0x83, 0x01, 0x00, 0x66, 0x70, 0x65, 0x72,
        0x73, 0x6f, 0x6e, 0x81, 0x01, 0x81, 0x84, 0x01, 0x02, 0x6c, 0x46, 0x4f, 0x4c, 0x2d, 0x6d,
        0x73, 0x2d, 0x65, 0x71, 0x2f, 0x76, 0x31, 0x81, 0x83, 0x00, 0x00, 0x83, 0x01, 0x00, 0x66,
        0x70, 0x65, 0x72, 0x73, 0x6f, 0x6e,
    ];
    require(
        to_canon(&formula)? == EXPECTED,
        "bytes de ∀x.P(x) divergiram",
    )?;
    require(
        hash(&formula)?.to_hex()
            == "170b828b4fa21b68596442c7fb16f06e34482978becc297897978f36f9a14fec",
        "hash de fórmula divergente",
    )?;
    require(
        from_canon::<Formula>(EXPECTED)? == formula,
        "decode de fixture ∀x.P(x)",
    )?;
    require(
        formula.depth() == 3 && formula.node_count() == 3,
        "estatísticas agregadas",
    )?;
    for limits in [
        Limits {
            max_depth: 2,
            max_nodes: 3,
        },
        Limits {
            max_depth: 3,
            max_nodes: 2,
        },
    ] {
        let outcome = Formula::decode_with_limits(EXPECTED, limits);
        require(
            matches!(
                outcome,
                Err(AirError::DepthExceeded { .. } | AirError::NodesExceeded { .. })
            ),
            "limite agregado não rejeitou",
        )?;
    }
    let mut wrong_logic = EXPECTED.to_vec();
    wrong_logic[4] = b'X';
    require(
        Formula::decode_with_limits(&wrong_logic, Limits::default())
            == Err(AirError::UnsupportedLogic),
        "LogicId desconhecido aceito",
    )?;
    require(
        Formula::new(vec![FormulaNode::Not], Limits::default()) == Err(AirError::InvalidStructure),
        "prefixo incompleto aceito",
    )?;
    require(
        Formula::new(
            vec![FormulaNode::True, FormulaNode::False],
            Limits::default(),
        ) == Err(AirError::InvalidStructure),
        "raízes extras aceitas",
    )?;
    println!(
        "AIR_SMOKE_OK: FOL-ms-eq/v1, fixture/hash independentes, limites e erros ({})",
        std::env::consts::ARCH
    );
    Ok(())
}
