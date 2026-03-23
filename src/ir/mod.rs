// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Intermediate representations for the Trident compiler.
//!
//! Four IRs form the lowering chain from typed AST to target assembly:
//!
//! ```text
//! AST → KIR → TIR → LIR (register targets)
//!                  → Tree (tree targets)
//! ```

pub mod kir;
pub mod lir;
pub mod tir;
pub mod tree;
