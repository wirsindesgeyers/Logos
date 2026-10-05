//! Fronteira contextual: identidade, sort-check e um único orçamento com latch.

use logos_air::{Formula, Grounding, SymbolResult, Vocabulary, check, check_with_binders};
use logos_core::{Budget, CheckOutcome, Exhausted, StepMeter};
use logos_crypto::{Hash, hash};

use crate::{EvalError, FiniteModel};

/// Avalia uma fórmula fechada no modelo vinculado ao vocabulário exato.
///
/// Expande Defined sem tabelas substitutas; valida totalidade/unicidade de todas
/// as definições de valores e compatibilidade das constantes enumeradas antes
/// da fórmula, inclusive símbolos não utilizados. Conectivos e quantificadores
/// não fazem curto-circuito. Um único StepMeter cobre toda a execução.
///
/// Impõe `budget.steps`; `wall_ms` é ignorado como nos checkers internos do core.
/// `mem_bytes`/`size_bytes` não são tetos físicos: entradas já foram construídas
/// sob AIR Limits/ModelLimits. Reservas falíveis do interpretador preservam
/// Unknown(Memory). Contagem lógica e fronteiras: `spec/evaluation.md`.
///
/// # Errors
/// Unknown preserva exaustão; identidade/tipagem/definição inválida é erro de
/// entrada, nunca `false` nem uma refutação. Não emite CheckOutcome ou Verdict.
pub fn eval(
    model: &FiniteModel,
    vocabulary: &Vocabulary,
    formula: &Formula,
    budget: &Budget,
) -> Result<bool, EvalError> {
    let mut result = Err(EvalError::Unknown(Exhausted::Steps));
    let outcome = StepMeter::run(budget, |meter| {
        result = eval_on_meter(model, vocabulary, formula, meter);
        match &result {
            Err(EvalError::Unknown(reason)) => CheckOutcome::Unknown(*reason),
            // Accepted finaliza apenas o medidor; nunca é exposto como prova.
            _ => CheckOutcome::Accepted,
        }
    });
    if outcome == CheckOutcome::Unknown(Exhausted::Steps) {
        Err(EvalError::Unknown(Exhausted::Steps))
    } else {
        result
    }
}

fn charge_nodes(meter: &StepMeter, nodes: usize) -> Result<(), EvalError> {
    meter.charge(u64::try_from(nodes).map_err(|_| Exhausted::Steps)?)?;
    Ok(())
}

/// Avalia sob um medidor existente, sem restaurar seu saldo entre chamadas.
///
/// Mesmas validações e cobranças de [`eval`]; o chamador deve finalizar pelo
/// latch de [`StepMeter::run`] e preservar erros `Unknown`.
///
/// # Errors
/// Retorna os mesmos erros contextuais e de exaustão de [`eval`].
pub fn eval_on_meter(
    model: &FiniteModel,
    vocabulary: &Vocabulary,
    formula: &Formula,
    meter: &StepMeter,
) -> Result<bool, EvalError> {
    meter.charge(1)?;
    if model.data().vocab != hash(vocabulary)? {
        return Err(EvalError::VocabularyMismatch);
    }
    charge_nodes(meter, formula.node_count())?;
    check(vocabulary, formula)?;
    check_definitions(vocabulary, meter)?;
    crate::evaluate::run(model, vocabulary, formula, meter)
}

fn check_definitions(vocabulary: &Vocabulary, meter: &StepMeter) -> Result<(), EvalError> {
    for declaration in vocabulary.symbols() {
        meter.charge(1)?;
        if let Grounding::Defined(body) = &declaration.grounding {
            charge_nodes(meter, body.node_count())?;
            let signature = &declaration.signature;
            let mut binders = Vec::new();
            let capacity = signature
                .arity
                .len()
                .checked_add(1)
                .ok_or(Exhausted::Memory)?;
            binders
                .try_reserve_exact(capacity)
                .map_err(|_| Exhausted::Memory)?;
            binders.extend(signature.arity.iter().rev());
            if let SymbolResult::Sort(sort) = &signature.result {
                binders.push(sort);
            }
            check_with_binders(vocabulary, body, &binders)?;
        }
    }
    Ok(())
}

pub(crate) fn validate_model_context<'a>(
    model: &'a FiniteModel,
    vocabulary: &'a Vocabulary,
    expected_vocabulary: Hash<Vocabulary>,
    meter: &'a StepMeter,
) -> Result<crate::evaluate::Machine<'a>, EvalError> {
    meter.charge(1)?;
    if model.data().vocab != expected_vocabulary {
        return Err(EvalError::VocabularyMismatch);
    }
    check_definitions(vocabulary, meter)?;
    let mut machine = crate::evaluate::Machine::new(model, vocabulary, meter);
    machine.validate_definitions()?;
    Ok(machine)
}

pub(crate) fn eval_checked<'a>(
    machine: &mut crate::evaluate::Machine<'a>,
    formula: &'a Formula,
    meter: &StepMeter,
) -> Result<bool, EvalError> {
    charge_nodes(meter, formula.node_count())?;
    machine.evaluate_formula(formula)
}
