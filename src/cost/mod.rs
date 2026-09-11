// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Cost analysis.
//!
//! nox (the default target) prices programs in reductions — see `nox.rs`
//! and `nox_cost_project`. The Algebraic-Execution-Table cost model
//! (`ProgramCost`, `CostAnalyzer`, per-target `CostModel`) moved to trisha
//! with the Triton lowering it priced (trident's own copy had exactly one
//! implementation of that trait — Triton — same shape as `StackLowering`
//! before it moved): reference/warrior-api.md,
//! .claude/plans/warrior-owns-lowering.md.

pub mod nox;
