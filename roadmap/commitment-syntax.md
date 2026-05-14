---
status: draft
author: mastercyb
area: cryptography
planned: 64K
---

# Commitment Schemes as Language Primitives

## Motivation

In existing cryptographic libraries, commitment schemes are function calls. `commit(value)` calls a hash function, returns a digest, and the programmer manages the binding between commitments and values manually. The language knows nothing about commitments — they are bytes, or hashes, or opaque objects. The compiler cannot optimize across commitment boundaries because it does not know what they mean.

When commitments are language primitives — known to the compiler at the semantic level — the compiler can optimize across them. Multiple `commit` calls over related data can be batched into a single sponge absorption. The compiler can prove that a revealed value matches its commitment without separate verification calls. The developer writes high-level intent; the compiler generates optimal hash circuitry.

## Design

### The Primitive Operations

```trident
// Commit: produce a binding commitment to a value
let c: Commitment = commit(value);

// Reveal: open a commitment to its value with a proof
let (v, proof): (Field, OpeningProof) = reveal(c);

// Verify: check that an opening is valid
assert!(verify(c, v, proof));
```

These are language keywords, not function calls. The compiler recognizes them and generates TASM that uses Triton VM's native Tip5 hash instruction for `commit`, with optimal sponge scheduling.

### Batch Optimization

When the compiler sees multiple `commit` calls over related data, it merges them into a single Tip5 sponge absorption:

```trident
// Source:
let c1 = commit(v1);
let c2 = commit(v2);
let c3 = commit(v3);

// Compiler emits:
let (c1, c2, c3) = commit_batch(v1, v2, v3);
// Single sponge absorption: one Hash table row sequence instead of three
```

The `commit_batch` instruction absorbs all values in a single sponge pass, then squeezes out multiple commitments. This is semantically equivalent to three separate commits (with appropriate domain separation) but uses ~3× fewer Hash table rows.

The compiler performs this batch automatically — the developer writes three separate `commit` calls and the compiler detects the pattern and emits the batched form. No manual refactoring required.

### Compiler-Managed Binding Table

The compiler maintains a binding table: a mapping from commitment values to the values they commit to. Within the scope where both `c = commit(v)` and the value `v` are in scope, the compiler knows that `verify(c, v, ...)` is trivially true — no STARK constraint needed, no runtime verification call. The binding is statically known.

```trident
let c = commit(v);
// Later in the same scope:
assert!(verify(c, v, some_proof));  // compiler eliminates this — binding is known
```

This static binding elimination can remove entire chains of commit/verify pairs that exist only for protocol completeness but are computationally free when the values are known at compile time.

### Sponge Scheduling Across Function Boundaries

The compiler traces commitment patterns across function calls. If a function produces commitments that are passed to another function that verifies them, the compiler can schedule the sponge absorptions and squeezes optimally across the entire call chain — not just within each function.

```trident
fn producer() -> (Commitment, Commitment) {
    (commit(secret_1), commit(secret_2))
}

fn consumer(c1: Commitment, c2: Commitment, v1: Field, v2: Field) -> bool {
    verify(c1, v1, open_1) && verify(c2, v2, open_2)
}

// Compiler: producer's two commits and consumer's two verifies
// form a single sponge session — scheduled as one sequence
```

### Domain Separation

Commitment semantics include domain separation: `commit(v)` in one context produces a different commitment than `commit(v)` in another, to prevent cross-protocol attacks. The compiler handles domain separation automatically based on the syntactic context of each `commit` call — no manual domain tag management.

## Key Tradeoffs

**Batch semantics**: Batched commitments are not the same as individual commitments — the binding between values and commitments changes when they are absorbed together. The compiler must generate appropriate domain separation for batched commits to ensure that `commit_batch(v1, v2)` does not produce the same `c1` as `commit(v1)`. The verifier must know which form was used to verify correctly.

**Interaction with ZK types**: When `commit` is called on a `Private<Field>` value inside a `zk fn`, the resulting `Commitment` is `Public<Commitment>` — the commitment is safe to reveal because the commitment scheme is hiding. The type system must model this: `commit: Private<Field> -> Public<Commitment>`.

**Optimality of batch scheduling**: The compiler's automatic batch detection is a heuristic. It batches commits that appear sequentially in the same block. Commits that appear in different branches, or separated by complex control flow, may not be batched. For maximum optimization, the developer can use explicit `commit_batch` calls.

**Proof size for openings**: Opening proofs (`OpeningProof` in the `reveal` result) are field elements generated by the STARK. Their size depends on the commitment scheme parameters. For Tip5-based commitments, openings are small. The developer should be aware that accumulating many commitments and opening them all adds up in proof size.

## Implementation Sketch

Commitment primitives are lowered in the TIR-to-TASM phase:

```rust
// tir/commitment.rs
fn lower_commit(value: TirExpr, tir: &mut TirBuilder) -> TirVar {
    let hash_input = tir.prepare_hash_input(value);
    let commitment = tir.emit(TasmOp::Tip5(hash_input));
    tir.record_binding(commitment, value);  // static binding table
    commitment
}

fn try_batch_commits(block: &TirBlock, tir: &mut TirBuilder) {
    let commits: Vec<_> = block.sequential_commits().collect();
    if commits.len() >= 2 {
        let values: Vec<_> = commits.iter().map(|c| c.input()).collect();
        let batch_result = tir.emit(TasmOp::Tip5Batch(values));
        // Replace individual commit results with projections from batch
        for (i, commit) in commits.iter().enumerate() {
            tir.replace(commit.output(), batch_result.field(i));
        }
    }
}

fn lower_verify(commitment: TirVar, value: TirVar, proof: TirVar, tir: &mut TirBuilder) {
    // Check static binding table first:
    if tir.binding_table().knows(commitment, value) {
        // Statically known — no STARK constraint needed
        return;
    }
    // Otherwise: emit hash recomputation and comparison constraint
    let recomputed = lower_commit(tir.expr(value), tir);
    tir.emit_constraint(TirExpr::Eq(recomputed, commitment));
}
```
