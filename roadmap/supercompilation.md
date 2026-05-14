---
status: draft
author: mastercyb
area: compiler
planned: 16K
---

# Supercompilation for Proof Machines

## Motivation

Every algebraic pass in the compiler is a local transformation: see pattern, apply rewrite, move on. Local optimizations compose additively. Supercompilation — Valentin Turchin's technique of driving, folding, and generalization — is global. It symbolically executes the entire program, tracks the shape of every computation, and collapses iterative processes to closed forms. Applied to an algebraic virtual machine operating over Goldilocks, it produces gains no local pass can match: linear recurrences over field elements become single exponentiation expressions, and a loop of 1000 Processor rows becomes 10.

No proof-native language has implemented supercompilation. The combination of symbolic field execution and closed-form discovery is entirely unexplored territory.

## Design

### Driving: Symbolic Field Execution

The supercompiler executes the program symbolically. Every variable holds either a concrete field element (constant) or a symbolic expression. Field identities propagate through the execution: if `a = 3` and `b = 5`, then `a * b` immediately reduces to `15`. If `a = x` (symbolic) and `b = 0`, then `a * b` reduces to `0` regardless of `x`.

For Trident, driving propagates not just arithmetic but field-theoretic constraints: the supercompiler knows $x^{p-1} = 1$ for any nonzero symbolic $x$, applies Fermat reduction automatically, detects roots-of-unity shifts, and folds constant sub-expressions. A fully driven loop body often reduces by 30–60% before the loop structure is even analyzed.

### Folding: Iterative to Closed Form

The key algorithmic contribution. When the supercompiler's symbolic state at iteration $n+1$ is a generalization of its state at iteration $n$ (same structure, different symbolic parameters), it recognizes convergence. Instead of continuing to unroll, it identifies the recurrence relation and replaces the loop with a closed-form expression.

The canonical case is the linear recurrence $x_{n+1} = a \cdot x_n + b$:

```trident
// Source: n iterations of a linear recurrence
fn iterate(x0: Field, a: Field, b: Field, n: Field) -> Field {
    let mut x = x0;
    for _ in 0..n {
        x = a * x + b;
    }
    x
}
```

The supercompiler recognizes the pattern and replaces it with:

```trident
// Closed form: a^n * x0 + b * (a^n - 1) / (a - 1)
fn iterate(x0: Field, a: Field, b: Field, n: Field) -> Field {
    let an = pow(a, n);
    an * x0 + b * (an - 1) * invert(a - 1)
}
```

Processor table rows: $n$ iterations → $O(\log n)$ for the exponentiation. For $n = 1000$, this is roughly 3 orders of magnitude.

**Goldilocks advantage**: Recurrences involving powers of 2 (common in cryptographic contexts) have compact closed forms because $2^k \bmod p$ reduces via the golden-ratio identity. The supercompiler discovers these automatically.

### Partial Evaluation via `specialize`

The `specialize` keyword triggers supercompilation at compile time for a specific argument configuration:

```trident
fn generic_hash<const ROUNDS: Field>(input: Field) -> Field {
    let mut state = input;
    for _ in 0..ROUNDS {
        state = state * state + ROUND_CONSTANT;
    }
    state
}

// Compile-time specialization with ROUNDS = 5:
let hash_5 = specialize(generic_hash, ROUNDS = 5);
// Result: fully unrolled, constant-folded, algebraically simplified
// TASM output: ~10 straight-line instructions
```

The specialized function carries no loop overhead, no dynamic dispatch, no runtime counter. Its STARK proof is minimal — exactly the cost of the 5 squarings and additions, after constant folding has absorbed the round constant arithmetic.

## The Multiplicative Gain Stack

Supercompilation amplifies every other pass:

1. Supercompiler unfolds a loop, discovers a linear recurrence
2. Algebraic pass replaces the recurrence with `pow(a, n) * x0 + ...`
3. Addition chain optimizer computes the `pow(a, n)` chain
4. Multi-exponentiation fusion combines with nearby power computations
5. Constant folder evaluates everything that depends only on constants
6. Dead field eliminator removes the now-unused loop variables

A loop of 1000 Processor rows becomes:
- After supercompilation: $O(\log n)$ via closed form → ~10 multiply rows
- After addition chain: ~9 rows (optimal chain for the specific $n$)
- After multi-exponentiation fusion with other nearby pows: ~7 rows
- After constant folding (if $a$ and $b$ are compile-time constants): 0 rows

Each pass multiplies the benefit of the previous. This is why supercompilation must run first in the pass pipeline.

## Key Tradeoffs

**Termination**: The supercompiler must not diverge on recursive programs. Folding provides the termination guarantee: when a previously seen state recurs (up to generalization), the supercompiler creates a recursive call rather than continuing to unfold. But recognizing "previously seen state" requires a generalization algorithm that is conservative by design — too conservative means missed opportunities, too aggressive means infinite unfolding.

**False closed forms**: Not every loop is a linear recurrence. The supercompiler must verify that the discovered closed form actually equals the loop for all inputs. It uses the Schwartz-Zippel oracle (Pass 6) for this — evaluate both forms at a random point, require agreement.

**Scope**: Supercompilation is exponential in the worst case. The compiler must bound the unfolding depth (configurable, default: 8 recursive steps) and fall back to local passes when the bound is hit. Programs with complex control flow (many branches, nested loops with data-dependent bounds) may not benefit from supercompilation at all.

**Goldilocks-specific recurrences**: The supercompiler needs a library of known closed-form patterns. The linear recurrence is one. Geometric series, exponential decay, and modular counting sequences are others. Each must be proven correct once and added to the pattern library. This library grows over time as new patterns are discovered.

## Implementation Sketch

Supercompilation requires a symbolic evaluator over TIR:

```rust
// tir/supercompile/driver.rs
enum SymbolicValue {
    Concrete(FieldElement),
    Symbolic(TirExpr),
}

struct SuperState {
    env: HashMap<TirVarId, SymbolicValue>,
    history: Vec<SuperState>,  // for fold detection
}

fn drive(node: &TirNode, state: &mut SuperState) -> SymbolicValue {
    match node {
        Add(a, b) => {
            let va = drive(a, state);
            let vb = drive(b, state);
            field_add_symbolic(va, vb)  // applies algebraic identities
        }
        Loop { body, count } => {
            try_fold_to_closed_form(body, count, state)
                .unwrap_or_else(|| drive_loop_unrolled(body, count, state))
        }
        // ...
    }
}

fn try_fold_to_closed_form(
    body: &TirBlock,
    count: &TirNode,
    state: &SuperState,
) -> Option<SymbolicValue> {
    // Pattern match for known recurrence types
    // Validate via Schwartz-Zippel
    // Return closed-form TIR if found
}
```

The supercompiler is planned for 16K — later than the algebraic passes (32K) because it requires full compiler maturity: the algebraic passes must be stable, the symbolic evaluator must correctly implement all field identities, and the Schwartz-Zippel oracle must be in place. Supercompilation is the capstone of the compiler's algebraic intelligence, not its foundation.
