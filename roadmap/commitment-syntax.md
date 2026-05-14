---
status: draft
author: mastercyb
area: cryptography
planned: 64K
---

# Commitment Schemes as Language Primitives

**Related:** [[private-public-types]] · [[merkle-iterators]] · [[cybergraph]] · [[hemera]] · [[nox]] · [[soft3]] · [[bbg]]

## Vision

In the cyber ecosystem, every commitment is a [[hemera]]-addressed particle in the [[cybergraph]]. `commit(v)` = `sponge_absorb(v)` → `sponge_squeeze()` → hemera CID. The commitment particle exists in the graph as a first-class node. Later, `reveal(c)` submits the opening proof as a cyberlink from the commitment particle to the opened value. The verifier calls [[soft3]]'s `verify(commitment_cid, proof)` to check it, reading the cyberlink and validating the [[zheng]] proof inline.

Commitment schemes become a first-class interaction pattern in the knowledge graph, not a one-off cryptographic construction. Every protocol that commits-then-reveals leaves a permanent, proven trail in the [[cybergraph]]: commit particle → reveal cyberlink → opened value particle. The graph accumulates cryptographic history by construction.

`commit_batch` optimizes multiple commits into one [[hemera]] sponge invocation — one [[nox]] jet call, reducing both the nox trace length and the focus cost charged by [[bbg]]. Batch commits correspond to fewer cyberlinks in the [[cybergraph]] (one sponge session, one proof segment) while preserving the same semantic guarantees. The [[bbg]] network prices computation by focus (τ); batch commitment is how developers minimize the cost of privacy-preserving protocols.

## Current Status

Language.md §10 already defines `seal` (committed secret) and `reveal` (public output) as first-class events in the language. Language.md §14 already defines `sponge_init`, `sponge_absorb`, `sponge_squeeze` as Tier 2 builtins. This proposal is about building a higher-level `commit`/`verify` primitive layer on top of these existing mechanisms, with compiler-managed binding tracking and batch optimization.

## Motivation

In existing cryptographic libraries, commitment schemes are function calls. `commit(value)` calls a hash function, returns a digest, and the programmer manages the binding between commitments and values manually. The language knows nothing about commitments — they are bytes, or hashes, or opaque objects. The compiler cannot optimize across commitment boundaries because it does not know what they mean.

When commitments are language primitives — known to the compiler at the semantic level — the compiler can optimize across them. Multiple `commit` calls over related data can be batched into a single hemera sponge absorption (using the existing `sponge_absorb`/`sponge_squeeze` builtins). The compiler can prove that a revealed value matches its commitment without separate verification calls. The developer writes high-level intent; the compiler generates optimal hash circuitry.

## Design

### The Primitive Operations

```trident
// Commit: produce a binding commitment to a value
// Desugars to: sponge_init(); sponge_absorb(value); sponge_squeeze()
let c: Commitment = commit(value);

// Reveal: open a commitment to its value with a proof
// Desugars to a reveal event (language.md §10) that writes the value to public output
let (v, proof): (Field, OpeningProof) = reveal(c);

// Verify: check that an opening is valid
assert!(verify(c, v, proof));
```

These are language keywords, not function calls. The compiler recognizes them and generates nox patterns using the hemera sponge — `sponge_absorb` and `sponge_squeeze` builtins (language.md §14) — for `commit`, with optimal sponge scheduling. hemera (Poseidon2) is the hash function underlying all sponge operations; it replaced Tip5.

The `seal` event (language.md §10) is the primitive that `commit` desugars to at the event level: it hashes fields via the sponge and writes only the commitment digest to public output.

### Batch Optimization

When the compiler sees multiple `commit` calls over related data, it merges them into a single hemera sponge absorption sequence:

```trident
// Source:
let c1 = commit(v1);
let c2 = commit(v2);
let c3 = commit(v3);

// Compiler emits:
let (c1, c2, c3) = commit_batch(v1, v2, v3);
// Single sponge absorption: sponge_init; sponge_absorb(v1, v2, v3...); sponge_squeeze ×3
```

The `commit_batch` form absorbs all values in a single `sponge_absorb` / `sponge_squeeze` sequence (language.md §14). This is semantically equivalent to three separate commits (with appropriate domain separation) but uses ~3× fewer nox reduction steps on the sponge path.

The compiler performs this batch automatically — the developer writes three separate `commit` calls and the compiler detects the pattern and emits the batched form. No manual refactoring required.

### Compiler-Managed Binding Table

The compiler maintains a binding table: a mapping from commitment values to the values they commit to. Within the scope where both `c = commit(v)` and the value `v` are in scope, the compiler knows that `verify(c, v, ...)` is trivially true — no nox constraint needed, no runtime verification call. The binding is statically known.

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

**Interaction with ZK types**: When `commit` is called on a `Private<Field>` value inside a `zk fn` (see [[private-public-types]]), the resulting `Commitment` is `Public<Commitment>` — the commitment is safe to reveal because the hemera sponge is hiding. The type system must model this: `commit: Private<Field> -> Public<Commitment>`. This is the typed counterpart to using `seal` (language.md §10) inside a ZK function.

**Optimality of batch scheduling**: The compiler's automatic batch detection is a heuristic. It batches commits that appear sequentially in the same block. Commits that appear in different branches, or separated by complex control flow, may not be batched. For maximum optimization, the developer can use explicit `commit_batch` calls.

**Proof size for openings**: Opening proofs (`OpeningProof` in the `reveal` result) are field elements within the zheng proof (Brakedown PCS). Their size depends on the Brakedown commitment parameters. The developer should be aware that accumulating many commitments and opening them all adds up in proof size.

**Merkle commitments**: For committing to sets of values (rather than individual fields), the hemera-based Merkle tree (see [[merkle-iterators]]) is more efficient. `commit` is for individual values; `merkle_step` (language.md §15) is for authenticated sets.

## Implementation Sketch

Commitment primitives are lowered in the TIR-to-nox phase:

```rust
// tir/commitment.rs
fn lower_commit(value: TirExpr, tir: &mut TirBuilder) -> TirVar {
    let hash_input = tir.prepare_hash_input(value);
    // Uses hemera (Poseidon2) sponge: SpongeInit → SpongeAbsorb → SpongeSqueeze
    let commitment = tir.emit_sponge_commit(hash_input);
    tir.record_binding(commitment, value);  // static binding table
    commitment
}

fn try_batch_commits(block: &TirBlock, tir: &mut TirBuilder) {
    let commits: Vec<_> = block.sequential_commits().collect();
    if commits.len() >= 2 {
        let values: Vec<_> = commits.iter().map(|c| c.input()).collect();
        // Batched: single SpongeInit, multi-chunk SpongeAbsorb, multiple SpongeSqueeze
        let batch_result = tir.emit_sponge_batch_commit(values);
        // Replace individual commit results with projections from batch
        for (i, commit) in commits.iter().enumerate() {
            tir.replace(commit.output(), batch_result.field(i));
        }
    }
}

fn lower_verify(commitment: TirVar, value: TirVar, proof: TirVar, tir: &mut TirBuilder) {
    // Check static binding table first:
    if tir.binding_table().knows(commitment, value) {
        // Statically known — no nox constraint needed
        return;
    }
    // Otherwise: emit hash recomputation and comparison constraint
    let recomputed = lower_commit(tir.expr(value), tir);
    tir.emit_constraint(TirExpr::Eq(recomputed, commitment));
}
```
