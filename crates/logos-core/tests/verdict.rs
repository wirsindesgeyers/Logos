//! Testes de construção de veredictos e preservação estrita de erros.

use logos_canon::Text;
use logos_core::{
    CheckOutcome, Exhausted, Reason, Verdict, refuted_from_countermodel, verified_from_proof,
};

#[test]
fn verified_from_proof_matrix() {
    // 1. Accepted -> Ok(Verdict::Verified)
    assert_eq!(
        verified_from_proof(CheckOutcome::Accepted),
        Ok(Verdict::Verified)
    );

    // 2. Rejected(Reason) -> Err(CheckOutcome::Rejected(Reason))
    let reason = Reason::new(Text::new("cláusula não resolvida").expect("valid"));
    let rejected = CheckOutcome::Rejected(reason.clone());
    let err = verified_from_proof(rejected).expect_err("Rejected não é veredicto");
    assert_eq!(err, CheckOutcome::Rejected(reason));

    // 3. Unknown com os quatro motivos de exaustão -> Err(CheckOutcome::Unknown(e))
    let exhausted_cases = [
        Exhausted::Time,
        Exhausted::Steps,
        Exhausted::Memory,
        Exhausted::Size,
    ];
    for e in exhausted_cases {
        let outcome = CheckOutcome::Unknown(e);
        let err = verified_from_proof(outcome).expect_err("Unknown não é veredicto");
        assert_eq!(err, CheckOutcome::Unknown(e));
    }
}

#[test]
fn refuted_from_countermodel_matrix() {
    // 1. Accepted -> Ok(Verdict::Refuted)
    assert_eq!(
        refuted_from_countermodel(CheckOutcome::Accepted),
        Ok(Verdict::Refuted)
    );

    // 2. Rejected(Reason) -> Err(CheckOutcome::Rejected(Reason))
    let reason = Reason::new(Text::new("modelo não satisfaz fórmula").expect("valid"));
    let rejected = CheckOutcome::Rejected(reason.clone());
    let err = refuted_from_countermodel(rejected).expect_err("Rejected não é veredicto");
    assert_eq!(err, CheckOutcome::Rejected(reason));

    // 3. Unknown com os quatro motivos de exaustão -> Err(CheckOutcome::Unknown(e))
    let exhausted_cases = [
        Exhausted::Time,
        Exhausted::Steps,
        Exhausted::Memory,
        Exhausted::Size,
    ];
    for e in exhausted_cases {
        let outcome = CheckOutcome::Unknown(e);
        let err = refuted_from_countermodel(outcome).expect_err("Unknown não é veredicto");
        assert_eq!(err, CheckOutcome::Unknown(e));
    }
}

#[test]
fn reason_text_and_diagnostics_preserved_intact() {
    let raw = "erro no nó 123: ¬(P ∧ Q) ⊢ ¬P ∨ ¬Q";
    let text = Text::new(raw).expect("valid NFC");
    let reason = Reason::new(text);

    assert_eq!(reason.as_str(), raw);
    assert_eq!(format!("{reason}"), raw);

    let outcome = CheckOutcome::Rejected(reason.clone());
    let err_proof = verified_from_proof(outcome.clone()).unwrap_err();
    let err_counter = refuted_from_countermodel(outcome).unwrap_err();

    match (err_proof, err_counter) {
        (CheckOutcome::Rejected(r1), CheckOutcome::Rejected(r2)) => {
            assert_eq!(r1.as_str(), raw);
            assert_eq!(r2.as_str(), raw);
            assert_eq!(r1, r2);
        }
        _ => panic!("ambos devem ser Rejected"),
    }
}
