# Warrior owns lowering — Triton/Neptune into trisha, silicon into its own folder

**STATUS: proposal, awaiting owner sign-off (2026-09-10, rev 3).**

**One sentence:** trident-lang is the language and its compilation
harness — front end, TIR, optimizer, neural optimizer, cost framework,
audit, LSP, nox lowering — and every other machine is a warrior that owns
the last mile: TIR → its ISA, its verifier and cost model, run, prove,
verify, deploy. The first and only such warrior is trisha, which absorbs
everything that knows Triton or Neptune. The 28 native-silicon emitters
leave the crate into one unpublished folder; no 28 warriors.

## Audited facts (2026-09-09/10, agents + direct reads)

| fact | state |
|---|---|
| pipeline in CLAUDE.md | claims `AST → TypeCheck → KIR → TIR → LIR → Target`. Code: KIR = 30 lines of doc + `create_kernel_lowering() → None`; LIR = `tir_to_lir` is `todo!()`; nothing produces or consumes either. Two real paths: stack `ast::File → TIRBuilder → optimize → TASM`, tree `ast::File → NoxCompiler → Noun` |
| TIR | `ir/tir`: builder 3 318, optimize 712, stack model 522, encode (TIR → block tensors) 416, linker 248, neural hooks 305, **lower 859 of which `lower/triton.rs` is 328** — the only file that knows TASM. `create_stack_lowering(_)` ignores its argument and always returns Triton |
| neural | `compile(tir_ops: &[TIROp], baseline_tasm) → CompileResult`. model 2 101 (GNN over the TIR graph), `data/tir_graph` 667 (zero Triton mentions), inference 547, training 1 567 (94 Triton hits: reward = cycles, oracle = `cost/stack_verifier` 1 170 LOC = a TASM block executor), output vocabulary = TASM tokens |
| nox path | `NoxCompiler::compile_file(&ast::File)` bypasses TIR; uses no typecheck output; `TreeLowering` impl is a stub nothing calls |
| dispatch | `src/api/mod.rs::compile_with_options`: `if arch == Tree` else stack; `Arch::Register` silently falls to Triton; duplicated in `compile_project_with_options` |
| Triton threaded elsewhere | 30 files: `cli/bench.rs` 61 hits, `cli/train.rs`, `api/tests/{neptune,compile,prove,features}.rs`, `package/cache.rs`, `config/target/*`, `cli/audit.rs`, `cost/scorer.rs`, `deploy/*`, `typecheck/builtins.rs` (xfield, Digest[5]), `lsp/builtins.rs`, `cli/trisha.rs` |
| assets | `baselines/triton` 10 755 lines .tasm, `os/neptune` 2 098 lines .tri, `cost/model/triton.rs` |
| silicon emitters | `src/compile/` 14 379 LOC, 27 files, born 2026-03-29 in four commits; original home per `nox-native-compile.md` was `nox/rs/compile/`. Input = nox `Reduction` only, zero `crate::` refs — a leaf. Phase-1 patterns only (0,1,4,5–14); `hash`/`divine`/cons/look → `UnsupportedPattern`. Emit text/bytes, no execution, no trace. Tests in 4 of 27 files. Only caller: `trident compile` (`cli/compile.rs`). `mir2nox` (720) is the inverse direction — rustc MIR JSON → nox — and duplicates the orphaned `src/import/` (1 067, not in the module tree) |
| dead weight | `src/gpu` 325 LOC (no callers; sole reason for `pollster`), `src/ir/kir`, `src/ir/lir`; deps `bytemuck` `petgraph` `statrs` unused, `blake3` benches-only; `burn`/`wgpu` imported directly by `cli/{bench,build,train}.rs` |
| trisha | `~/cyber/trisha`: cli/ rs/ wgpu/ (accelerator 1 728) honeycrisp/ patches/; vendors triton-vm (68k); run/prove/verify/deploy over ProgramBundle; **trisha#1 open: build broken** (vendored twenty-first serde) |

## Decision (proposed)

1. **One warrior = one binary = the last mile of its machine.** trisha
   gains `build` and `cost`: takes TIR, emits TASM, links, bills cycles;
   then run/prove/verify/deploy as today. `trident build --target triton`
   delegates to `trisha build` the way `trident prove` already delegates;
   the warrior is found by name from `vm/triton/target.toml`.
2. **The core keeps the harness.** Criterion: *a thing stays in the core if
   it produces the core's representation or every warrior consumes it.*
   Front end, typecheck, TIR builder + optimizer + stack model + encode,
   nox lowering (`ir/tree` — nox is the semantics; the differential
   harness checks every other target against it), cost framework
   (`cost/scorer`, `cost/model/mod`), audit, LSP, package, MIR import.
3. **The contract is TIR.** Warrior-facing API of trident-lang:
   `trident::front::compile_to_tir(source, TerrainConfig) → Vec<TIROp>`
   (+ `TerrainConfig`, `ModuleExports`), documented in
   `reference/warrior-api.md`; semver, no serialization — trisha links
   trident-lang as it does today. A flat op list is a smaller surface than
   the recursive AST. `create_stack_lowering` and the `StackLowering` trait
   go: the core stops at TIR for stack targets.
4. **neural is the harness; the target is a trait.** Model, TIR graph,
   inference, training loop stay in the core behind
   `neural::Target { vocabulary, verify_block, cost }`. Triton implements
   it in trisha: TASM token vocabulary, `stack_verifier` (the TASM block
   executor), cycle cost. Core training/inference never names an ISA.
   `burn`/`wgpu` stay core deps, gated behind a default-on `neural`
   feature so `cargo install trident-lang --no-default-features` is light.
5. **MIR import is core infrastructure.** `mir2nox` + the orphaned
   `src/import/` merge into one `src/import/` whose output is trident's
   representation (AST → TIR; until then, nox as the semantics), so
   trisha gets Rust programs the same way it gets `.tri` ones.
6. **Silicon leaves the crate into one folder.** `src/compile/` (minus
   mir2nox) + `cli/compile.rs` → `trident/silicon/`, a workspace crate
   `trident-silicon`, `publish = false`, depends on `cyber-nox` only, its
   own `silicon` binary carrying today's `compile` CLI. Not 28 warriors:
   the emitters have no execution and no trace, so they are sketches, and
   the folder says so. Later home may be `nox/` — a `git mv` then.
7. **Target registry stays configuration in trident.** `vm/*/target.toml`
   and `os/*/target.toml` keep declaring engines, unions and their
   warrior; a target whose warrior is absent is refused with the install
   hint.

## What moves

| from (trident) | to | LOC |
|---|---|---|
| `src/ir/tir/lower/triton.rs`, `linker.rs`, Triton arms of `lower/mod.rs` | `trisha/rs/lower/` — `trisha build` | ~700 |
| `src/cost/model/triton.rs`, `src/cost/stack_verifier/**` | `trisha/rs/cost/`, `neural::Target` impl | ~1 300 |
| TASM vocabulary + cycle reward out of `src/neural/{training,inference,data}` | `trisha/rs/neural_target.rs` | ~300 (est.) |
| `baselines/triton/**`, `benches/references` TASM comparisons, `cli/bench.rs` Triton arms | `trisha/baselines/`, `trisha bench` | 10 755 + ~600 |
| `os/neptune/**` (.tri), `src/api/tests/neptune.rs`, Neptune parts of `deploy/**` | `trisha/os/neptune/`, trisha tests | 2 098 + ~600 |
| `src/cli/trisha.rs`, Triton arms of `cli/{audit,build,mod}.rs` | trisha cli | ~500 |
| `src/compile/**` (minus mir2nox), `src/cli/compile.rs`, `Command::Compile` | `trident/silicon/` | 13 659 + 240 |
| `src/compile/mir2nox.rs` + `src/import/**` | one `src/import/` in the core (`trident mir` stays) | 720 + 1 067 |
| `src/ir/kir`, `src/ir/lir`, `src/gpu` | deleted (history keeps them) | ~1 200 |
| `bytemuck petgraph statrs pollster` (`blake3` → dev) | out of `[dependencies]` | — |

Rust actually leaving trident-lang for trisha: **~3.5k LOC** plus the
`.tasm`/`.tri` assets. Stays, made generic: `typecheck/builtins.rs` and
`lsp/builtins.rs` read digest/xfield widths from `TerrainConfig` instead
of naming Triton; `cost/scorer.rs` scores in the target's unit. Miden
keeps sharing TIR in the core; its lowering is a future warrior's.
`Arch::Register` loses its dead LIR scaffold; the variant stays for the
registry.

Core deps after: `clap ariadne tower-lsp tokio serde serde_json nox nebu
hemera` + `burn wgpu rayon rkyv` under `neural`.

## Sequence

| step | scope | sessions | gate |
|---|---|---|---|
| S0 attic — **DONE** (PR TBD, 2026-09-10) | `silicon/` workspace crate (25 emitters, own `silicon` binary); `mir2nox` merged into `src/import/` (the orphaned `mir.rs`/`structurize.rs`/`types.rs` deleted — needed `mir-format`, a `publish = false` crate that can never be a trident-lang manifest dependency); deleted `ir/kir`, `ir/lir` (both dead), `src/gpu` (325 LOC, zero callers); dropped `bytemuck`/`petgraph`/`statrs`/`pollster`, `blake3` → dev-deps; README + landing rewritten (25, not 28 — `x64-sysv`/`ptx-parallel`/`ane-batch` are CLI variants, not separate backends); `trident compile` removed from main binary | 1 (spent ~1) | 955+34+9+13+3+17 tests green, zero warnings, `differential`/`nox_surface` unchanged. **Neural feature-gate deferred to S2** (bundling it with the `neural::Target` extraction avoids gating twice). **Found, not fixed:** `silicon`/old `trident compile` hangs on every invocation, reproduced identically on unmodified master — trident#32, pre-existing, out of scope here |
| S1 trisha builds — **DONE** (trident#34, trisha#2 #3, trisha#1 closed, 2026-09-10) | Root cause of trisha#1 was one manifest line: vendored twenty-first ships `crate-type = ["cdylib", "rlib"]`, so rustc saw two versions of serde and rand (`BFieldElement: Serialize is not satisfied`, `StandardUniform: Distribution` unsatisfied, `can't find crate for triton_isa`). `patches/apply.nu` [T0] now rewrites it to a plain rlib, surviving re-vendoring. Three separate breakages also fixed: API drift (poseidon2→hemera, `reads_state`), `triton` missing from `default` (why the installed binary had no warrior CLI), and a `cfg` on a non-existent `trisha-rs` feature. Dead code removed. trident gained a default-on `neural` feature — build weight, not the fix | ½–1 (spent ~1½) | Full re-vendor → dev + release build the workspace, tests green, zero warnings outside `.vendor/`; release-installed warrior end to end on trident-built TASM: `run` 12 cycles → `prove` STARK 33 ms (padded height 256, 710 KB) → `verify` PASS. Triton side of the differential harness unblocked |
| S2 contract — **DONE** (trident#37, 2026-09-10) | `reference/warrior-api.md`: what a warrior may link (`compile_to_bundle`, `build_tir`/`build_tir_project`, runtime types+traits, `TerrainConfig`, hashing, proof sizing), the feature contract (`default-features = false`) and the stability promise (TIR is an in-process Rust contract, not a wire format; `ProgramBundle` JSON is the only crossing form and its round trip is lossy in a documented way). `tests/warrior_api.rs` makes it executable in both feature configurations. The contract functions already existed — they were undocumented, not missing. **`neural::Target` deliberately deferred to S3**: a trait with one implementation is what quality.md pass 10 forbids; it earns its place when vocab.rs (449) + grammar (721) + stack_verifier (1170) actually move | 1 (spent ~½) | Writing the test found a real hole: `Diagnostic`, the error type every documented entry point returns, was unreachable as `trident::Diagnostic` (privately imported in lib.rs) — now re-exported. 1039 tests green, zero warnings both ways |
| S3 the move | triton lowering + linker, Triton cost + stack_verifier, TASM neural target, baselines, os/neptune, tests → trisha; `trisha build`/`cost`/`bench`; `trident build --target triton` delegates | 2–3 | trisha `cargo test`; every `benches/references` program: `trisha run` == `joy run`; hand-baseline ratios unchanged; neural verified-wins column unchanged |
| S4 core cleanup | typecheck/lsp target-generic; CLAUDE.md pipeline line + Key Modules corrected; roadmap; CHANGELOG; trident-lang 0.3.0, trisha 0.2.0 | 1 | zero warnings, `trident audit`, docs sweep |

**Total 5–7 sessions.** Kelvin: restructuring, no language change — stack
temperature unchanged; trident-lang 0.3.0 because the CLI loses `compile`
and `--target triton` now needs trisha installed.

## Verification (per repo rules)

Every step: `cargo check` zero warnings · `cargo test` · `trident bench`
(through trisha after S3) no regressions · `trident audit` · differential
triton×nox on `benches/references` (unblocked by S1) · fresh
`cargo install trident-lang cyber-joy` in /tmp. Branches `feat/*`, PRs,
no direct master commits. Two agents never share a directory: trident
side by `syntax/ ast/+typecheck/ ir/ cost/+verify/ cli/ package/ lsp/
neural/ docs/`, trisha side its own repo.

## Open for the owner

- `silicon/` in trident (proposed) vs straight into `nox/` now.
- `neural` as a default-on feature (proposed) vs unconditional.

## Settled

- Warriors own the last mile; one binary per machine. (owner, 2026-09-09)
- Neptune + Triton go to trisha. (owner, 2026-09-09)
- Emitters out of the crate into one folder, no 28 warriors. (owner, 2026-09-09)
- Shared infrastructure stays in the core; criterion: produces the core
  representation or consumed by every warrior. MIR import is core.
  (owner, 2026-09-10)
- TIR is the core's IR and the warrior contract; only `lower/triton.rs`
  and the linker move. (owner, 2026-09-10)
- neural harness (model, TIR graph, training, inference) is core; the
  target-specific verifier, vocabulary and cost move behind
  `neural::Target`. (owner, 2026-09-10)
