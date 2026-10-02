//! Orçamentos de execução e medidor de passos lógicos com latch de exaustão.

use crate::outcome::{CheckOutcome, Exhausted};
use core::cell::Cell;

/// Orçamento computacional e físico alocado para a execução de checadores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Budget {
    /// Limite de passos lógicos computacionais.
    pub steps: u64,
    /// Limite de consumo de memória em bytes.
    pub mem_bytes: u64,
    /// Limite de tamanho de certificado ou dados em bytes.
    pub size_bytes: u64,
    /// Limite de tempo de relógio em milissegundos.
    /// Ignorado por checkers internos; somente supervisores externos usam tempo.
    pub wall_ms: u64,
}

impl Budget {
    /// Cria uma nova especificação de orçamento com os limites fornecidos.
    #[must_use]
    pub const fn new(steps: u64, mem_bytes: u64, size_bytes: u64, wall_ms: u64) -> Self {
        Self {
            steps,
            mem_bytes,
            size_bytes,
            wall_ms,
        }
    }
}

/// Medidor de passos lógicos com latch de exaustão irreversível.
///
/// Criado exclusivamente através de [`StepMeter::run`]. A closure de checagem
/// recebe uma referência compartilhada `&StepMeter`, impedindo que o medidor seja
/// reinicializado ou substituído via `std::mem::swap`.
///
/// Qualquer cobrança que exceda o saldo restante dispara um latch permanente que
/// faz com que a execução finalize **sempre** com [`CheckOutcome::Unknown(Exhausted::Steps)`].
pub struct StepMeter {
    remaining: Cell<u64>,
    exhausted: Cell<bool>,
}

impl StepMeter {
    const fn new(steps: u64) -> Self {
        Self {
            remaining: Cell::new(steps),
            exhausted: Cell::new(false),
        }
    }

    /// Executa um checker sob o orçamento especificado e finaliza o resultado.
    ///
    /// Se o limite de passos for excedido durante a execução, o latch irreversível
    /// prevalece sobre qualquer valor retornado pela closure (mesmo `Accepted`,
    /// `Rejected` ou outro `Unknown`), forçando o retorno de
    /// [`CheckOutcome::Unknown(Exhausted::Steps)`].
    ///
    /// Apenas `budget.steps` é imposto aqui. A closure deve cobrar antes do trabalho
    /// e retornar ao esgotar; trabalho não contabilizado não é detectado.
    pub fn run(budget: &Budget, checker: impl FnOnce(&Self) -> CheckOutcome) -> CheckOutcome {
        let meter = Self::new(budget.steps);
        let outcome = checker(&meter);
        if meter.exhausted.get() {
            CheckOutcome::Unknown(Exhausted::Steps)
        } else {
            outcome
        }
    }

    /// Deduz `steps` do saldo disponível do medidor.
    ///
    /// # Erros
    /// Retorna [`Err(Exhausted::Steps)`](Exhausted::Steps) se `steps` for maior que o saldo
    /// restante, disparando irreversivelmente o latch de exaustão.
    /// Após o disparo do latch, todas as cobranças subsequentes retornam erro, inclusive
    /// cobranças de zero passos.
    pub fn charge(&self, steps: u64) -> Result<(), Exhausted> {
        if self.exhausted.get() {
            return Err(Exhausted::Steps);
        }

        let remaining = self.remaining.get();
        if steps <= remaining {
            self.remaining.set(remaining - steps);
            Ok(())
        } else {
            self.exhausted.set(true);
            Err(Exhausted::Steps)
        }
    }
}

impl core::fmt::Debug for StepMeter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("StepMeter")
            .field("remaining", &self.remaining.get())
            .field("exhausted", &self.exhausted.get())
            .finish()
    }
}
