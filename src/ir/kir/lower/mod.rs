// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! KernelLowering: wraps scalar TIR programs into GPU compute kernels.
//!
//! Each GPU target implements `KernelLowering` to emit kernel source
//! that runs N instances of the same Trident program in parallel.
//! The program itself stays scalar — parallelism is across instances,
//! not within a single execution.
//!
//! This is the data-parallel counterpart of:
//! - `tir::lower::StackLowering` — stack targets → assembly text
//! - `lir::lower::RegisterLowering` — register targets → machine code

use crate::tir::TIROp;

/// Lowers TIR operations into a GPU compute kernel (source text).
///
/// The kernel wraps one Trident program for batch execution:
/// each GPU thread runs one instance with its own inputs/outputs.
pub trait KernelLowering {
    /// The target name (e.g. "cuda", "metal", "vulkan").
    fn target_name(&self) -> &str;

    /// Lower a scalar TIR program into GPU kernel source code.
    /// The returned string is a complete, compilable kernel.
    fn lower(&self, ops: &[TIROp]) -> String;
}

/// Create a kernel-lowering backend for the given target name.
pub fn create_kernel_lowering(_target: &str) -> Option<Box<dyn KernelLowering>> {
    // No backends implemented yet. Implementers add match arms here.
    None
}
