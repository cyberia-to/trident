---
status: draft
author: mastercyb
area: cryptography
planned: 64K
---

# Merkle Proof Iterators

**Related:** [[commitment-syntax]] · [[cybergraph]] · [[nox]] · [[hemera]] · [language.md §15](../reference/language.md#15-merkle-authentication)

## Vision

Every noun in [[nox]] is Merkle-addressed — `cons(a, b)` computes `hemera(a_hash ∥ b_hash)` and stores it as the parent hash. [[hemera]] (Poseidon2) is the hash function throughout, so every internal node is itself a particle in the [[cybergraph]]. `verified_walk(root)` traverses this structure with zero extra cost — the Merkle proofs fall out of the [[nox]] execution trace naturally, covered by the [[zheng]] proof of the execution.

In a program that traverses a subgraph of the [[cybergraph]], `verified_walk` over the Merkle tree of cyberlinks gives the developer authenticated iteration: every link encountered is proven to be in the graph at the given root, without any additional network round-trips. The traversal generates a sequence of `merkle_step` jet calls; [[zheng]] proves the whole walk in one proof. The [[cybergraph]] serves as both the data source and the global root anchor — the root hash is itself a [[hemera]] particle stored in the graph.

The `merkle_verify` jet (Layer 3 in [[nox]]) accelerates the `merkle_step` computation. `verified_walk` generates a sequence of `merkle_step` calls that the jet handles in a single precomputed batch — one jet invocation covers multiple levels of the tree. This is how [[warrior-cyber]] (the nox execution engine) achieves fast authenticated traversal of the [[cybergraph]]'s Merkle structure without blowing up the proof trace.

## Current Status

Language.md §15 already defines `merkle_step(idx: U32, d: Digest) -> (U32, Digest)` and `merkle_step_mem` as Tier 2 builtins. The reference spec even includes a worked example of a full Merkle path verification loop using `merkle_step`. This proposal is about an ergonomic iterator abstraction — `verified_walk` — that desugars to the existing `merkle_step` calls with hemera as the underlying hash, so the developer writes a loop but never manually manages authentication paths.

## Motivation

Merkle trees are the standard data structure for proving membership in a set without revealing the full set. Implementing Merkle proof verification correctly requires managing authentication paths, computing hash sequences, and checking root equality — repeated for every leaf accessed. In practice, developers copy-paste Merkle verification logic, introduce subtle bugs in path indexing or hash ordering, and write code that is structurally identical across every project.

When Merkle iteration is a language primitive, the compiler generates the authentication path management and hash chain computation automatically. The developer writes a loop; the compiler generates `merkle_step` nox instructions. The zheng proof covers every Merkle membership check inside the loop body as part of the main nox execution proof — no separate proof structure needed.

## Design

### The Verified Walk Iterator

```trident
for (leaf, auth_path) in merkle_tree.verified_walk(root) {
    // Inside this loop body:
    //   - `leaf` is STARK-proven to be in the tree with the given root
    //   - `auth_path` is the current authentication path (for inspection/logging)
    //   - The merkle_step instructions have already been emitted
    //   - A proof violation here is an invalid zheng proof
    process(leaf);
}
```

The `verified_walk` method is a built-in iterator over a Merkle tree structure. At each iteration, the compiler has generated instructions to:

1. Read the leaf value from the tree
2. Compute the authentication path hash sequence from leaf to root
3. Verify the computed root equals the expected `root` parameter
4. If verification fails, the nox constraint is violated — the zheng proof is invalid

Inside the loop body, `leaf` carries the semantic guarantee: it is a member of the Merkle tree committed to by `root`. This guarantee is proven by the STARK, not by a runtime assertion.

### Desugaring to `merkle_step`

`verified_walk` desugars to the existing `merkle_step` builtin (language.md §15). For a depth-4 tree, the compiler emits the pattern from the language spec reference example:

```trident
// Compiler output for a depth-4 Merkle tree:
// For each leaf at index i:
let mut idx = leaf_index
let mut current = leaf_hash  // hemera hash of the leaf value
for _ in 0..4 bounded 64 {
    (idx, current) = merkle_step(idx, current)  // one level up, hemera internally
}
assert_digest(current, root)  // nox constraint: root must match
```

`merkle_step` is a nox jet (one of the 5 built-in jets: hash, poly_eval, merkle_verify, fri_fold, ntt). It computes one hash combination step using hemera (Poseidon2) internally — more efficient than calling the general hash function manually, because it maps to a single nox jet invocation rather than multiple reduction steps.

### Authentication Path Management

The compiler manages the authentication path automatically. The tree structure (depth, hash function, left-right ordering) is determined at compile time from the tree's type parameters. The compiler knows:

- How many `merkle_step` instructions to generate per leaf
- The ordering convention (left sibling hashed on the left, right on the right)
- The path indexing (computed from the leaf index)

The developer never manually computes authentication paths, never manages path arrays, never indexes into sibling arrays. The compiler does all of this.

### Batch Verification

For multiple roots or multiple leaves verified against the same root, the compiler can batch Merkle proofs using shared hash computations:

```trident
// Two separate verified_walk calls over the same root:
for leaf in tree1.verified_walk(root) { process1(leaf); }
for leaf in tree2.verified_walk(root) { process2(leaf); }

// Compiler detects same root → can share root computation
// Authentication paths are independent, but root hash need not be recomputed
```

More powerfully, for leaves that share a common ancestor (partially overlapping authentication paths), the compiler can share the common hash computations, reducing total nox reduction steps for the merkle_step jet.

### Integration with ZK Functions

In a `zk fn`, the Merkle tree structure and root can be public while the specific leaf accessed is private:

```trident
zk fn process_member(
    tree:    Public<MerkleTree>,
    root:    Public<MerkleRoot>,
    secret_leaf: Private<Field>,   // which leaf to process is private
) -> Public<Field> {
    // The loop executes only once (for the private leaf index)
    // But the STARK proves the leaf is in the tree without revealing the index
    let result = tree.verify_member(secret_leaf, root);
    hash(result)
}
```

The Merkle membership proof becomes a zero-knowledge membership proof: the prover knows a leaf is in the tree without revealing which leaf.

## Key Tradeoffs

**Static tree structure**: The compiler generates optimal `merkle_step` sequences when the tree depth is known at compile time. For dynamically-sized Merkle trees, the compiler generates a loop over hash steps, which is less efficient than the straight-line sequence. The developer should declare tree depth as a `const` parameter when possible.

**Iterator vs. random access**: The `verified_walk` iterator processes leaves in order. Random access to a specific leaf is also supported (`tree.verify_at(index, root)`) but the compiler cannot share hash computations across unrelated random accesses. The iterator model is the high-performance path.

**Authentication path supply**: The authentication paths must be supplied as part of the Merkle tree structure at runtime. For programs that prove Merkle membership over trees they don't control (e.g., blockchain state trees), the authentication paths must be fetched and provided by the caller. This is a runtime concern, not a compiler concern — the compiler assumes authentication paths are available.

**Hash function choice**: The built-in Merkle operations use hemera (Poseidon2) as the hash function — `merkle_step` calls hemera internally. Programs that need compatibility with external Merkle trees using SHA-256 must use the general hash primitive, losing the `merkle_step` jet optimization. hemera replaced Tip5; any reference to Tip5-based Merkle trees refers to this implementation.

## Implementation Sketch

The Merkle iterator is a compiler builtin desugared during TIR construction:

```rust
// tir/merkle.rs
struct MerkleTreeType {
    depth: u32,
    // hash_fn is always hemera (Poseidon2) for merkle_step jet
    leaf_type: Type,
}

fn lower_verified_walk(
    tree: TirExpr,
    root: TirExpr,
    loop_body: TirBlock,
    tree_ty: &MerkleTreeType,
    tir: &mut TirBuilder,
) {
    let leaf_count = 1u64 << tree_ty.depth;

    for leaf_index in 0..leaf_count {
        let leaf = tir.emit(NoxOp::ReadLeaf(tree.clone(), leaf_index));
        let mut idx = tir.const_u32(leaf_index as u32);
        let mut current = leaf.clone();

        // Desugar to merkle_step calls (language.md §15, nox merkle_verify jet)
        for _level in 0..tree_ty.depth {
            let (new_idx, new_current) = tir.emit(NoxOp::MerkleStep(idx, current));
            idx = new_idx;
            current = new_current;
        }

        // Root equality nox constraint — must match expected root
        tir.emit_constraint(TirExpr::AssertDigest(current, root.clone()));

        // Bind `leaf` and `auth_path` for the loop body
        let loop_env = tir.extend_env([("leaf", leaf), ("auth_path", ...)]);
        lower_block_in_env(&loop_body, &loop_env, tir);
    }
}
```

For the ZK case, the leaf index is a private witness and the loop is replaced by a single iteration with a runtime-selected leaf — the compiler generates a different sequence that reads the leaf at a witness-supplied index rather than iterating all leaves.
