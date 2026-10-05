//! Fuzzing do sort-checker: consome decoders existentes de air/v1 e vocabulary/v1,
//! valida limites dos paths e invariantes dos motivos de recusa.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_air::{Formula, FormulaNode, Limits, SortErrorKind, TermPosition, Vocabulary, check};
use logos_canon::{Value, from_canon};

fn exercise(vocab: &Vocabulary, formula: &Formula) {
    let result = check(vocab, formula);

    if let Err(err) = result {
        // formula_node DEVE ser um índice válido no array de nós da fórmula
        assert!(
            err.path.formula_node < formula.nodes().len(),
            "formula_node fora dos limites de formula.nodes()"
        );

        if let Some(term_path) = err.path.term {
            let formula_node = &formula.nodes()[err.path.formula_node];
            match (formula_node, term_path.position) {
                (FormulaNode::Eq(left, _), TermPosition::EqualityLeft) => {
                    assert!(
                        term_path.node < left.nodes().len(),
                        "nó fora dos limites do termo esquerdo de Eq"
                    );
                }
                (FormulaNode::Eq(_, right), TermPosition::EqualityRight) => {
                    assert!(
                        term_path.node < right.nodes().len(),
                        "nó fora dos limites do termo direito de Eq"
                    );
                }
                (FormulaNode::Pred { args, .. }, TermPosition::PredicateArgument(arg_index)) => {
                    assert!(
                        arg_index < args.len(),
                        "índice de argumento fora dos limites do Pred"
                    );
                    assert!(
                        term_path.node < args[arg_index].nodes().len(),
                        "nó fora dos limites do argumento de Pred"
                    );
                }
                _ => {
                    panic!("TermPath aponta para posição inexistente no record de FormulaNode");
                }
            }
        }

        // Valida invariantes dos variantes de SortErrorKind
        match err.kind {
            SortErrorKind::UnboundVariable { index, binders } => {
                assert!(
                    u128::from(index) >= binders as u128,
                    "UnboundVariable emitido para índice menor que a quantidade de binders ativos"
                );
            }
            SortErrorKind::ArityMismatch { expected, actual } => {
                assert_ne!(
                    expected, actual,
                    "ArityMismatch com contagens esperada e real idênticas"
                );
            }
            SortErrorKind::InvalidConstantArity { actual } => {
                assert!(
                    actual > 0,
                    "InvalidConstantArity reportado com aridade zero"
                );
            }
            SortErrorKind::SortMismatch { expected, actual } => {
                assert_ne!(
                    expected, actual,
                    "SortMismatch reportado com sorts idênticos"
                );
            }
            _ => {}
        }
    }
}

fuzz_target!(|data: &[u8]| {
    // A fuzz-only pair of existing canonical values, not a new production format.
    if let Ok(Value::Array(pair)) = from_canon::<Value>(data) {
        if let [vocabulary, formula] = pair.items() {
            if let (Ok(vocab), Ok(formula)) = (
                Vocabulary::from_value_with_limits(vocabulary, Limits::default()),
                Formula::from_value_with_limits(formula, Limits::default()),
            ) {
                exercise(&vocab, &formula);
            }
        }
    }
    // Standalone AIR also exercises errors against an undeclared namespace.
    if let Ok(formula) = from_canon::<Formula>(data) {
        let empty = Vocabulary::new(1, Vec::new(), Vec::new())
            .expect("empty vocabulary satisfies all construction invariants");
        exercise(&empty, &formula);
    }
});
