# Changelog

Kelvin versioning: versions count down toward 0K (frozen forever).
Lower is colder. Colder is more stable.

## Unreleased

- **BREAKING: Triton/Neptune-only logic moved out of trident, wholesale
  (S3 of `.claude/plans/warrior-owns-lowering.md`).** trident stops at
  TIR; trisha owns the last mile. Moved to trisha (as its own
  `trisha-rs` modules, ~9k LOC + the moved test suites):
  - `ir/tir/lower/*` (`StackLowering`, `TritonLowering`, `create_stack_lowering`)
    and `ir/tir/linker.rs` — instruction selection and TASM linking.
  - The whole AET-table cost subsystem (`cost::{analyzer, model, scorer,
    stack_verifier, report, json, visit}`) — a `CostModel` trait with
    exactly one implementation (Triton), the same shape `StackLowering`
    was before it moved. `cost::nox` (reductions) is trident's only cost
    model now.
  - The neural optimizer (`src/neural/`, all of it — vocabulary, GNN
    encoder/decoder, GFlowNet training, beam search) — its vocabulary
    *is* TASM tokens and its reward runs through the moved
    `stack_verifier`; there is no other target that trains today. The
    `neural` feature flag is gone with it — trident no longer depends on
    burn/wgpu/rayon/rkyv at all.
  - `cli::{bench, train, trisha}` (the bench harness, training CLI, and
    trisha-subprocess helpers they shared).
  - `baselines/triton/*.tasm` (10.7k lines) and the ~110 TASM-asserting
    tests (`api/tests/{compile,features,neptune,prove}.rs`,
    `tests/audit_stdlib.rs`) — they test the moved lowering, so they
    test it from trisha now.
  - `trident audit`'s execution-correctness mode (classic/hand/neural
    vs baselines, no-args) — redundant with `trisha bench`; `trident
    audit <file>` (symbolic/formal verification) is unchanged.
  - `trident doc` and `--annotate` (`api/doc.rs`, `cli/doc.rs`) —
    cost-annotated documentation built entirely on the moved analyzer;
    removed rather than kept dishonest for nox (its `--target nox`
    default silently ran the Triton cost model, since it was the only
    one registered).
  - LSP inlay hints and hover cost annotations (`lsp/hints.rs`,
    `format_cost_inline`, the `**Cost:**` line in hover) — same reason.
  Deleted as dead code surfaced along the way: `ir/tir/encode.rs` (416
  LOC, zero callers), `package/cache.rs` (446 LOC, zero callers,
  TASM-typed), `PreparedProject::{program_module, last_file}`.
  **Fixed in the same pass**, found while tracing the last real callers
  of the moved cost types: `deploy::generate_artifact` took the
  Triton-only `ProgramCost` for a job (embedding cost numbers in a
  package manifest) that only needed the already-generic `BundleCost` —
  it does now, and packaging is target-agnostic again. `trident build
  --target triton` (and `--target` for any other stack engine) now
  delegates to the installed warrior, the same way `run`/`prove`/
  `verify` already did — the honest error trident gives when no warrior
  is installed points at a real, working delegation path, not an
  aspirational one.
  Verified: `trisha build` on real programs is byte-identical to what
  `trident build --target triton` used to emit, then to what it now
  gets via delegation; `trisha run/prove/verify` round-trip for real
  (`triple(5)+1` → 16, proof in 33 ms, PASS); `trisha build --costs`
  prints a real per-function AET-table report. 654 trident tests green
  (was 955 — the delta is what moved to trisha, not lost coverage), 295
  trisha tests green (179 of them moved from trident), zero warnings in
  both repos.

- **BREAKING (library): the default terrain is nox.** The CLI has
  defaulted to nox since 0.2.0, but `CompileOptions::default()` and
  `::for_profile()` still handed back Triton — so `trident::compile()`,
  the crate's front door, emitted TASM while `trident build` emitted a
  nox formula. The library now follows the CLI: default = nox, the
  terrain the stack runs and proves on (joy + zheng). A caller who wants
  a stack engine names it (`options.target_config =
  TerrainConfig::triton()`), which is also what the ~110 TASM-asserting
  tests now do — they say which engine they test instead of leaning on a
  default, so they keep saying it when the Triton lowering moves to
  trisha. The same fix landed in trisha, where it was a real bug: the
  Triton warrior inherited whatever the compiler defaulted to instead of
  naming its own terrain.
- **`neural` is a default-on feature.** burn (and the cubecl/wgpu graph
  under it) is the heaviest thing trident links. The compiler binary is
  unchanged; `--no-default-features` gives a light library build for
  warriors that embed trident and never train — trisha takes that path,
  which cuts its dependency graph to a fraction. Gated: the `neural`
  module, `trident bench`, `trident train`, `trident build --neural/--train`,
  the bench harness in `cli/trisha.rs`, and the `end_to_end` bench target
  (`required-features`). Also fixes a pre-existing break on master:
  `benches/end_to_end.rs` constructed `BeamConfig` without four of its
  fields; now `..Default::default()`, no invented numbers.
- **Superseded by the entry above**: the `neural` feature flag this entry
  introduced is gone entirely now — `src/neural/` itself moved to trisha,
  so there is nothing left in trident-core to gate. `benches/end_to_end.rs`
  (which needed the flag) moved with it.
- **Correction to that commit's message** (51aa904): it claimed dropping
  burn from a warrior's graph removed the class of breakage behind
  trisha#1. It did not. trisha#1's root cause was the vendored
  twenty-first shipping `crate-type = ["cdylib", "rlib"]`, which makes
  rustc see two versions of serde and rand; with that patched, trisha
  builds fine with burn back in the graph. The `neural` feature stands on
  build weight alone.
- **BREAKING (CLI): `trident compile` moved to a separate `silicon`
  binary.** S0 of `.claude/plans/warrior-owns-lowering.md` — the 25
  native-silicon emitters (`src/compile/`, 14.4k LOC) are a nox-only
  leaf with zero dependency on trident's compiler internals; they now
  live in an in-repo, unpublished workspace member `silicon/`
  (`cargo install --path silicon`, binary `silicon`). `trident mir`
  stays in the core (it produces a nox formula, trident's own
  representation, not a translation out to a machine) and moved to
  `src/import/` alongside the merged-in `mir2nox`. Dead code removed:
  `src/ir/kir` and `src/ir/lir` (never implemented — `create_kernel_lowering`
  returned `None`, `tir_to_lir` was `todo!()`), `src/gpu` (325 LOC, zero
  callers), and the orphaned `src/import/{mir,structurize,types}.rs`
  (depended on `mir-format`, a `publish = false` crate that cannot be a
  trident-lang manifest dependency — see `feedback_crates_publish_path_deps`
  — and was never wired into the module tree). Unused dependencies
  dropped: `bytemuck`, `petgraph`, `statrs`, `pollster`; `blake3` moved
  to `[dev-dependencies]` (benches-only). No change to `.tri` → nox/TASM
  compilation, no proof-size or reduction-count change.
- Found, not fixed here (pre-existing on master, reproduced before this
  change): `trident compile` / `silicon` hangs on every invocation —
  `Order::<65536>::new()` in the CLI's formula parser never returns in a
  debug build. Filed as trident#32.
- proofs are constant-size (zheng 0.3.1 via joy 0.3.0): one universal
  step CCS, ≤ 2 accumulator groups for any program, every wire byte
  verifier-read — hello 1.3 KB, two secrets 1.4 KB, one hash 2.4 KB,
  depth-32 Merkle path 2.6 KB (was 2.67 MB). `cargo install cyber-joy`.
- `Digest` limbs index on nox: `d[k]` for k < 4 lowers to the hash
  pair's axes (reference/language.md already said `Digest` is
  `[Field; D]`; the typechecker rejected it). Enables Merkle paths
  chained through digests — depth 32 costs 1,906 reductions statically
  (825 in the 33 hash patterns). Stack targets still reject digest
  indexing (no limb store yet).

## 0.2.0 / 500K — Cast (2026-09-08)

The soft3 release. trident compiles to **nox** by default and the whole
soft3 stack stands behind it — strata algebra and hemera hashes inside
the compiler, nox execution, zheng proofs, bbg state — through a new
warrior, **joy**. `cargo install trident-lang cyber-joy` gives
build → run → prove → verify with a zheng proof, out of the box:

```
$ trident build hello.tri                  # hello.nox
$ trident prove hello.tri --secret 7,13    # Proved in 14 ms: 17 reductions, 20 KB
$ trident verify hello.zheng               # Verification: PASS (zheng proof)
```

Kelvin: 512K → 500K. Still hot, but the change is fundamental — the
language now stands on its own stack (nox by default, a real prover,
one algebra, one hash) instead of borrowing Triton's. The cyber stack
and Noun layers cleared most of their 256K checklists (see
`reference/roadmap.md`); the next big drop comes when a whole tier
clears.

### nox is the default target
- every command's `--target` defaults to `nox`; `--target triton` keeps
  TASM and the trisha warrior. `TerrainConfig::nox()` is a built-in
  fallback (like `triton()`), so an installed binary works from any
  directory. `vm/nox/target.toml`: level 4, cost model, tests, joy as
  warrior with `prover = true`.

### the full language surface lowers to nox (M1)
- statements lower to composable subject transformers; mutable
  assignment is a subject edit (fixes lost mutations inside `if`/loop
  arms); bounded `for` unrolls static bounds and guards dynamic ends;
  function calls inline (recursion rejected, node budget); tuples,
  structs and fixed arrays are cons-trees; `hash`, `assert*`, field
  builtins lower to patterns. `divine()` emitted a malformed call —
  fixed. every mapping is verified by reducing on the real nox VM.
- struct-field and array-element assignment now reach the front end on
  **both** targets. the Triton path previously compiled `a[i] = v` to
  a silent `swap 0; pop 1` no-op; it now emits a real store or an
  honest error (multi-word writes, runtime indices).
- `os.state.read(key)` lowers to the look pattern (BBG dimension 0);
  `ProgramBundle.reads_state` is additive and JSON-backward-compatible.
- unsupported on nox is an explicit compile error, never wrong code:
  sponge/Merkle/RAM/IO builtins, xfield ops, dynamic array index,
  `return` inside loops.

### honest cost model (M2)
- `trident build --costs` bills reductions from the emitted noun;
  branch-dependent cost is a `min..=max` range, exact only for
  straight-line code; `trident bench` grows a `Nox(r)` column (2 of 42
  baselines are inside the nox surface — the rest are `-`). the static
  bill equals the runtime bill on straight-line programs.

### joy — the cyber warrior (M3, M4)
- `joy run` / `joy prove` / `joy verify` on nox: zheng proofs
  (SuperSpartan + Brakedown + HyperNova folding), verified without
  re-execution; hash blocks and BBG `look` reads prove; artifacts are
  self-contained binary `.zheng` files (postcard) with the prover's
  folded witness never on the wire. `trident run/prove/verify` delegate
  to it. Measured (release): add proves in 4 ms / 5.4 KB, two `divine()`
  secrets in 14 ms / 20 KB, a hash in 70 ms / 52 KB — ~1.7 KiB per
  accumulator group, one group per CCS structure (zheng#8 tracks
  collapsing groups toward the 2–5 KB program-level target).
  github.com/cyberia-to/joy

### differential harness
- `tests/differential.rs`: the stdlib modules inside the nox surface
  (fibonacci, poseidon) execute on nox and must equal independent Rust
  ground truth; a census pin fails whenever the surface grows without
  coverage. Triton×nox agreement pends trisha's build repair (trisha#1).

### fixed in the stack, surfaced by this release
- zheng: four silent soundness holes — `pattern_quote` constrained a
  stale register, the relaxed-fold loophole, the whole arithmetic family
  on a stale register map (mul/eq/branch violations were silently
  accepted), relaxed-proof pairing at m>1 — and the axis / hash-rate /
  look bindings that were the release blocker. zheng 0.2.0.
- nox: call rows carried the arena Order of the check result instead of
  its value, so every `divine()` proof failed verification; the branch
  selector r10 was clobbered on success; the arena-forking parallel
  executor hijacked std builds. cyber-nox 0.2.0. Filed: arena hang at
  N ≥ 8192 (nox#2), spec/code budget drift (nox#1).
- bbg: cli broken on master; QueryProof/Commitment/Opening serde. bbg
  0.2.0, cyber-lens 0.1.3. lens: porphyry/genies drift filed (lens#2).

### removed
- the Rs → Trident MIR importer (`src/import/`, `mir-format`) leaves the
  published crate: `mir-format` is `publish = false` by design and
  `cargo publish` rejects any reference to an unpublished dependency
  (path, git and optional forms alike). it returns as a companion crate.
- `trident::poseidon2` — see BREAKING below.
- `media/` is excluded from the package (a 9 MB gif was 70% of the
  crate); the README image is an absolute URL.

### BREAKING: content hashes are now cyber-hemera

One Poseidon2 on the planet, and it is hemera's. The compiler's in-tree
Poseidon2 (`src/package/poseidon2.rs`, BLAKE3-derived constants,
t=8/RF=8/RP=22) is deleted; every content identifier — `trident deploy`
digests, package hashes, function identity from normalized AST — is now
`cyber-hemera`'s hash. All previously computed content hashes change
value. Representative old → new digests:

| input | old (in-tree Poseidon2) | new (cyber-hemera) |
|---|---|---|
| `""` | `0c3c1eaef3b8f8d5…8de91b59` | `a67a71b221e6bdd6…806d563c` |
| `"trident"` | `0027d32685465f66…26026a86` | `5386a7e49142b577…abc9df71` |
| `"fn main() { pub_write(42) }"` | `5f4ccf1c8d38936c…5864e2dd` | `1c315c3651929e24…cd2013bd` |

The API `trident::poseidon2::hash_bytes` is gone; use
`trident::hash::content_hash_bytes`. The Trident-language Poseidon2
library (`std/crypto/poseidon2.tri`) and its Rust ground-truth fixture
are unchanged — they are a language-level pair, not compiler
infrastructure.

The compiler's Goldilocks arithmetic is now `strata-nebu`'s (bit-exact
migration — field arithmetic values are unchanged; only the
implementation moved). `babybear.rs`/`mersenne31.rs` stay: they are
foreign-target fields (risczero, sp1), not stack algebra.

### Install

```
cargo install trident-lang cyber-joy
```

## 0.1.0 / 512K — Smelt (2026-02-26)

First public release. Hot, experimental, not production ready.

### Compiler

- Full pipeline: source → lexer → parser → AST → typecheck → TIR → optimizer → lowering → TASM
- 912 tests passing, zero warnings
- 54-operation intermediate representation mapping ~1:1 to target instructions
- Static cost analysis: exact proving cost from source, before execution

### CLI

`build`, `check`, `test`, `fmt`, `audit`, `bench`, `lsp`, `package`, `deploy`

### Standard Library

- `std.crypto`: poseidon2, poseidon, sha256, keccak256, ecdsa, secp256k1, ed25519, merkle, bigint, auth
- `std.nn`: tensor, dense, attention, convolution, lookup-table activations
- `std.private`: polynomial ops, NTT for FHE
- `std.quantum`: gate set (H, X, Y, Z, S, T, CNOT, CZ, SWAP)
- `std.compiler`: lexer, parser, typechecker, codegen, optimizer, lowering, pipeline (9,195 lines of self-hosted Trident)

### Neptune Programs

- Coin (TSP-1): fungible token — pay, lock, update, mint, burn
- Card (TSP-2): non-fungible token — royalties, creator immutability
- Lock scripts: generation, symmetric, timelock, multisig
- Type scripts: native currency, custom token conservation
- Programs: transaction validation, recursive verification, proof aggregation

### Tooling

- Language Server Protocol (`trident-lsp`): diagnostics, completions, hover, go-to-definition
- Editor support: Zed extension, Helix config
- Formal verification: `trident audit` with `#[requires]`/`#[ensures]` contracts
- Neural optimizer: 13M-parameter GNN+Transformer learning to emit optimized TASM
- Benchmark scoreboard: `trident bench` comparing compiler vs hand-written vs neural output

### Install

```
cargo install trident-lang
```

### License

Cyber License: Don't trust. Don't fear. Don't beg.
