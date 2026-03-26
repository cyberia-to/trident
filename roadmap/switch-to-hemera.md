# Switch to Hemera

## Context

Trident currently uses two hash functions:
- **BLAKE3** (external crate) — only for Poseidon2 round constant generation
- **Custom Poseidon2** (two implementations) — content addressing, program digests

Hemera (`cyber-hemera` on crates.io, `~/git/hemera`) is the canonical hash
primitive of the cyber cyberstate. It is a Poseidon2 sponge over Goldilocks
with self-bootstrapped constants derived from the seed "cyber". Output is
always 64 bytes (8 Goldilocks elements).

## Why

- One hash function for the entire ecosystem, not two custom Poseidon2 impls
- Hemera is battle-tested with pinned test vectors and zero dependencies
- Self-bootstrapped constants (from "cyber" seed) vs BLAKE3-derived constants
- 64-byte output (256-bit collision resistance) vs current 32-byte output
- Tree hashing, XOF, keyed hashing, key derivation — all built in
- Aligns trident with the cyberstate's cryptographic identity

## Breaking Change

ContentHash changes from 32 bytes to 64 bytes. All existing hashes
(codebase store, lockfiles, program digests) become invalid. This is a
clean break — all content must be rehashed.

## Inventory: What Changes

### Layer 1: Remove BLAKE3 dependency

| File | What | Action |
|------|------|--------|
| `Cargo.toml` | `blake3 = "1"` | Remove, add `cyber-hemera = "0.2"` |

### Layer 2: Remove custom Poseidon2 implementations

| File | LOC | What | Action |
|------|-----|------|--------|
| `src/package/poseidon2.rs` | ~340 | Standalone Goldilocks Poseidon2 | Delete entirely |
| `src/field/poseidon2.rs` | ~295 | Generic Poseidon2 over PrimeField | Delete entirely |

These are replaced by `cyber_hemera::hash()` and `cyber_hemera::Hasher`.

### Layer 3: Update ContentHash (32 → 64 bytes)

| File | What | Change |
|------|------|--------|
| `src/package/hash/mod.rs` | `ContentHash([u8; 32])` | Change to `ContentHash([u8; 64])` |
| `src/package/hash/mod.rs:118` | `hash_file_content()` | Replace `poseidon2::hash_bytes()` with `hemera::hash()` |
| `src/package/hash/normalize.rs:180,193,204` | `hash_bytes()` calls | Replace with `hemera::hash().as_bytes()` |
| `src/package/hash/mod.rs` | `hash_version: 1` | Bump to `hash_version: 2` |
| `src/package/hash/mod.rs` | hex display (64 chars) | Update to 128 chars |

### Layer 4: Update all hash call sites

| File | Line | What | Change |
|------|------|------|--------|
| `src/deploy/mod.rs` | 87 | `poseidon2::hash_bytes(tasm)` | `hemera::hash(tasm)` |
| `src/deploy/tests.rs` | 115 | determinism test | Update expected values |
| `src/cli/deploy.rs` | 150 | dry-run hash | `hemera::hash()` |
| `src/cli/package.rs` | 92 | dry-run hash | `hemera::hash()` |
| `src/cli/hash.rs` | entire | `trident hash` command | Use hemera, update output format (128 hex chars) |
| `src/package/manifest/resolve.rs` | 217 | source hash | `hemera::hash()` |
| `src/package/manifest/mod.rs` | comment | mentions "BLAKE3" | Fix comment |

### Layer 5: Update field layer

| File | What | Change |
|------|------|--------|
| `src/field/poseidon2.rs` | Generic Poseidon2 | Delete — hemera is the hash |
| `src/field/mod.rs` | `pub mod poseidon2` | Remove module declaration |
| `src/field/proof.rs` | If references poseidon2 | Update imports |

### Layer 6: Update re-exports

| File | What | Change |
|------|------|--------|
| `src/lib.rs:30` | `pub use package::hash` | Keep |
| `src/lib.rs:32` | `pub use package::poseidon2` | Remove, add `pub use cyber_hemera as hemera` |
| `src/package/mod.rs:5` | `pub mod poseidon2` | Remove |

### Layer 7: Update runtime artifacts

| File | What | Change |
|------|------|--------|
| `src/runtime/artifact.rs` | `source_hash: String` | Now 128 hex chars |
| `src/runtime/artifact.rs` | per-function `hash: String` | Now 128 hex chars |

### Layer 8: Update store (on-disk format)

| File | What | Change |
|------|------|--------|
| `src/package/store/mod.rs` | Def storage paths | 2-char prefix from 128-char hex |
| `src/package/store/mod.rs` | Serialization | Update hash field widths |
| `src/package/manifest/lockfile.rs` | Lockfile format | 128-char hex hashes |

### Layer 9: Update benchmarks and references

| File | What | Change |
|------|------|--------|
| `benches/references/std/crypto/poseidon2.rs` | Benchmark reference | Rewrite using hemera |
| `benches/references/std/crypto/merkle.rs` | Merkle reference | Use hemera tree hashing |
| `benches/references/std/trinity/inference.rs:365,371` | Round constants | Use hemera |

### Layer 10: Update Trident standard library

| File | What | Change |
|------|------|--------|
| `vm/*/hash.tri` or equivalent | Hash builtins | Must emit hemera, not old Poseidon2 |
| `std/crypto/poseidon2.tri` | Stdlib hash | Rename/rewrite as hemera wrapper |
| Cost tables in `src/cost/` | Hash cycle costs | Update to hemera's cycle count |

### Layer 11: Update reference docs

| File | What | Change |
|------|------|--------|
| `reference/language.md` | Digest type description | Reference hemera |
| `docs/explanation/content-addressing.md` | Hash function description | BLAKE3 → hemera |
| `src/README.md` | Package description | BLAKE3 → hemera |
| `CLAUDE.md` | Key modules section | Update poseidon2 references |

### Layer 12: Update trisha (companion repo)

| What | Change |
|------|--------|
| Cargo.toml | Add `cyber-hemera` if needed |
| Any hash verification | Must match hemera output format |
| Program digest checking | 64-byte digests |

## Execution Order

1. **Add hemera dependency, remove blake3** (Cargo.toml)
2. **Create `src/hemera.rs` thin wrapper** — re-export `cyber_hemera` with
   project-local helpers (`content_hash()`, `program_digest()`)
3. **Update ContentHash** — 32 → 64 bytes, bump hash_version
4. **Replace all `poseidon2::hash_bytes()` calls** with hemera
5. **Delete `src/package/poseidon2.rs`**
6. **Delete `src/field/poseidon2.rs`**
7. **Update re-exports** in lib.rs and package/mod.rs
8. **Update CLI** (hash command, deploy, package)
9. **Update store** (on-disk paths, serialization)
10. **Update runtime artifacts** (ProgramBundle hash fields)
11. **Update benchmarks and references**
12. **Update stdlib and cost tables**
13. **Update docs** (reference, explanation, README)
14. **Rebuild trisha** and verify compatibility
15. **Run full test suite**: `cargo check`, `cargo test`, `trident bench`

## Verification

After complete switch:
- `cargo check` — zero warnings
- `cargo test` — all tests pass (with updated expected values)
- `grep -r blake3 src/` — zero hits
- `grep -r 'package::poseidon2' src/` — zero hits
- `grep -r 'field::poseidon2' src/` — zero hits
- `trident hash` — outputs 128 hex chars
- `trident bench` — no regressions
- Content store rehashed under new scheme

## Risk

- **Trisha compatibility**: trisha verifies program digests. Both repos must
  switch atomically or use a version flag.
- **Existing lockfiles**: All lockfiles become invalid. Users must `trident lock --force`.
- **Test vectors**: Every hash-dependent test needs new expected values.
- **Poseidon2 in TIR/cost model**: The compiler emits Poseidon2 instructions
  for on-chain hashing. These target the VM's native hash (Tip5 on Triton),
  NOT the content-addressing hash. Hemera replaces only the off-chain
  content addressing, not the VM hash instruction. Verify this distinction
  is preserved.

## Estimate

~2 sessions (12 pomodoros). Most work is mechanical replacement.
The tricky parts: ContentHash width change ripple, store migration,
trisha coordination.
