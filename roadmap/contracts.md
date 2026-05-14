---
status: draft
author: mastercyb
area: verification
planned: 32K
---

# Requires/Ensures Contracts Compiled to nox Constraints

**Related:** [[loop-invariants]] · [[termination-proofs]] · [[refinement-types]] · [language.md §7](../reference/language.md#7-attributes)

## Current Status

The `#[requires]`, `#[ensures]`, and `#[pure]` attribute syntax already exists in the language spec ([language.md §7](../reference/language.md#7-attributes)). `trident audit` checks them via symbolic execution today. This proposal is about the next step: compiling these attributes into nox constraints so that a zheng proof of execution simultaneously proves specification compliance — without a separate audit pass.

## Motivation

Formal verification usually involves a separate tool. You write your program, you write specifications in a verification language, you run the verifier, you get a separate proof artifact. Two artifacts, two trust chains, two maintenance burdens. Every change to the program potentially invalidates the verification, and the developer must re-verify separately.

In Trident, program execution and formal verification share the same artifact: the zheng proof of the nox execution trace. When a function carries `#[requires]`/`#[ensures]` attributes, those clauses compile to additional nox constraints. The execution proof IS the verification proof. One proof, one trust chain. The proof is valid if and only if both the program terminated correctly AND all specifications were satisfied.

## Design

### Syntax

The attribute syntax is already in the language spec. The compilation target changes from symbolic audit to nox constraints:

```trident
#[requires(x < p/2)]
#[ensures(result * result - x < EPSILON)]
fn sqrt_approx(x: Field) -> Field {
    // Padé approximant or Newton-Raphson implementation
    let y = x * INITIAL_GUESS;
    for _ in 0..3 {
        y = (y + x * invert(y)) * INV_2;
    }
    y
}
```

`#[requires]` is a precondition: the caller must ensure it holds before the call. `#[ensures]` is a postcondition: the callee guarantees it holds when the function returns. `result` in `#[ensures]` refers to the return value.

### Compilation Model

`#[requires]` clauses compile to nox constraints on the inputs at the call site. In nox, `reduce(object, formula, hints)` is the core operation — the execution trace IS the witness. When `sqrt_approx` is called with argument `x`:

1. The constraint `x < p/2` is emitted as a nox constraint at the reduction step corresponding to the call
2. If this constraint is not satisfied, the constraint evaluates to a nonzero value in the nox trace
3. A nonzero value means the zheng proof (via Brakedown PCS + sumcheck) is invalid
4. The verifier rejects the proof

The result: calling `sqrt_approx` with an out-of-range input does not produce a wrong answer — it produces an unprovable computation. The proof system enforces the contract.

`#[ensures]` clauses compile similarly, but constrain the output rather than the input. The constraint is checked on the return value before it flows back to the caller.

### One Proof, Two Purposes

The same zheng proof of the nox trace serves as both:
- Proof of execution (the program ran and produced this output)
- Proof of specification compliance (all `#[requires]` and `#[ensures]` clauses were satisfied)

A verifier who checks the zheng proof learns both simultaneously. This is the key property: verification is not a post-hoc step. It is embedded in the proof of execution. If you trust the zheng proof of the nox trace, you trust the specification compliance.

### Caller Obligations

When a function has a `#[requires]` clause, the caller carries the obligation to satisfy it. The compiler tracks this obligation through the call chain:

```trident
fn caller(x: Field) -> Field {
    // ERROR: must prove x < p/2 before calling sqrt_approx
    sqrt_approx(x)
}

fn safe_caller(x: Field) -> Field {
    // Option 1: runtime check (generates a nox constraint)
    assert!(x < p/2);
    sqrt_approx(x)

    // Option 2: type-level proof (x: Positive implies x < p/2)
    // If x: Positive is in scope, no runtime constraint needed
}
```

The compiler attempts to discharge preconditions statically from the type context (see [[refinement-types]]). If it cannot, it generates a runtime nox constraint. If the precondition is violated at runtime, the proof is invalid.

### Ensures and Result Types

The `#[ensures]` attribute can reference the result value, other parameters, and pre-call values of mutable parameters. A standard pattern for preservation properties:

```trident
#[ensures(is_permutation(arr, result))]
#[ensures(is_sorted(result))]
fn sort_array(arr: [Field; N]) -> [Field; N] {
    // sorting implementation
}
```

Both postconditions compile to nox constraints. A proof of `sort_array` execution is also a proof that the output is a sorted permutation of the input.

Loop body contracts connect directly to [[loop-invariants]]: when the loop invariant at termination implies the function's `#[ensures]` postcondition, the compiler can discharge the postcondition without a separate nox constraint.

## Key Tradeoffs

**Constraint cost**: Each `#[requires]`/`#[ensures]` clause adds nox constraints. For functions called in tight loops, these constraints appear at every reduction step in the loop's trace — potentially doubling the reduction step count for the function. The developer must be deliberate about where contracts are placed. See [[loop-invariants]] for the interaction with loop-level invariants.

**Static discharge**: The compiler attempts to statically discharge preconditions from type information (refinement types, linear types). This is the preferred path — zero runtime cost. For conditions that cannot be discharged statically, the runtime nox constraint is unavoidable.

**Quantified specifications**: The `#[ensures]` example `is_permutation(arr, result)` is itself a non-trivial predicate. If `is_permutation` is an expensive function to compute, its evaluation at the call site adds significant reduction steps to the nox trace. Specifications must be efficient to evaluate, not just correct to state.

**No separate verifier**: Specifications must be expressible as polynomial constraints over the nox execution trace. Specifications that require reasoning about infinite sets or unbounded computation cannot be expressed this way. In practice, most preconditions and postconditions are efficiently checkable. See also [[termination-proofs]] for how bounded execution interacts with this constraint.

## Implementation Sketch

Contracts integrate with TIR construction. The compiler generates constraint nodes from `#[requires]`/`#[ensures]` attributes and inserts them at the appropriate nox trace locations:

```rust
// tir/contracts.rs
struct Contract {
    requires: Vec<TirExpr>,   // must hold on entry
    ensures: Vec<TirExpr>,    // must hold on exit
}

fn lower_function_with_contract(
    func: &AstFunction,
    contract: &Contract,
    tir: &mut TirBuilder,
) {
    // Emit requires constraints at function entry (nox reduction step)
    for req in &contract.requires {
        let constraint_node = tir.emit_constraint(req.clone());
        tir.mark_as_proof_relevant(constraint_node);
    }

    // Lower function body
    lower_body(&func.body, tir);

    // Emit ensures constraints at function exit, binding `result`
    let result_var = tir.current_return_value();
    for ens in &contract.ensures {
        let bound = ens.bind("result", result_var);
        let constraint_node = tir.emit_constraint(bound);
        tir.mark_as_proof_relevant(constraint_node);
    }
}
```

The `mark_as_proof_relevant` call ensures these nodes survive dead field operation elimination (Pass 14) — they must appear in the nox trace even if the computed values are not used elsewhere in the program logic.
