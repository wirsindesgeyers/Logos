use logos_core::{Budget, CheckOutcome, StepMeter};

fn main() {
    let budget = Budget::new(10, 0, 0, 0);
    StepMeter::run(&budget, |meter1| {
        StepMeter::run(&budget, |meter2| {
            std::mem::swap(meter1, meter2);
            CheckOutcome::Accepted
        })
    });
}
