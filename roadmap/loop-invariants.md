---
status: draft
author: mastercyb
area: verification
planned: 32K
---

# Invariant-Carrying Loops

## Motivation

Loop invariants are the standard technique for proving properties of iterative computations. In conventional verification tools (Dafny, Frama-C, Why3), invariants are checked by an SMT solver at the beginning and end of each loop iteration — the inductive step must hold. But the check is separate from the program's execution. A valid SMT proof does not guarantee that the running program maintains the invariant; it guarantees that the program, as modeled by the verifier, does.

In Trident, loop invariants become inductive STARK constraints. The invariant is checked at every iteration as part of the execution trace. If the invariant fails at any point during execution, the STARK proof is invalid — not just flagged, but mathematically refuted. The running program and the verification are the same artifact.

## Design

### Invariant Syntax

```trident
fn sum_array(arr: [Field; N]) -> Field {
    let mut acc: Field = 0;
    for i in 0..N
      invariant acc == sum(arr[0..i])
    {
        acc = acc + arr[i];
    }
    acc
}
```

The `invariant` clause states a property that must hold at the start of every loop iteration. At $i = 0$: `acc == sum(arr[0..0]) == 0` — trivially true. After iteration $k$: `acc == sum(arr[0..k])`. The invariant is inductive if the loop body maintains it. If it does not, the STARK proof is invalid.

### Compilation Model

Each invariant clause compiles to a STARK constraint that is checked at every loop iteration. The constraint polynomial evaluates to zero if and only if the invariant holds. If the invariant fails at any iteration, the polynomial is nonzero at that trace row — producing an invalid proof.

```trident
// Invariant: acc == sum(arr[0..i])
// Compiled constraint: acc - partial_sum(arr, i) == 0
// Where partial_sum is a helper that computes the running sum
```

For the summation example, `sum(arr[0..i])` is itself a summation that would be expensive to evaluate fresh at every iteration. The compiler recognizes this structure and generates the constraint incrementally: the invariant check at iteration $i+1$ is `acc_new == acc_old + arr[i]`, which is a single addition — not a full re-sum.

This incremental compilation is the key efficiency insight: invariants over accumulating computations compile to incremental constraints that cost constant time per iteration, not time proportional to loop progress.

### Multiple Invariants

A loop can carry multiple invariants:

```trident
fn max_array(arr: [Field; N]) -> Field {
    let mut max_so_far: Field = 0;
    for i in 0..N
      invariant max_so_far <= arr[j] for all j < i implies false  // not right — see below
      invariant max_so_far == max(arr[0..i])
    {
        if arr[i] > max_so_far { max_so_far = arr[i]; }
    }
    max_so_far
}
```

Each invariant compiles to a separate constraint. The conjunction of all constraints must hold at every iteration. Failure of any one produces an invalid proof.

### Invariant as Proof of Algorithmic Correctness

The postcondition of the loop (the state after the last iteration) follows directly from the invariant. For `sum_array`, at $i = N$: `acc == sum(arr[0..N])` — which is exactly what the function is supposed to return. The loop invariant is a proof of correctness for the algorithm.

This connects to `ensures` clauses: when the loop invariant at termination implies the function's postcondition, the compiler can discharge the postcondition automatically — no separate STARK constraint needed.

```trident
fn sum_array(arr: [Field; N]) -> Field
  ensures result == sum(arr)  // discharged from invariant
{
    let mut acc: Field = 0;
    for i in 0..N
      invariant acc == sum(arr[0..i])  // at i=N: acc == sum(arr[0..N]) == sum(arr)
    {
        acc = acc + arr[i];
    }
    acc
}
```

## Key Tradeoffs

**Invariant cost**: Each invariant adds STARK constraints at every loop iteration. For a loop of $N$ iterations with $k$ invariants, this adds $O(k \cdot N)$ constraint evaluations to the trace. For tight inner loops (thousands of iterations), expensive invariants dominate the proof cost. The developer must weigh verification value against proof cost.

**Invariant choice**: The invariant must be inductive — the loop body must maintain it. Choosing an inductive invariant is often the hard part of loop verification. The compiler checks inductiveness (approximately) by symbolic execution of one iteration from a generic state satisfying the invariant. If it cannot verify inductiveness statically, it reports a warning and relies on the STARK constraint to catch failures at runtime.

**Helper functions in invariants**: Invariants that call complex helper functions (like `sum(arr[0..i])`) embed those function calls in the constraint. The compiler must either unfold the helper (increasing constraint complexity) or use the incremental compilation strategy (recognizing accumulation patterns). For invariants over non-accumulating properties, the helper is evaluated fresh each iteration — which may be acceptable or prohibitive depending on the helper's cost.

**No infinite loops**: Trident already requires bounded loops (fixed bounds at compile time). This makes invariant reasoning decidable — the invariant must hold for exactly $N$ iterations where $N$ is a constant. No concerns about infinite loops or non-terminating invariant checking.

## Implementation Sketch

Loop invariants integrate with TIR loop lowering:

```rust
// tir/loops.rs
struct InvariantCarryingLoop {
    body: TirBlock,
    count: TirExpr,
    invariants: Vec<TirExpr>,  // expressions that must evaluate to zero each iteration
}

fn lower_invariant_loop(
    loop_: &InvariantCarryingLoop,
    tir: &mut TirBuilder,
) {
    // Emit loop preamble
    let counter = tir.fresh_var();

    for iter in 0..loop_.count {
        // Check invariants at top of each iteration
        for inv in &loop_.invariants {
            let check = tir.emit_constraint(inv.substitute("i", counter));
            tir.mark_as_proof_relevant(check);
        }

        // Execute loop body
        lower_block(&loop_.body, tir);

        tir.increment(counter);
    }
}
```

The incremental optimization for accumulation patterns runs as a separate analysis pass that identifies invariants of the form `acc == f(arr[0..i])` where `f` admits an incremental update formula, and rewrites the constraint to use the incremental form.
