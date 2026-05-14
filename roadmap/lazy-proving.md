---
status: draft
author: mastercyb
area: runtime
planned: 64K
---

# Lazy Proof Batching

## Motivation

STARK proof generation has two cost components: variable cost (proportional to trace size) and fixed cost (Brakedown commitment, FRI queries, grinding). For small computations, the fixed cost dominates. Generating separate proofs for each individual computation is wasteful — each proof pays the full fixed cost regardless of trace size.

Lazy proof batching lets the developer explicitly group computations under a single proof. One STARK for the entire block, amortizing the fixed cost across all computations in the block. The developer controls the proof granularity — fine-grained for high-security operations that need individual attestation, coarse-grained for throughput-critical paths where amortization matters.

## Design

### The `defer_proof` Block

```trident
defer_proof {
    let x = expensive_computation_1();
    let y = expensive_computation_2();
    let z = x + y;
    // ... more computations
}
// ONE STARK proof for the entire block
// Fixed overhead paid once: Brakedown commitment, grinding, FRI queries
// Variable cost: sum of all computation trace sizes
```

Without `defer_proof`, three separate computations would generate three proofs — three Brakedown commitments, three grinding sessions, three FRI query sets. With `defer_proof`, these three computations share one proof structure. For small computations (say, 100 Processor rows each), the fixed proof overhead (say, equivalent to 1000 rows) would otherwise dominate each individual proof.

### Economics of Batching

Fixed proof overhead: $F$ rows equivalent.
Variable cost per computation: $V_i$ rows.
Cost without batching: $k \cdot (F + \bar{V})$ where $k$ is the computation count and $\bar{V}$ is the average variable cost.
Cost with batching: $F + \sum V_i$.

For $k = 10$, $F = 1000$, $\bar{V} = 100$:
- Without batching: $10 \times (1000 + 100) = 11{,}000$ rows equivalent
- With batching: $1000 + 1000 = 2{,}000$ rows equivalent
- Savings: 5.5×

The savings are largest when $F \gg \bar{V}$ (many small computations). For large computations ($\bar{V} \gg F$), batching provides negligible benefit — fixed cost is already amortized.

### Nested Batching

`defer_proof` blocks can be nested. The inner block's computations are included in the outer block's proof:

```trident
defer_proof {  // Outer block: one proof for everything
    let a = computation_1();

    defer_proof {  // Inner block: could be extracted as separate proof if needed
        let b = computation_2();
        let c = computation_3();
    }

    let d = a + inner_result;
}
```

Nesting allows the developer to group logically related computations while maintaining the option to split them out later. The compiler flattens nested `defer_proof` blocks into a single proof unless the developer explicitly requests splitting.

### Interaction with Incremental Proving

`defer_proof` and incremental proving compose. A `defer_proof` block that has been proven once can be incrementally updated when part of it changes:

```trident
let proof_v1 = prove(defer_proof {
    let x = computation_v1();
    let y = expensive_stable_computation();  // this won't change
});

// Later: computation_v1 changes, expensive_stable_computation stays the same
let proof_v2 = prove_delta(proof_v1, defer_proof {
    let x = computation_v2();  // changed
    let y = expensive_stable_computation();  // unchanged — reuse
});
```

The stable computation's Brakedown layers are reused. Only the changed computation's layers are recomputed. The fixed cost is paid once for the first proof; subsequent proofs pay only for the changes.

### Proof Granularity as a Design Dimension

The choice of proof granularity trades off latency against throughput:

- **Fine-grained proofs**: Each computation has its own proof. Latency is minimal (prove one small computation quickly). Throughput is limited by repeated fixed overhead.
- **Coarse-grained proofs (large `defer_proof` blocks)**: Many computations share one proof. Throughput is high (amortized fixed cost). Latency is high (must wait for all computations before any proof is available).
- **Hierarchical proofs**: Nested `defer_proof` enables intermediate granularity. Inner blocks provide per-group attestation; the outer block provides full-batch attestation.

The developer chooses based on the application's latency/throughput tradeoff. The `defer_proof` primitive makes this tradeoff explicit and adjustable.

## Key Tradeoffs

**Proof availability**: Values computed inside `defer_proof` are not proven until the block exits. If a value needs to be proven before the block exits (e.g., for a security-critical decision that depends on proof validity), `defer_proof` cannot be used. The developer must split the computation out of the deferred block.

**Memory pressure**: A large `defer_proof` block accumulates an AET table that grows throughout the block's execution. For very large blocks, the AET may exceed available RAM. The compiler can warn when estimated AET size approaches a configurable limit.

**Block boundaries and control flow**: `defer_proof` blocks must have static control flow — no `return` or `break` that exits the block prematurely. This ensures the entire block always executes before the proof is generated. The compiler enforces this constraint.

**Error handling**: If a computation inside `defer_proof` would normally trigger an error (e.g., a division by zero), the error becomes a proof invalidity — the STARK constraint corresponding to the error is violated. This is correct behavior, but the developer must be aware that error discovery is deferred to proof generation time, not execution time.

## Implementation Sketch

`defer_proof` is a TIR construct that wraps a block:

```rust
// tir/deferred.rs
struct DeferredProofBlock {
    body: TirBlock,
    parent_proof: Option<ProofHandle>,  // for incremental reuse
}

// The runtime executes the body, accumulates the AET, then proves once at block exit.
// tir/runtime.rs
fn execute_deferred(block: &DeferredProofBlock, state: &mut ExecutionState) -> Proof {
    let mut aet = AET::new();
    execute_block(&block.body, state, &mut aet);
    // One proof generation call for the entire accumulated AET:
    generate_proof(aet, block.parent_proof.as_ref())
}
```

The key implementation detail: `execute_block` runs normally, accumulating trace rows into the AET without generating any proof. `generate_proof` is called once at the end, with the full AET. For incremental proofs, `generate_proof` receives the previous proof and reuses stable Brakedown layers.
