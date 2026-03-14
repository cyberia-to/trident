//! Block-level TASM stack verifier for neural training.
//!
//! Executes straight-line TASM blocks on concrete u64 values using
//! Goldilocks field arithmetic. Used to verify neural-generated TASM
//! produces the same stack transformation as classical TASM.
//!
//! Not a full Triton VM — only handles the ~25 instructions that appear
//! in straight-line blocks. Crypto/IO/memory ops modeled by stack effects
//! only (correct push/pop counts, dummy values). Full verification uses
//! trisha (Triton VM execution).

pub mod equivalence;
pub mod executor;
pub mod scoring;

pub use equivalence::{diagnose_failure, generate_test_stack, verify_equivalent};
pub use executor::StackState;
pub use scoring::{score_candidate, score_neural_improvement, score_neural_output};

#[cfg(test)]
mod tests;
