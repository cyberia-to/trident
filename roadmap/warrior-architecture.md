---
status: draft
author: mastercyb
---

# Warrior Architecture: Core vs Tooling vs Warriors

**Related:** [[cyber-warrior]], [[cyber-stack-adoption]]
**Reference:** [reference/vm.md](../reference/vm.md), [reference/targets.md](../reference/targets.md)

## Problem

Trident currently compiles for 20+ targets in one monolithic crate.
~36k LOC, growing. Not everyone needs every target. Compilation
targets have fundamentally different architectures (stack, register,
tree, GPU, quantum) but share lowering infrastructure within
families. The boundary between core compiler, shared tooling, and
external warriors is unclear.

## Principle

Core = what you always need. Everything else is opt-in.

You always need:
1. Parse and typecheck source code
2. Compile to the primary target (nox)
3. Run programs natively (testing, bootstrap, tools)

You sometimes need:
4. Cross-compile to other provable VMs (Triton/Neptune via trisha, Miden, SP1)
5. Accelerate proving on GPU
6. Compile quantum circuits

## What belongs in core

### nox (primary provable target)

nox compiles directly from AST — no TIR. NounBuilder is the compiler
for nox. It is not a lowering, not a backend, not a plugin. It is the
native compilation path.

```
Source → AST → TypeCheck → NounBuilder → Noun
```

nox has no warrior. Execution and proving are handled by cyber stack
crates (nox, zheng) called directly. See [[cyber-warrior]] for the
warrior-cyber PoC that wires this pipeline end-to-end.

### Native (testing and bootstrap target)

Native targets (x86, ARM, RISC-V, WASM) belong in core because:

1. Testing. You run programs natively to verify correctness before
   proving on nox. Every development cycle needs this.
2. Bootstrap. The compiler itself runs natively. Self-hosting means
   the native path compiles the compiler that then runs on nox.
3. Tools. CLI utilities, formatters, LSP server — all run natively.
4. WASM. Browser playground, client-side compilation. Integral to
   developer experience.
5. No warrior needed. `trident build --target wasm` → .wasm. Run
   it directly. No proving, no external binary.

Native targets share LIR (register IR) and RegisterLowering. The
shared infrastructure is ~3k LOC. Per-target backends (instruction
encoding, ABI) are ~500 LOC each. Total ~5k LOC for 4 native targets.

This is core compiler infrastructure, not optional tooling.

## What is tooling (opt-in subcrates)

### trident-stack — provable stack VMs

StackLowering is ~80% shared across all stack VMs. Per-VM specific:
instruction names, encoding format, cost tables.

Targets: Triton VM (for the trisha/Neptune target), Miden (MASM), and
future stack VMs. nox is a tree machine — it uses NounBuilder, not
StackLowering. See reference/vm.md for the full target list.

### trident-gpu — GPU acceleration

KernelLowering → SPIR-V is shared. CUDA/Metal/Vulkan are final
backends. GPU doesn't execute trident programs — it accelerates
proving for other targets (nox GPU prover via warrior-cyber's webgpu
and metal backends).

### trident-quantum — quantum circuit compilation

Quantum processors are not warriors. They are compilation targets,
like GPUs. The pattern is the same:

- GPU: compile computation kernel → submit to GPU hardware
- Quantum: compile quantum circuit → submit to quantum hardware

Both are specialized accelerators. Neither executes general trident
programs. Neither has its own prover. Neither needs a warrior.

Quantum lowering: TIR → quantum IR → gate decomposition →
hardware-specific gate set (IBM superconducting, trapped ion, etc.)

`std.quantum.gates` already provides H, X, Y, Z, S, T, CNOT, CZ,
SWAP over Goldilocks field. The quantum backend compiles programs
that use these gates into executable quantum circuits.

The deep connection: quantum gates are unitary matrices over complex
amplitudes. Discretized to prime fields, they become field operations
— the same operations trident already compiles. A quantum circuit
over Goldilocks is natively provable on nox: you can STARK-prove
that the circuit was correctly compiled and correctly simulated.

Quantum is opt-in because not everyone needs it, but architecturally
it is trident tooling, not a warrior.

## What is a warrior (separate repos)

A warrior is an external binary that:
1. Takes a compiled artifact (Noun for nox, nox patterns trace for
   nox; TASM for Triton VM; MASM for Miden; RISC-V asm for SP1 etc.)
2. Executes it on a specific VM
3. Generates a proof of execution
4. Verifies the proof
5. Deploys to a specific chain/network

Warriors exist because execution environments are complex, have their
own dependencies (VM implementations, proof systems, chain SDKs), and
evolve independently from the compiler.

The warrior boundary: warriors receive lowered artifacts, not TIR.
Trident (core or tooling subcrate) does all compilation and lowering.
The warrior adds runtime.

### Current warriors

| Warrior | VM | Depends on | Repo |
|---------|-----|-----------|------|
| warrior-cyber | nox (PRIMARY) | trident-core (NounBuilder), nox, zheng | ~/git/warrior-cyber |
| trisha | Triton VM (Neptune target) | trident-stack[triton] | ~/git/trisha |

warrior-cyber is the primary warrior. It has three backends:
- **cpu** — AMX/NEON via honeycrisp (acpu), portable aarch64 baseline
- **webgpu** — wgpu crate + WGSL shaders, cross-platform (macOS/Linux/Windows/browser)
- **metal** — aruminium, pure Metal API (not wgpu-based), unimem zero-copy on Apple Silicon

trisha remains for Triton VM / Neptune programs. See reference/targets.md for
the full target catalogue.

### Future warriors

| Warrior | VM | Depends on |
|---------|-----|-----------|
| warrior-miden | Miden VM | trident-stack[miden] |
| warrior-sp1 | SP1 | trident-core (native RISC-V) |
| warrior-risczero | RISC Zero | trident-core (native RISC-V) |
| warrior-jolt | Jolt | trident-core (native RISC-V) |

Note: SP1, RISC Zero, Jolt all target RISC-V. The lowering is in
trident-core (native RegisterLowering). The warriors add their
specific execution + proving runtimes.

### nox is not a warrior

nox compilation is in trident-core (NounBuilder). Execution and
proving are cyber stack crates (nox, zheng) — general-purpose
libraries, not trident-specific binaries. A thin CLI wrapper may
exist but it is ~50 LOC calling three crates sequentially:

```
trident_core::noun::build() → nox::execute() → zheng::prove()
```

## Architecture

```
trident/                          Cargo workspace
│
├── trident-core/                 ALWAYS INSTALLED
│   ├── syntax/                   lexer, parser
│   ├── ast/                      AST types
│   ├── typecheck/                type checker, ModuleExports
│   ├── ir/tir/                   TIR builder + optimizer (export for tooling)
│   ├── ir/noun/                  NounBuilder (AST→Noun, nox native path)
│   ├── ir/lir/                   LIR + RegisterLowering (native targets)
│   │   ├── x86.rs
│   │   ├── arm.rs
│   │   ├── riscv.rs
│   │   └── wasm.rs
│   ├── cost/                     cost models (focus for nox, cycles for native)
│   ├── runtime/                  ProgramBundle, Runner/Prover/Verifier traits
│   ├── package/                  content addressing (hemera), store, registry
│   └── field/                    nebu bridge, PrimeField trait
│
├── trident-stack/                OPT-IN: provable stack VMs
│   ├── lower/mod.rs              shared StackLowering
│   ├── lower/triton.rs           Triton VM / Neptune target (trisha warrior)
│   ├── lower/miden.rs            MASM backend
│   └── cost/                     per-VM cost models
│
├── trident-gpu/                  OPT-IN: GPU acceleration
│   ├── spirv.rs                  shared SPIR-V generation
│   ├── cuda.rs
│   ├── metal.rs
│   └── vulkan.rs
│
├── trident-quantum/              OPT-IN: quantum circuit compilation
│   ├── circuit.rs                quantum IR
│   ├── decompose.rs              gate decomposition
│   └── backends/                 hardware-specific gate sets
│
└── trident-cli/                  the `trident` binary
    ├── main.rs
    └── Cargo.toml                feature-gated dependencies

warriors/                         SEPARATE REPOS
├── warrior-cyber/                PRIMARY: nox warrior (cpu/webgpu/metal)
├── trisha/                       Triton VM / Neptune warrior
├── warrior-miden/                Miden VM warrior
├── warrior-sp1/                  SP1 warrior
└── ...
```

## Installation profiles

```
# Default: core + nox. Minimal. Fast build. ~20k LOC.
cargo install trident-lang

# With provable stack VMs (for Triton/Miden cross-compilation)
cargo install trident-lang --features stack

# With native cross-compilation (x86, ARM already in core)
# Nothing extra needed — native is in core.

# Full (everything, including GPU and quantum)
cargo install trident-lang --features all

# Feature list:
#   default = []           core + nox + native always included
#   stack   = [triton, miden]
#   gpu     = [cuda, metal, vulkan]
#   quantum = []
#   all     = [stack, gpu, quantum]
```

## Size estimates

| Subcrate | ~LOC | External deps |
|----------|------|---------------|
| trident-core | ~22k | nebu, hemera |
| trident-stack | ~5k | trident-core |
| trident-gpu | ~3k | trident-core, spirv-tools |
| trident-quantum | ~2k | trident-core |
| trident-cli | ~2k | above via features |
| **Default install** | **~24k** | **nebu, hemera, clap** |
| **Full install** | **~34k** | **+ spirv-tools** |

vs current monolith: ~36k LOC, hemera (replacing blake3 + custom poseidon2), growing.

## Target categories summary

| Category | Targets | Where | Warrior? |
|----------|---------|-------|----------|
| Primary provable | nox | core (NounBuilder) | warrior-cyber (cpu/webgpu/metal) |
| Native | x86, ARM, RISC-V, WASM | core (LIR) | no — run directly |
| Provable stack | Triton VM (Neptune), Miden | trident-stack | trisha (Triton); warrior-miden |
| Provable register | SP1, RiscZero, Jolt | core LIR + warrior | yes |
| GPU accelerator | CUDA, Metal, Vulkan | trident-gpu | no — accelerator |
| Quantum | IBM, Google, IonQ | trident-quantum | no — submit to hw |
| Smart contract | EVM, Solana, Move | TBD | no — deploy |

See reference/vm.md for the full registry of 20+ target VMs and their integration levels.

## Migration path

1. Split current `src/` into workspace subcrates (no behavior change)
2. Move StackLowering to trident-stack
3. Feature-gate trident-cli
4. Warriors already separate (trisha exists)
5. Add NounBuilder to trident-core
6. Add trident-quantum when std.quantum matures

Each step is a refactor commit. No functionality changes during split.
Tests must pass after every step.
