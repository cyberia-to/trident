---
status: draft
author: mastercyb
area: cryptography
planned: 64K
---

# Merkle Proof Iterators

## Motivation

Merkle trees are the standard data structure for proving membership in a set without revealing the full set. Implementing Merkle proof verification correctly requires managing authentication paths, computing hash sequences, and checking root equality — repeated for every leaf accessed. In practice, developers copy-paste Merkle verification logic, introduce subtle bugs in path indexing or hash ordering, and write code that is structurally identical across every project.

When Merkle iteration is a language primitive, the compiler generates the authentication path management and hash chain computation automatically. The developer writes a loop; the compiler generates `merkle_step` TASM instructions. The STARK proof covers every Merkle membership check inside the loop body as part of the main execution proof — no separate proof structure needed.

## Design

### The Verified Walk Iterator

```trident
for (leaf, auth_path) in merkle_tree.verified_walk(root) {
    // Inside this loop body:
    //   - `leaf` is STARK-proven to be in the tree with the given root
    //   - `auth_path` is the current authentication path (for inspection/logging)
    //   - The merkle_step instructions have already been emitted
    //   - A proof violation here is an invalid STARK proof
    process(leaf);
}
```

The `verified_walk` method is a built-in iterator over a Merkle tree structure. At each iteration, the compiler has generated instructions to:

1. Read the leaf value from the tree
2. Compute the authentication path hash sequence from leaf to root
3. Verify the computed root equals the expected `root` parameter
4. If verification fails, the STARK constraint is violated — invalid proof

Inside the loop body, `leaf` carries the semantic guarantee: it is a member of the Merkle tree committed to by `root`. This guarantee is proven by the STARK, not by a runtime assertion.

### Compiler-Generated TASM

The compiler generates `merkle_step` instructions (a Triton VM primitive) for each level of the Merkle tree:

```
// Compiler output for a depth-4 Merkle tree:
// For each leaf at index i:
read_leaf(tree, i)
merkle_step(tree, auth_path[0], i >> 0)  // Level 0
merkle_step(tree, auth_path[1], i >> 1)  // Level 1
merkle_step(tree, auth_path[2], i >> 2)  // Level 2
merkle_step(tree, auth_path[3], i >> 3)  // Level 3 (root level)
assert_eq(computed_root, expected_root)   // STARK constraint: root must match
```

The `merkle_step` instruction is a TASM primitive that computes one hash combination step in the Merkle tree. It is optimized for Triton VM's hash table — more efficient than calling the general hash function manually.

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

More powerfully, for leaves that share a common ancestor (partially overlapping authentication paths), the compiler can share the common hash computations, reducing total Hash table rows.

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

**Hash function choice**: The built-in Merkle operations use Tip5 as the hash function, matching Triton VM's native hash instruction. Programs that need compatibility with external Merkle trees using SHA-256 or Poseidon must use the general hash primitive, losing the `merkle_step` optimization.

## Implementation Sketch

The Merkle iterator is a compiler builtin desugared during TIR construction:

```rust
// tir/merkle.rs
struct MerkleTreeType {
    depth: u32,
    hash_fn: HashFn,
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
        let leaf = tir.emit(TasmOp::ReadLeaf(tree.clone(), leaf_index));
        let mut computed = leaf.clone();

        for level in 0..tree_ty.depth {
            let sibling_index = leaf_index ^ (1 << level);
            let sibling = tir.emit(TasmOp::ReadLeaf(tree.clone(), sibling_index));
            let is_left = leaf_index & (1 << level) == 0;
            computed = tir.emit(TasmOp::MerkleStep(computed, sibling, is_left));
        }

        // Root equality constraint — must match expected root
        tir.emit_constraint(TirExpr::Eq(computed, root.clone()));

        // Bind `leaf` and `auth_path` for the loop body
        let loop_env = tir.extend_env([("leaf", leaf), ("auth_path", ...)]);
        lower_block_in_env(&loop_body, &loop_env, tir);
    }
}
```

For the ZK case, the leaf index is a private witness and the loop is replaced by a single iteration with a runtime-selected leaf — the compiler generates a different sequence that reads the leaf at a witness-supplied index rather than iterating all leaves.
