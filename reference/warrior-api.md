# Warrior API

A warrior is an external binary that owns execution, proving, verification
and deployment on one battlefield. Trident compiles; warriors fight.

Two contracts hold between them:

| contract | direction | where |
|---|---|---|
| process | `trident run/prove/verify` → `<warrior>` on PATH | [targets.md](targets.md) §Warriors |
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

## Surface

Everything below is `pub` in `trident-lang` and covered by the stability
promise. Paths are as a warrior writes them.

### Compilation

```rust
trident::compile_to_bundle(entry: &Path, options: &CompileOptions)
    -> Result<ProgramBundle, Vec<Diagnostic>>
```

The primary entry point: a multi-module project in, a `ProgramBundle`
out — assembly text, entry point, per-function hashes, cost, source
hash, `reads_state`. A warrior that takes finished assembly needs
nothing else.

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

`default = ["neural"]` pulls burn and the cubecl/wgpu graph under it —
the neural optimizer, which is a compiler-side tool. A warrior never
trains, so it never needs that graph, and dropping it cuts the
dependency tree to a fraction. Everything in this document is available
without default features.

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
  additive and `from_json` ignores keys it does not know. It is also
  lossy today: the `functions` array and the cost table values/names do
  not survive a round trip (they are re-derived, not read back). A
  warrior that needs them must take the bundle in memory from
  `compile_to_bundle`, not through JSON.
- Everything else in `trident-lang` is internal. Depending on it is
  allowed and unsupported: it moves without notice.

## Warriors today

| warrior | battlefield | uses |
|---|---|---|
| [joy](https://github.com/cyberia-to/joy) | nox · cyber | `runtime::*`, `target`, `field::proof` — takes the bundle, executes on cyber-nox, proves with zheng |
| [trisha](https://github.com/cyberia-to/trisha) | Triton VM · Neptune | `runtime::*`, `target`, `hash::content_hash_bytes`, `field::proof` — takes the bundle's TASM, executes and proves on Triton VM |

Neither takes TIR yet: today the core lowers to TASM and hands over
finished assembly. Moving the Triton lowering into trisha — so the core
stops at TIR for stack targets — is the next step
(`.claude/plans/warrior-owns-lowering.md`).
