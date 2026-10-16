//! Recursos lógicos conservadores, reservas falíveis e erros internos.

use logos_core::{Exhausted, StepMeter};

pub(crate) enum Failure {
    Invalid,
    Unknown(Exhausted),
}

impl From<Exhausted> for Failure {
    fn from(value: Exhausted) -> Self {
        Self::Unknown(value)
    }
}

pub(crate) type Result<T> = core::result::Result<T, Failure>;

pub(crate) struct Resources<'a> {
    pub meter: &'a StepMeter,
    remaining: u64,
}

impl<'a> Resources<'a> {
    pub fn new(meter: &'a StepMeter, memory: u64) -> Self {
        Self {
            meter,
            remaining: memory,
        }
    }

    pub fn allocate<T>(&mut self, count: usize) -> Result<Vec<T>> {
        let bytes = count.checked_mul(size_of::<T>()).ok_or(Exhausted::Memory)?;
        self.memory(bytes)?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(count)
            .map_err(|_| Exhausted::Memory)?;
        Ok(result)
    }

    pub fn push<T>(&mut self, vec: &mut Vec<T>, item: T) -> Result<()> {
        if vec.len() == vec.capacity() {
            let extra = vec.capacity().max(4);
            self.memory(extra.checked_mul(size_of::<T>()).ok_or(Exhausted::Memory)?)?;
            vec.try_reserve_exact(extra)
                .map_err(|_| Exhausted::Memory)?;
        }
        vec.push(item);
        Ok(())
    }

    fn memory(&mut self, bytes: usize) -> Result<()> {
        let bytes = u64::try_from(bytes).map_err(|_| Exhausted::Memory)?;
        self.remaining = self.remaining.checked_sub(bytes).ok_or(Exhausted::Memory)?;
        Ok(())
    }

    pub fn step(&self, count: usize) -> Result<()> {
        self.meter
            .charge(u64::try_from(count).map_err(|_| Exhausted::Steps)?)?;
        Ok(())
    }
}
