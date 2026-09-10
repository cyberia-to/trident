// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Intermediate representations for the Trident compiler.
//!
//! Two real lowering paths from typed AST:
//!
//! ```text
//! AST → TIR (TIRBuilder) → stack target (Triton, in trisha)
//! AST → Tree (NoxCompiler) → nox — the default, bypasses TIR
//! ```

pub mod tir;
pub mod tree;
