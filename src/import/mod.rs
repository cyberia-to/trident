// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Import module: reads serialized MIR from rsc and translates to TIR.
//!
//! Pipeline: `.mir.json` → `mir_to_tir()` → `Vec<TIROp>`

pub mod mir;
pub mod structurize;
pub mod types;
