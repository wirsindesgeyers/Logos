//! Testes comportamentais de orçamento e medidor de passos lógicos.

use logos_canon::Text;
use logos_core::{Budget, CheckOutcome, Exhausted, Reason, StepMeter};

/// Checker de referência: percorre bytes, cobrando 1 passo por byte antes de
/// inspecionar. Rejeita se encontrar o byte 255 (0xFF).
fn reference_checker(meter: &StepMeter, bytes: &[u8]) -> CheckOutcome {
    for &b in bytes {
        if let Err(e) = meter.charge(1) {
            return CheckOutcome::Unknown(e);
        }
        if b == 255 {
            let reason = Reason::new(Text::new("invalid byte 255").expect("valid text"));
            return CheckOutcome::Rejected(reason);
        }
    }
    CheckOutcome::Accepted
}

#[test]
fn reference_checker_acceptance_with_sufficient_budget() {
    let bytes = [10, 20, 30, 40, 50];
    let budget = Budget::new(10, 1024, 2048, 100);
    let outcome = StepMeter::run(&budget, |meter| reference_checker(meter, &bytes));
    assert_eq!(outcome, CheckOutcome::Accepted);
}

#[test]
fn reference_checker_acceptance_exact_boundary() {
    let bytes = [1, 2, 3];
    let budget = Budget::new(3, 0, 0, 0);
    let outcome = StepMeter::run(&budget, |meter| reference_checker(meter, &bytes));
    assert_eq!(outcome, CheckOutcome::Accepted);
}

#[test]
fn reference_checker_rejection_with_sufficient_budget() {
    // Byte 255 está no índice 2; requer 3 cobranças de 1 passo.
    let bytes = [1, 2, 255, 4];
    let budget = Budget::new(10, 0, 0, 0);
    let outcome = StepMeter::run(&budget, |meter| reference_checker(meter, &bytes));
    match outcome {
        CheckOutcome::Rejected(reason) => {
            assert_eq!(reason.as_str(), "invalid byte 255");
        }
        other => panic!("esperava Rejected, obteve {other:?}"),
    }
}

#[test]
fn reference_checker_exhaustion_before_invalid_byte() {
    // Byte 255 está no índice 2 (3º passo), mas o orçamento só permite 2 passos.
    // A exaustão DEVE preceder a detecção do byte inválido: exaustão nunca é rejeição.
    let bytes = [1, 2, 255, 4];
    let budget = Budget::new(2, 0, 0, 0);
    let outcome = StepMeter::run(&budget, |meter| reference_checker(meter, &bytes));
    assert_eq!(outcome, CheckOutcome::Unknown(Exhausted::Steps));
}

#[test]
fn reference_checker_zero_budget_fails_immediately() {
    let bytes = [255];
    let budget = Budget::new(0, 0, 0, 0);
    let outcome = StepMeter::run(&budget, |meter| reference_checker(meter, &bytes));
    assert_eq!(outcome, CheckOutcome::Unknown(Exhausted::Steps));
}

#[test]
fn ignored_error_latch_precedence_over_accepted() {
    let budget = Budget::new(2, 0, 0, 0);
    let outcome = StepMeter::run(&budget, |meter| {
        assert!(meter.charge(1).is_ok());
        assert!(meter.charge(1).is_ok());
        // Cobrança excede o saldo e dispara o latch
        assert_eq!(meter.charge(1), Err(Exhausted::Steps));
        // Checker ignora o erro e tenta retornar Accepted
        CheckOutcome::Accepted
    });
    // O latch irreversível DEVE sobrepor o retorno da closure
    assert_eq!(outcome, CheckOutcome::Unknown(Exhausted::Steps));
}

#[test]
fn ignored_error_latch_precedence_over_rejected() {
    let budget = Budget::new(1, 0, 0, 0);
    let outcome = StepMeter::run(&budget, |meter| {
        let _ = meter.charge(5); // exaustão ignorada
        let reason = Reason::new(Text::new("rejeição simulada").expect("valid text"));
        CheckOutcome::Rejected(reason)
    });
    assert_eq!(outcome, CheckOutcome::Unknown(Exhausted::Steps));
}

#[test]
fn ignored_error_latch_precedence_over_other_unknown() {
    let budget = Budget::new(1, 0, 0, 0);
    let outcome = StepMeter::run(&budget, |meter| {
        let _ = meter.charge(10); // excede passos
        // Checker tenta sinalizar Time em vez de Steps
        CheckOutcome::Unknown(Exhausted::Time)
    });
    // Latch de passos prevalece
    assert_eq!(outcome, CheckOutcome::Unknown(Exhausted::Steps));
}

#[test]
fn latch_is_irreversible_even_for_zero_charge() {
    let budget = Budget::new(5, 0, 0, 0);
    StepMeter::run(&budget, |meter| {
        assert_eq!(meter.charge(6), Err(Exhausted::Steps));
        // Após o disparo do latch, inclusive charge(0) deve falhar
        assert_eq!(meter.charge(0), Err(Exhausted::Steps));
        assert_eq!(meter.charge(1), Err(Exhausted::Steps));
        CheckOutcome::Accepted
    });
}

#[test]
fn zero_budget_allows_zero_steps_work() {
    let budget = Budget::new(0, 0, 0, 0);
    let outcome = StepMeter::run(&budget, |meter| {
        assert_eq!(meter.charge(0), Ok(()));
        assert_eq!(meter.charge(0), Ok(()));
        CheckOutcome::Accepted
    });
    assert_eq!(outcome, CheckOutcome::Accepted);
}

#[test]
fn batched_and_fractional_charges() {
    let budget = Budget::new(100, 0, 0, 0);
    let outcome = StepMeter::run(&budget, |meter| {
        assert_eq!(meter.charge(30), Ok(()));
        assert_eq!(meter.charge(20), Ok(()));
        assert_eq!(meter.charge(50), Ok(())); // saldo agora é 0
        assert_eq!(meter.charge(0), Ok(())); // 0 é permitido antes da exaustão
        assert_eq!(meter.charge(1), Err(Exhausted::Steps)); // esgota
        assert_eq!(meter.charge(0), Err(Exhausted::Steps)); // 0 agora falha
        CheckOutcome::Accepted
    });
    assert_eq!(outcome, CheckOutcome::Unknown(Exhausted::Steps));
}

#[test]
fn u64_max_boundary_and_no_overflow() {
    // 1. Orçamento máximo consome u64::MAX e depois esgota
    let budget_max = Budget::new(u64::MAX, 0, 0, 0);
    let outcome_max = StepMeter::run(&budget_max, |meter| {
        assert_eq!(meter.charge(u64::MAX), Ok(()));
        assert_eq!(meter.charge(1), Err(Exhausted::Steps));
        CheckOutcome::Accepted
    });
    assert_eq!(outcome_max, CheckOutcome::Unknown(Exhausted::Steps));

    // 2. Orçamento pequeno com tentativa de cobrança de u64::MAX não causa overflow
    let budget_small = Budget::new(10, 0, 0, 0);
    let outcome_small = StepMeter::run(&budget_small, |meter| {
        assert_eq!(meter.charge(u64::MAX), Err(Exhausted::Steps));
        CheckOutcome::Accepted
    });
    assert_eq!(outcome_small, CheckOutcome::Unknown(Exhausted::Steps));
}

#[test]
fn non_step_fields_do_not_affect_meter() {
    let b1 = Budget::new(5, 0, 0, 0);
    let b2 = Budget::new(5, 999_999, 888_888, 777_777);

    let o1 = StepMeter::run(&b1, |m| {
        let _ = m.charge(5);
        CheckOutcome::Accepted
    });
    let o2 = StepMeter::run(&b2, |m| {
        let _ = m.charge(5);
        CheckOutcome::Accepted
    });

    assert_eq!(o1, CheckOutcome::Accepted);
    assert_eq!(o2, CheckOutcome::Accepted);
}
