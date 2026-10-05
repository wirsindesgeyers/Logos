use logos_obligation::Obligation;

fn main() {
    let _o = Obligation {
        logic: logos_air::LogicId::FolMsEqV1,
        encoder_version: 1,
        vocab: logos_crypto::hash_raw::<logos_air::Vocabulary>(b"dummy"),
        premises: Vec::new(),
        goal: logos_air::Formula::new(
            vec![logos_air::FormulaNode::True],
            logos_air::Limits::default(),
        )
        .unwrap(),
    };
}
