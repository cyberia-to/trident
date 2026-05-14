# Switch to Hemera

**Related:** [[cyber-stack-adoption]], [[commitment-syntax]], [[merkle-iterators]]

## Context

Trident currently uses two hash functions:
- **BLAKE3** (external crate) — only for Poseidon2 round constant generation
- **Custom Poseidon2** (two implementations) — content addressing, program digests

Hemera (`cyber-hemera` on crates.io, `/Users/master/cyberia-to/hemera/`) is
the canonical hash primitive of the cyber stack. Parameters are fixed:
Poseidon2 sponge over Goldilocks (p = 2^64 - 2^32 + 1), d=7, t=16, Rf=8,
Rp=16. Output: 4 field elements × 8 bytes = 32 bytes (displayed as 64 hex
chars). Constants self-bootstrapped from the seed "cyber". These parameters
are permanent — hemera provides no algorithm agility. All content hashed
under hemera is a permanent commitment.

The `hash` jet in nox (pattern 15) uses hemera. ContentHash in trident
stays `[u8; 32]` — the byte width does not change, because hemera also
outputs 32 bytes. The breaking change is the hash algorithm: all existing
BLAKE3/Poseidon2 content hashes become invalid and must be recomputed.
`hash_version` bumps from 1 → 2 to signal the algorithm change.

## Why

- One hash function for the entire ecosystem, not two custom Poseidon2 impls
- Hemera is battle-tested with pinned test vectors and zero dependencies
- Self-bootstrapped constants (from "cyber" seed) vs BLAKE3-derived constants
- 32-byte output (4 × u64 Goldilocks elements, 256-bit collision resistance) — same size as current
- Tree hashing, XOF, keyed hashing, key derivation — all built in
- Aligns trident with the cyberstate's cryptographic identity
- Permanent commitment: no algorithm agility, no migration path after adoption

## Breaking Change

ContentHash byte width stays 32 bytes — hemera also outputs 32 bytes (4
Goldilocks elements). The breaking change is the hash algorithm: the
BLAKE3-derived round constants and custom Poseidon2 parameters are replaced
by hemera's "cyber"-seeded constants. All existing hashes (codebase store,
lockfiles, program digests) are invalidated and must be recomputed.
`hash_version` bumps 1 → 2 to distinguish old from new hashes. This is a
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

### Layer 3: Update ContentHash (algorithm change, byte width unchanged)

| File | What | Change |
|------|------|--------|
| `src/package/hash/mod.rs` | `ContentHash([u8; 32])` | Stays `[u8; 32]` — hemera outputs 32 bytes |
| `src/package/hash/mod.rs:118` | `hash_file_content()` | Replace `poseidon2::hash_bytes()` with `hemera::hash()` |
| `src/package/hash/normalize.rs:180,193,204` | `hash_bytes()` calls | Replace with `hemera::hash().as_bytes()` |
| `src/package/hash/mod.rs` | `hash_version: 1` | Bump to `hash_version: 2` (algorithm change signal) |
| `src/package/hash/mod.rs` | hex display (64 chars) | Stays 64 chars (32 bytes × 2 hex digits) |

### Layer 4: Update all hash call sites

| File | Line | What | Change |
|------|------|------|--------|
| `src/deploy/mod.rs` | 87 | `poseidon2::hash_bytes(tasm)` | `hemera::hash(tasm)` |
| `src/deploy/tests.rs` | 115 | determinism test | Update expected values |
| `src/cli/deploy.rs` | 150 | dry-run hash | `hemera::hash()` |
| `src/cli/package.rs` | 92 | dry-run hash | `hemera::hash()` |
| `src/cli/hash.rs` | entire | `trident hash` command | Use hemera, output stays 64 hex chars (32-byte digest) |
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
| `src/runtime/artifact.rs` | `source_hash: String` | Stays 64 hex chars (algorithm change only) |
| `src/runtime/artifact.rs` | per-function `hash: String` | Stays 64 hex chars (algorithm change only) |

### Layer 8: Update store (on-disk format)

| File | What | Change |
|------|------|--------|
| `src/package/store/mod.rs` | Def storage paths | 2-char prefix from 64-char hex (format unchanged) |
| `src/package/store/mod.rs` | Serialization | Update hash_version; all stored content must be rehashed |
| `src/package/manifest/lockfile.rs` | Lockfile format | All hashes invalidated; users must `trident lock --force` |

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

## Vision

After the [[hemera]] switch, the [[cybergraph]]'s content addressing and [[zheng]]'s internal hashing use the same primitive. A particle and the hash commitment inside its proof are computed by the same function. This creates an unexpected synergy: Merkle proofs over content-addressed data — using [[hemera]] — can be verified inside [[zheng]] proofs, which use [[hemera]] internally, at zero marginal cost. The graph and the proof system share a hash function and a security assumption.

The self-bootstrapped constants (from the "cyber" seed) matter more than they appear to. Every content address in the [[cybergraph]], every program digest in [[Atlas]], every Brakedown commitment in every [[zheng]] proof: all are rooted in the same cryptographic identity. One hash function, one trust root, one security assumption for the entire stack. The current split — BLAKE3 for round constant generation, custom Poseidon2 for content addressing — means two trust roots and two audit surfaces. After the switch, there is one.

The permanent commitment aspect is a feature of the [[cybergraph]] design: no migration path, no algorithm agility. Every particle ever created will be addressed by hemera permanently. The breaking change is clean and one-time. Everything after it compounds indefinitely on a single foundation.

## Stack Integration

The [[nox]] `hash` jet (Layer 3, pattern 15) accelerates [[hemera]] calls. After the switch, every `hash()` call in Trident code invokes [[hemera]] via this jet, at ~200 [[nox]] steps instead of the software fallback. The [[zheng]] Brakedown commitment's internal Merkle tree is built with [[hemera]] — the same crate, the same constants, the same output format as the content-addressed particle CIDs in the [[cybergraph]]. [[bbg]] state transitions that read or write particles use [[hemera]] addresses. The entire stack becomes one-crate-deep in hashing: `cyber-hemera`, and nothing else.

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
- `trident hash` — outputs 64 hex chars (32-byte hemera digest)
- `trident bench` — no regressions
- Content store rehashed under new scheme

## Risk

- **Trisha compatibility**: trisha verifies program digests. Both repos must
  switch atomically or use a version flag.
- **Existing lockfiles**: All lockfiles become invalid. Users must `trident lock --force`.
- **Test vectors**: Every hash-dependent test needs new expected values.
- **VM hash instruction vs content addressing**: The compiler emits hash
  instructions for on-chain hashing. For the Triton VM (trisha target),
  the on-chain hash is Tip5 — unchanged. For the nox target, the `hash`
  jet (pattern 15) uses hemera. Hemera replaces only the off-chain
  content addressing in trident-core; trisha's on-chain Tip5 is unaffected.
  Verify this distinction is preserved.

## Estimate

~2 sessions (12 pomodoros). Most work is mechanical replacement.
The tricky parts: ContentHash width change ripple, store migration,
trisha coordination.
