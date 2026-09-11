# Warrior API

A warrior is an external binary that owns execution, proving, verification
and deployment on one battlefield. Trident compiles; warriors fight.

Two contracts hold between them:

| contract | direction | where |
|---|---|---|
| process | `trident build/run/prove/verify` → `<warrior>` on PATH | [targets.md](targets.md) §Warriors |
| library | warrior links `trident-lang` as a Rust crate | this document |

This document is the library contract: what a warrior may depend on,
what the core promises not to break, and where the line between them
runs.

## The line

A thing belongs to the **core** if it produces trident's own
representation or every warrior consumes it: the front end (lexer,
parser, AST), the type checker, TIR and its optimizer, nox lowering
(nox is the semantics every other target is checked against), the cost
framework, `audit`, the LSP, the package system, and the MIR import.

A thing belongs to a **warrior** if it knows one machine: the lowering
from TIR to that ISA, its cost model and verifier, its runtime, its
prover, its deployment path, and its hand-written baselines.

The [2026-09-11 ownership review](../docs/explanation/target-ownership.md)
records remaining violations of this boundary and a proposed resource layout.
Its proposed interfaces are not implemented API guarantees.

## Surface

Everything below is `pub` in `trident-lang` and covered by the stability
promise. Paths are as a warrior writes them.

### Compilation

```rust
trident::compile_to_bundle(entry: &Path, options: &CompileOptions)
    -> Result<ProgramBundle, Vec<Diagnostic>>
```

The nox entry point returns assembly, entry point, function hashes, costs,
source hash and `reads_state`. Stack warriors obtain per-module TIR through
`build_tir_modules`, lower/link it, then call:

```rust
trident::bundle_with_assembly(entry: &Path, options: &CompileOptions,
    assembly: String, cost: BundleCost) -> Result<ProgramBundle, Vec<Diagnostic>>
```

The core derives source metadata once through this shared contract. The warrior
supplies its own assembly and measured or explicitly unavailable costs.

```rust
trident::build_tir(source: &str, filename: &str, options: &CompileOptions)
    -> Result<Vec<TIROp>, Vec<Diagnostic>>
trident::build_tir_project(entry: &Path, options: &CompileOptions)
    -> Result<Vec<TIROp>, Vec<Diagnostic>>
```

For a warrior that owns its own lowering: typed, optimized TIR — a flat
op list, already monomorphized, with control flow structured. The
warrior turns it into its ISA. `build_tir_project` resolves imports;
`build_tir` is the single-file form.

`CompileOptions` carries the `TerrainConfig` and the cfg flags. Build it
with `trident::CompileOptions::default()` and set `target_config`, or
take it from `trident::target` (`TerrainConfig::nox()`,
`TerrainConfig::triton()`, or a `vm/<engine>/target.toml` load).

`source_options(input, options)` resolves a project directory or a source file,
applies the project's named profile and dependency paths, and preserves the
caller's chosen terrain. `module_sources` carries version-matched warrior
modules by dotted name; module resolution uses these when no source file exists.
Core libraries and target declarations are embedded in the compiler package.

CLI target precedence is explicit register/target selection, then project
target, then nox. Selecting nox explicitly overrides a Triton project. Missing
warriors, unknown terrain, and failed child processes produce nonzero exit.

### Runtime

```rust
trident::runtime::{ProgramBundle, ProgramInput, ExecutionResult, ProofData}
trident::runtime::{Runner, Prover, Verifier, Guesser, Deployer}
```

`ProgramBundle` is the pipeline boundary. The five traits are the shapes
a warrior implements; nothing in the core calls them — the CLI delegates
by process, not by trait object. Implement the ones your battlefield
supports.

### Utilities

```rust
trident::hash::content_hash_bytes(data: &[u8]) -> [u8; 32]   // cyber-hemera
trident::hash::ContentHash                                    // hex, parse
trident::field::proof::{Claim, padded_height, ...}            // proof sizing
trident::target::{TerrainConfig, UnionConfig, Arch}
```

## Feature contract

A warrior takes trident-lang **without default features**:

```toml
trident-lang = { version = "0.3", default-features = false }
```

The default feature set is empty. Opt-in `neural` enables the shared model,
training and inference framework; warriors provide an ISA vocabulary, grammar,
equivalence oracle and costs. Ordinary build/run/prove installations require
no neural backend. The compilation and runtime surface is available without it.

## Stability

- The paths above follow semver on `trident-lang`. Breaking them is a
  major bump with a CHANGELOG entry naming the warrior-visible change.
- `TIROp` is an **in-process** contract: a Rust type, not a wire format.
  A warrior links the same `trident-lang` build it compiles against.
  Serialization is deliberately absent — the day a non-Rust warrior
  exists, it gets a defined encoding and a version tag; until then a
  schema would be a promise with no reader.
- `ProgramBundle` **does** have a wire form (JSON) because it crosses
  the process boundary in `trident run/prove/verify`. Fields are
  additive and `from_json` ignores unknown keys. Function metadata, named cost
  values, assembly and state declarations survive transport. Malformed JSON,
  duplicate identity keys and invalid field types are rejected. Named cost
  tables are maps; their textual order carries no semantics.
- Everything else in `trident-lang` is internal. Depending on it is
  allowed and unsupported: it moves without notice.

## Warriors today

| warrior | battlefield | uses |
|---|---|---|
| [joy](https://github.com/cyberia-to/joy) | nox · cyber | `runtime::*`, `target`, `field::proof` — takes the bundle, executes on cyber-nox, proves with zheng |
| [trisha](https://github.com/cyberia-to/trisha) | Triton VM · Neptune | per-module TIR, owned lowering/linker and runtime, source metadata via `bundle_with_assembly` |

Neptune runtime modules and Triton baselines live in Trisha. Target declarations
stay in the core registry. The release audit records current proof-support
limits; this API contract does not certify the cryptography of a linked warrior.
