# Changelog

Kelvin versioning: versions count down toward 0K (frozen forever).
Lower is colder. Colder is more stable.

## Unreleased

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
