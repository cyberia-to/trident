// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! KIR — Kernel IR for data-parallel GPU targets.
//!
//! KIR is not a separate IR representation — it takes TIR directly
//! and wraps it in a GPU compute kernel. The program stays scalar;
//! parallelism is across program instances, not within one execution.
//!
//! Pipeline:
//! ```text
//! AST → TIR ─→ StackLowering     → Vec<String>  (stack targets)
//!           ├→ LIR → RegisterLow  → Vec<u8>      (register targets)
//!           ├→ TreeLowering       → Noun         (tree targets: Nock)
//!           └→ KIR → KernelLow    → String        (GPU kernel source)
//! ```
//!
//! Each GPU thread runs one copy of the Trident program:
//! - ReadIo  → buffer[thread_id * input_width + i]
//! - WriteIo → buffer[thread_id * output_width + i]
//! - All other ops → scalar computation per thread
//!
//! Supported targets:
//! - CUDA (PTX) — NVIDIA GPUs
//! - Metal (MSL) — Apple Silicon GPUs
//! - Vulkan (SPIR-V) — cross-platform GPUs

pub mod lower;
