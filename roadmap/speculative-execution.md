---
status: draft
author: mastercyb
area: runtime
planned: 64K
---

# Speculative Execution with Proof Rollback

**Related:** [[lazy-proving]] · [[incremental-proving]] · [[cybergraph]] · [[nox]] · [[soft3]] · [[bbg]]

## Vision

The [[cybergraph]] is an oracle — it knows which (formula, object) pairs have been computed before. Speculative execution exploits this: the fast path assumes the answer is cached in the graph, the fallback computes it. [[soft3]]'s `query()` call is itself a speculative execution: check the [[cybergraph]] cache first, compute only on miss. Trident programs can encode this pattern explicitly with `speculate`/`fallback`, and the [[zheng]] proof system ensures the fallback always produces a valid result.

```trident
speculate {
    // Fast path: assume cybergraph has the answer
    let result = soft3::query(formula, object);
} fallback {
    // Miss: compute from scratch, proven by zheng
    let result = compute(formula, object);
}
```

[[nox]]'s lazy branch evaluation (pattern 4 — only the taken path generates reduction steps) makes speculative execution natural. The untaken speculative branch never generates [[nox]] reduction steps, so it contributes nothing to the [[zheng]] proof cost. A cache hit is provably free: zero nox trace, zero sumcheck rows, zero focus charged by [[bbg]]. The `speculate` construct makes this explicit and compiler-enforced rather than a manual optimization.

## Motivation

Aggressive compiler optimizations are often "usually correct" — they work for the vast majority of inputs but fail on edge cases. In conventional systems, the developer must prove the optimization is always correct before deploying it. This prevents many worthwhile optimizations from ever shipping.

In a proof system, the proof mechanism itself serves as a correctness oracle. Run the speculative path, generate the proof, verify it. If valid, commit the result. If invalid (edge case triggered), discard the nox trace and re-execute the conservative fallback path, prove that instead. The proof system catches failures that the optimizer missed.

Speculative execution with proof rollback decouples the "is this optimization always correct?" question from the "should I try this optimization?" question. The developer says "try this aggressive path, but fall back if the proof fails." Correctness is guaranteed by construction.

## Design

### The `speculate`/`fallback` Construct

```trident
speculate {
    // Aggressively optimized path (may be incorrect for edge cases)
    let result = fast_but_risky_algorithm(input);
} fallback {
    // Conservative path (always correct, possibly slower)
    let result = slow_but_safe_algorithm(input);
}
// result is available here: either the speculative or fallback result
// The zheng proof covers whichever nox trace path was actually taken
```

The runtime:
1. Attempts the speculative block, accumulating a nox trace
2. Generates an intermediate zheng proof of the speculative nox trace
3. Verifies the proof
4. If valid: commits the result, uses the speculative zheng proof as the final proof
5. If invalid: discards the speculative nox trace (rollback = trace discard), re-executes the fallback path, proves the fallback trace

The caller receives a valid result in both cases. The proof covers the actual execution path taken.

### What "Proof Fails" Means

A zheng proof fails when a nox constraint is violated. In the context of speculative execution, failure means the speculative algorithm produced a result that violates some nox constraint — either an explicit `assert!`, a `#[requires]`/`#[ensures]` contract (see [[contracts]]), a refinement type predicate, or a loop invariant (see [[loop-invariants]]).

The failure is not a segfault or an exception. It is a mathematical refutation: the zheng proof oracle says "this nox execution trace does not satisfy the constraint polynomials." The runtime detects this, discards the nox trace, and rolls back.

```trident
speculate {
    // Fast modular exponentiation using precomputed tables
    // Correct for most inputs, but table may be stale for rare inputs
    let result = table_pow(base, exp);
    assert!(result == reference_pow(base, exp));  // constraint check
    // If table is stale: assert fails → proof invalid → rollback
} fallback {
    // Exact reference implementation (always correct)
    let result = reference_pow(base, exp);
}
```

### Use Cases

**Approximate algorithms**: Algorithms that are fast and correct for typical inputs but may fail on adversarial or rare inputs. Speculative execution provides a safety net.

**Cached computation**: A cache lookup that is valid most of the time but may be stale. `speculate { return cache[key]; } fallback { recompute and update cache; }`.

**Optimistic concurrency**: When multiple computations are expected to be independent, run them speculatively. If a dependency conflict is detected (via constraint failure), roll back and serialize.

**Compiler-emitted speculation**: The compiler itself uses `speculate` for algebraic optimizations with restricted validity domains. Instead of proving an optimization is universally correct, the compiler wraps it in a `speculate` block with the conservative implementation as fallback.

### Proof Cost of Speculation

Speculation adds overhead:
- Speculative block execution: full variable cost of the speculative path
- Intermediate proof generation: full fixed cost (FRI, grinding)
- Verification: fast (milliseconds for STARK verification)
- Fallback execution (if speculative fails): full variable cost of the fallback path

If the speculative path is taken 90% of the time and the fallback 10%, the average cost is:
$0.9 \times (\text{speculative cost} + \text{proof overhead}) + 0.1 \times (\text{fallback cost} + \text{proof overhead})$

For speculation to be worthwhile, the speculative path must be substantially faster than the fallback, and the speculative path must succeed frequently.

### Nested Speculation

`speculate` blocks can nest. The inner speculation is attempted first, then the outer:

```trident
speculate {
    speculate {
        result = ultra_fast_risky(input);
    } fallback {
        result = fast_risky(input);  // still speculative
    }
} fallback {
    result = slow_safe(input);       // always correct
}
```

Each level has its own proof verification. The innermost speculative block is cheapest to prove (smallest trace). Rollback proceeds outward if needed.

## Key Tradeoffs

**Proof cost of failed speculation**: When the speculative block fails, its proof overhead is wasted. For programs where the speculative path fails frequently (e.g., >10% of inputs), speculation adds cost rather than saving it. The developer should only speculate when the success rate is high.

**Side effects**: The speculative block must not have externally visible side effects. Writing to external storage, sending network messages, or triggering actions outside the Trident program would need to be undone on rollback — which is impossible in general. The compiler enforces that speculative blocks contain only pure field computations with no external effects.

**Determinism of rollback**: The fallback block must be deterministic and always correct. If the fallback block can also fail (produce an invalid proof), the entire `speculate`/`fallback` construct fails. The type system could enforce that the fallback block has an `ensures` contract that proves it always succeeds, but this places a verification burden on the developer.

**Incremental speculation**: For programs that are speculatively proven repeatedly (e.g., in a loop), [[incremental-proving]] can amortize the cost of repeated speculation. A speculative block that succeeds in the first iteration and again in subsequent iterations reuses the stable Brakedown PCS layers from the previous successful nox trace. See also [[lazy-proving]] for how `defer_proof` composes with speculation at block granularity.

## Implementation Sketch

```rust
// runtime/speculative.rs
pub fn execute_speculate(
    speculative: &TirBlock,
    fallback: &TirBlock,
    state: &ExecutionState,
) -> (ExecutionResult, ZhengProof) {
    // Try speculative path — accumulate a nox trace
    let (spec_result, spec_trace) = execute_block(speculative, &mut state.clone());
    let spec_proof = generate_zheng_proof(spec_trace);

    match verify_zheng_proof(&spec_proof) {
        ProofValid => (spec_result, spec_proof),
        ProofInvalid => {
            // Proof rollback = nox trace discard + re-execute fallback path
            // spec_trace is dropped here; no partial state escapes
            let (fb_result, fb_trace) = execute_block(fallback, &mut state.clone());
            let fb_proof = generate_zheng_proof(fb_trace);
            (fb_result, fb_proof)
        }
    }
}
```

The key implementation requirement: the speculative block runs in a fully isolated state copy. Rollback simply discards the nox trace accumulated during speculation. The fallback runs in a fresh copy of the original state. No partial state mutations escape before the zheng proof is verified.
