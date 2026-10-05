//! P2 e aceitação lógica sob um orçamento compartilhado.

use logos_core::{Budget, CheckOutcome, Exhausted, StepMeter};
use logos_crypto::Hash;
use logos_ledger::{Cas, CasError};
use logos_obligation::{
    CheckedObligation, DecodeError, DecodeLimits, Obligation, decode_for_check,
};

use crate::eval::{eval_checked, validate_model_context};
use crate::{EvalError, FiniteModel, ModelChecker, ModelError, ModelLimits};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Refutation,
    Consistency,
}

impl ModelChecker<'_> {
    /// Aceita exatamente quando todas as premissas são verdadeiras e goal falso.
    ///
    /// Confere `hash_raw(bytes) == expected` e decodifica os mesmos bytes.
    /// Um medidor cobre bytes, contexto e todas as avaliações; não há ledger.
    /// `size_bytes` limita a obrigação; modelo já foi construído sob ModelLimits.
    /// `mem_bytes` não mede RSS e `wall_ms` é ignorado. Erros de recurso permanecem
    /// Unknown; demais entradas inválidas são Rejected, nunca falsidade lógica.
    #[must_use]
    pub fn check_refutation(
        &self,
        bytes: &[u8],
        expected: Hash<Obligation>,
        model: &FiniteModel,
        budget: &Budget,
    ) -> CheckOutcome {
        self.check(bytes, expected, model, budget, Mode::Refutation)
    }

    /// Aceita exatamente quando todas as premissas são verdadeiras.
    ///
    /// Valida sintaxe/tipagem do goal, mas não o interpreta. Mesmo sem premissas,
    /// confere identidade do modelo e validade global das definições de valores.
    #[must_use]
    pub fn check_consistency(
        &self,
        bytes: &[u8],
        expected: Hash<Obligation>,
        model: &FiniteModel,
        budget: &Budget,
    ) -> CheckOutcome {
        self.check(bytes, expected, model, budget, Mode::Consistency)
    }

    /// Checa refutação com certificado de modelo lido e verificado pelo CAS.
    ///
    /// Impõe soma dos bytes O+M em `size_bytes`; o CAS também tem limites próprios.
    /// Modelo ausente é Rejected; falha de I/O/integridade/teto do CAS é erro.
    ///
    /// # Errors
    /// Propaga CasError sem convertê-lo em falsidade ou exaustão inventada.
    pub fn check_refutation_from_cas(
        &self,
        bytes: &[u8],
        expected: Hash<Obligation>,
        cas: &Cas,
        model_hash: Hash<FiniteModel>,
        budget: &Budget,
    ) -> Result<CheckOutcome, CasError> {
        self.check_from_cas(bytes, expected, cas, model_hash, budget, Mode::Refutation)
    }

    /// Checa consistência com certificado de modelo verificado pelo CAS.
    ///
    /// Mesmos limites e fronteiras de erro de [`Self::check_refutation_from_cas`].
    ///
    /// # Errors
    /// Propaga CasError sem convertê-lo em falsidade ou exaustão inventada.
    pub fn check_consistency_from_cas(
        &self,
        bytes: &[u8],
        expected: Hash<Obligation>,
        cas: &Cas,
        model_hash: Hash<FiniteModel>,
        budget: &Budget,
    ) -> Result<CheckOutcome, CasError> {
        self.check_from_cas(bytes, expected, cas, model_hash, budget, Mode::Consistency)
    }

    fn prepare(
        &self,
        bytes: &[u8],
        expected: Hash<Obligation>,
        budget: &Budget,
        meter: &StepMeter,
    ) -> Result<CheckedObligation, DecodeError> {
        let size = u64::try_from(bytes.len()).map_err(|_| DecodeError::Unknown(Exhausted::Size))?;
        if size > budget.size_bytes {
            return Err(DecodeError::Unknown(Exhausted::Size));
        }
        meter.charge(size).map_err(DecodeError::Unknown)?;
        decode_for_check(bytes, expected, self.vocabulary, DecodeLimits::default())
    }

    fn check(
        &self,
        bytes: &[u8],
        expected: Hash<Obligation>,
        model: &FiniteModel,
        budget: &Budget,
        mode: Mode,
    ) -> CheckOutcome {
        StepMeter::run(budget, |meter| {
            let obligation = match self.prepare(bytes, expected, budget, meter) {
                Ok(obligation) => obligation,
                Err(DecodeError::Unknown(reason)) => return CheckOutcome::Unknown(reason),
                Err(_) => return self.reject_invalid(),
            };
            self.run_model(&obligation, model, meter, mode)
        })
    }

    fn check_from_cas(
        &self,
        bytes: &[u8],
        expected: Hash<Obligation>,
        cas: &Cas,
        model_hash: Hash<FiniteModel>,
        budget: &Budget,
        mode: Mode,
    ) -> Result<CheckOutcome, CasError> {
        let mut cas_error = None;
        let outcome = StepMeter::run(budget, |meter| {
            let obligation = match self.prepare(bytes, expected, budget, meter) {
                Ok(obligation) => obligation,
                Err(DecodeError::Unknown(reason)) => return CheckOutcome::Unknown(reason),
                Err(_) => return self.reject_invalid(),
            };
            let model_bytes = match cas.get(&model_hash) {
                Ok(Some(bytes)) => bytes,
                Ok(None) => return CheckOutcome::Rejected(self.missing_model.clone()),
                Err(error) => {
                    cas_error = Some(error);
                    // O erro é preservado fora da closure do medidor.
                    return self.reject_invalid();
                }
            };
            let model_size = match u64::try_from(model_bytes.len()) {
                Ok(size) => size,
                Err(_) => return CheckOutcome::Unknown(Exhausted::Size),
            };
            // prepare já provou que bytes.len cabe em u64 e size_bytes.
            if model_size > budget.size_bytes - bytes.len() as u64 {
                return CheckOutcome::Unknown(Exhausted::Size);
            }
            if let Err(reason) = meter.charge(model_size) {
                return CheckOutcome::Unknown(reason);
            }
            let model = match FiniteModel::decode_with_limits(
                &model_bytes,
                self.vocabulary,
                ModelLimits::default(),
            ) {
                Ok(model) => model,
                Err(ModelError::Unknown(reason)) => return CheckOutcome::Unknown(reason),
                Err(_) => return self.reject_invalid(),
            };
            self.run_model(&obligation, &model, meter, mode)
        });
        match cas_error {
            Some(error) => Err(error),
            None => Ok(outcome),
        }
    }

    fn run_model(
        &self,
        obligation: &CheckedObligation,
        model: &FiniteModel,
        meter: &StepMeter,
        mode: Mode,
    ) -> CheckOutcome {
        let mut machine =
            match validate_model_context(model, self.vocabulary, obligation.vocab(), meter) {
                Ok(machine) => machine,
                Err(EvalError::Unknown(reason)) => return CheckOutcome::Unknown(reason),
                Err(_) => return self.reject_invalid(),
            };
        match all_premises_true(&mut machine, obligation, meter) {
            Ok(true) => {}
            Ok(false) => return CheckOutcome::Rejected(self.premise_false.clone()),
            Err(EvalError::Unknown(reason)) => return CheckOutcome::Unknown(reason),
            Err(_) => return self.reject_invalid(),
        }
        if mode == Mode::Refutation {
            match eval_checked(&mut machine, obligation.goal(), meter) {
                Ok(false) => CheckOutcome::Accepted,
                Ok(true) => CheckOutcome::Rejected(self.goal_true.clone()),
                Err(EvalError::Unknown(reason)) => CheckOutcome::Unknown(reason),
                Err(_) => self.reject_invalid(),
            }
        } else {
            CheckOutcome::Accepted
        }
    }
}

fn all_premises_true<'a>(
    machine: &mut crate::evaluate::Machine<'a>,
    obligation: &'a CheckedObligation,
    meter: &StepMeter,
) -> Result<bool, EvalError> {
    for (_, premise) in obligation.premises() {
        if !eval_checked(machine, premise, meter)? {
            return Ok(false);
        }
    }
    Ok(true)
}
