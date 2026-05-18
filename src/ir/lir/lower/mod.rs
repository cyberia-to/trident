// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! RegisterLowering: consumes `&[LIROp]` and produces target machine code.
//!
//! Each register-machine target implements `RegisterLowering` to perform
//! instruction selection, register allocation, and binary encoding.
//!
//! This is the register-machine counterpart of `tir::lower::StackLowering`,
//! which produces assembly text for stack machines.

use super::LIROp;
use crate::compile::arm64_lir::Arm64LirBackend;

/// Lowers LIR operations into target machine code (binary).
pub trait RegisterLowering {
    /// The target name (e.g. "x86_64", "arm64", "riscv64").
    fn target_name(&self) -> &str;

    /// Lower a sequence of LIR operations into machine code bytes.
    fn lower(&self, ops: &[LIROp]) -> Vec<u8>;

    /// Lower to assembly text for debugging. Default uses Display.
    fn lower_text(&self, ops: &[LIROp]) -> Vec<String> {
        ops.iter().map(|op| format!("{}", op)).collect()
    }
}

/// Create a register-lowering backend for the given target name.
pub fn create_register_lowering(target: &str) -> Option<Box<dyn RegisterLowering>> {
    match target {
        "arm64" | "aarch64" => Some(Box::new(Arm64LirBackend::new())),
        _ => None,
    }
}
