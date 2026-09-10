// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Import module: a second front end into trident's representation.
//!
//! `mir2nox` reads Rust MIR JSON (from `rsc --emit=mir-rs`) and produces a
//! nox formula directly — the counterpart to `.tri` source going through
//! `ir::tree::lower::nox`. This is core infrastructure, not a silicon
//! emitter: it produces trident's own representation rather than
//! translating out to a machine.

pub mod mir2nox;
