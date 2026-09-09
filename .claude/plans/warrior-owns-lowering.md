# Warrior owns lowering — Triton/Neptune into trisha, silicon into its own folder

**STATUS: proposal, awaiting owner sign-off (2026-09-09).**

**One sentence:** trident-lang becomes the language — front end, nox
lowering, audit, LSP — and every other machine is a warrior that
compiles as well as runs, proves, verifies and deploys; the first and
only such warrior is trisha, which absorbs everything Triton/Neptune;
the 28 native-silicon emitters leave the crate into one unpublished
folder, no 28 warriors.

## Audited facts (2026-09-09, agents + direct reads)

| fact | state |
|---|---|
| pipeline in CLAUDE.md | claims `AST → TypeCheck → KIR → TIR → LIR → Target`. Code: KIR = 30 lines of doc + `create_kernel_lowering() → None`; LIR = `tir_to_lir` is `todo!()`; nothing produces or consumes either. Two real paths: stack `ast::File → TIRBuilder → TASM`, tree `ast::File → NoxCompiler → Noun` |
| what TIR lowering needs | `&ast::File` + `TerrainConfig` (xfield/digest widths, stack depth) + two typecheck tables (`mono_instances`, `call_resolutions: Vec<MonoInstance{name, size_args}>`). Contracts unused. No serde anywhere on AST/TIR |
| nox path | `NoxCompiler::compile_file(&ast::File)` — uses no typecheck output at all; the `TreeLowering` trait impl is a stub nothing calls |
| dispatch | `src/api/mod.rs::compile_with_options` — an `if arch == Tree` else stack; `Arch::Register` silently falls to Triton. Duplicated in `compile_project_with_options` |
| Triton footprint | dedicated: `ir/tir` 6.8k, `neural` 5.7k (trains on TASM), `cost/model/triton.rs` + `stack_verifier` 1.3k, `baselines/triton` 10.8k .tasm, `os/neptune` 2.1k .tri, trisha 1.7k + 68k vendored. Threaded: 30 more files (`cli/bench.rs` 61 hits, `cli/train.rs`, `api/tests/{neptune,compile,prove,features}.rs`, `package/cache.rs`, `config/target/*`, `cli/audit.rs`, `cost/scorer.rs`, `deploy/*`, `typecheck/builtins.rs` xfield/Digest[5], `lsp/builtins.rs`) |
| silicon emitters | `src/compile/` 14 379 LOC, 27 files, all born 2026-03-29 in four commits; original home per `nox-native-compile.md` was `nox/rs/compile/`. Input = nox `Reduction` only, zero `crate::` refs — a leaf. Phase-1 patterns only (0,1,4,5–14); `hash`/`divine`/cons/look → `UnsupportedPattern`. Emit text/bytes, no execution, no trace. Tests in 4 of 27 files. Only caller: `trident compile` / `trident mir` (`cli/compile.rs`). `mir2nox` is the inverse direction (rustc MIR JSON → nox text), no nox dep, duplicates the orphaned `src/import/` |
| dead weight | `src/gpu` 325 LOC (nobody calls it; sole reason for `pollster`), `src/import` not in the module tree, deps `bytemuck` `petgraph` `statrs` unused, `blake3` benches-only; `burn`/`wgpu` leak into `cli/{bench,build,train}.rs` directly |
| trisha | `~/cyber/trisha`: cli/ rs/ wgpu/ (accelerator 1.7k) honeycrisp/ patches/; vendors triton-vm; run/prove/verify/deploy over ProgramBundle; **trisha#1 open: build broken** (vendored twenty-first serde) |

Conclusion that shapes the plan: there is no IR to stabilise. The
contract a warrior needs is trident-lang's *library* API — parse,
typecheck, `ast::File`, `TerrainConfig`, `MonoInstance` — which trisha
already links (`path = "../trident"`). No wire schema, no dlopen.

## Decision (proposed)

1. **One warrior = one binary = everything about its machine.** trisha
   gains `build` (and `cost`): `.tri → TASM bundle`, cost bill, then the
   existing run/prove/verify/deploy. `trident build --target triton`
   delegates to `trisha build` the way `trident prove` already delegates —
   warrior found by name from `vm/triton/target.toml`.
2. **nox stays in the core.** Lowering to nox is the meaning of a trident
   program; every other target is a translation checked against nox
   (differential harness). Core = syntax → typecheck → `ir/tree` (nox) →
   `.nox` bundle, plus `audit`, `cost` in reductions, LSP, package. joy is
   unchanged.
3. **The contract is the library, documented.** `trident::front::{parse,
   check}` → `(ast::File, ModuleExports)`, `TerrainConfig`, `MonoInstance`
   become the declared warrior-facing API in `reference/` (semver, no
   serialization). Serialization only if a non-Rust warrior ever appears.
4. **Silicon leaves the crate into one folder.** `src/compile/` +
   `cli/compile.rs` + `mir2nox` → `trident/silicon/` — a workspace crate
   `trident-silicon`, `publish = false`, depends on `cyber-nox` only, its own
   `silicon` binary carrying today's `compile`/`mir` CLI. Not 28 warriors:
   the emitters have no execution and no trace, so they are sketches, and
   the folder says so. Later home may be `nox/` (their original plan) — a
   `git mv` then, not now.
5. **Target registry stays configuration in trident.** `vm/*/target.toml`
   and `os/*/target.toml` keep declaring engines, unions and their warrior;
   a target whose warrior is absent is refused with the install hint.

## What moves

| from (trident) | to | LOC |
|---|---|---|
| `src/ir/tir/**` (builder, optimize, stack, encode, linker, neural hooks) | `trisha/rs/tir/` | 6 811 |
| `src/cost/model/triton.rs`, `src/cost/stack_verifier/**` | `trisha/rs/cost/` | 1 319 |
| `src/neural/**` + `burn wgpu rayon rkyv` deps + `cli/{train,bench}` neural parts | `trisha/neural/` (crate `trisha-neural`) | 5 724 |
| `baselines/triton/**`, `benches/references` TASM comparisons, `cli/bench.rs` | `trisha/baselines/`, `trisha bench` | 10 755 + 1 019 |
| `os/neptune/**` (.tri), `src/api/tests/neptune.rs`, `deploy/**` Neptune parts | `trisha/os/neptune/`, trisha tests | 2 098 + ~600 |
| `src/cli/trisha.rs`, Triton arms of `cli/{audit,build,mod}.rs` | trisha cli | ~500 |
| `src/compile/**`, `src/cli/compile.rs`, `Command::{Compile,Mir}` | `trident/silicon/` | 14 379 + 280 |
| `src/ir/kir`, `src/ir/lir`, `src/gpu`, `src/import` | deleted (history keeps them) | ~1 300 |
| `bytemuck petgraph statrs pollster` (+ `serde serde_json` with mir2nox, `blake3` → dev) | out of `[dependencies]` | — |

Stays, made generic: `typecheck/builtins.rs` and `lsp/builtins.rs` read
digest/xfield widths from `TerrainConfig` instead of naming Triton;
`cost/scorer.rs` scores in the target's unit; `config/target` unchanged.

Core deps after: `clap ariadne tower-lsp tokio nox nebu hemera`. Miden
shares TIR with Triton → lives in trisha's tir until a Miden warrior
exists (out of scope). `Arch::Register` (LIR) has no lowering today and
loses its dead scaffold; the enum variant stays for the registry.

## Sequence

| step | scope | sessions | gate |
|---|---|---|---|
| S0 attic | `silicon/` crate; delete kir/lir/gpu/import + 4 deps; README "28 backends" section rewritten as sketch crate; `trident compile` removed from the main binary | 1 | nox path byte-identical: `tests/nox_surface`, `differential`, fresh `cargo install` smoke |
| S1 trisha builds | close trisha#1 (vendored twenty-first serde), installed binary = warrior CLI | ½–1 | `trisha run` on `benches/references` |
| S2 contract | `trident::front` API + `reference/warrior-api.md`; `compile_with_options` loses the stack branch behind a `Lowering` seam | 1 | trisha builds against it without touching `ir/tir` yet |
| S3 the move | tir, cost, stack_verifier, neural, baselines, os/neptune, tests → trisha; `trisha build`/`cost`/`bench`; `trident build --target triton` delegates | 3–4 | trisha `cargo test`; every `benches/references` program: `trisha run` == `joy run`; hand-baseline ratios unchanged |
| S4 core cleanup | typecheck/lsp target-generic; CLAUDE.md pipeline line and Key Modules table corrected; roadmap; CHANGELOG; trident-lang 0.3.0, trisha 0.2.0 | 1–2 | zero warnings, `trident audit`, docs sweep |

**Total 7–9 sessions.** Kelvin: restructuring, no language change — stack
temperature unchanged; trident-lang 0.3.0 because the CLI loses
`compile`/`mir` and `--target triton` now needs trisha installed.

## Verification (per repo rules)

Every step: `cargo check` zero warnings · `cargo test` · `trident bench`
(through trisha after S3) no regressions · `trident audit` · differential
triton×nox on `benches/references` (unblocked by S1) · fresh
`cargo install trident-lang cyber-joy` in /tmp. Branches `feat/*`, PRs,
no direct master commits. Two agents never share a directory: trident
side by `syntax/ ast/+typecheck/ ir/ cost/+verify/ cli/ package/ lsp/
docs/`, trisha side its own repo.

## Open for the owner

- `silicon/` in trident (proposed) vs straight into `nox/` now.
- neural into trisha (proposed — it learns TASM) vs its own crate.
- Delete `src/import/` (dead, mir_format never wired) vs move with
  mir2nox into `silicon/`.

## Settled

- Warriors own lowering; one binary per machine. (owner, 2026-09-09)
- Neptune + Triton go to trisha. (owner, 2026-09-09)
- Emitters out of the crate into one folder, no 28 warriors. (owner, 2026-09-09)
