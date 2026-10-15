//! CheckOutcome, Verdict, Budget e StepMeter: vocabulário comum dos checkers.
//!
//! **TCB** — regras da §4 do roadmap: sem `unsafe`, sem `unwrap`/`expect`/`panic` fora de testes.
//! A lista de crates de TCB vive em `tcb/manifest.toml`.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![warn(missing_docs)]

mod budget;
mod canon;
mod epoch;
mod outcome;
mod verdict;

pub use budget::{Budget, StepMeter};
pub use epoch::{EpochError, EpochRange};
pub use outcome::{CheckOutcome, Exhausted, Reason};
pub use verdict::{Verdict, refuted_from_countermodel, verified_from_proof};
