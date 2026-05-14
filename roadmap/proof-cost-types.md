---
status: draft
author: mastercyb
area: type system
planned: 32K
---

# Proof-Cost Type Annotations

**Related proposals:** [[table-aware-types]], [[refinement-types]], [[proof-cost-ide]], [[trace-predictor]]
**Reference:** [language.md §7 — Attributes (#[requires], #[ensures], #[pure])](../reference/language.md), [ir.md](../reference/ir.md)

## Motivation

In conventional languages, function signatures tell you the input and output types. In Trident, that is insufficient. A function that takes a `Field` and returns a `Field` might cost 10 nox reduction steps or 10,000. The caller has no way to know without reading the implementation. In a system where proof cost is the primary resource, hiding cost behind opaque function boundaries is a design error.

Proof-cost types solve this by making cost a first-class part of every function's interface. A function that declares `cost [steps: 800..1200, jets: 50..100]` is contractually bound: the compiler verifies the implementation stays within bounds, and callers can reason about composition without inspecting internals.

## Design

### Cost Annotations on Function Signatures

The `cost` attribute follows the `#[requires]`/`#[ensures]` attribute convention (language.md §7) — it is a first-class function attribute checked by the compiler.

```trident
fn transfer(a: Account, b: Account, amount: Field) -> Result<(), Error>
  #[cost(steps: 800..1200, jets: 50..100)]
{
    // implementation
}
```

The `cost` attribute specifies bounds on nox execution cost. In the nox VM, proof cost has two components: reduction steps (the 16 nox patterns applied during evaluation) and jet invocations (the 5 built-in jets that implement expensive primitives like hashing). The compiler statically verifies:

1. The implementation never produces fewer reduction steps than the lower bound (guards against incorrect bounds claims)
2. The implementation never produces more reduction steps than the upper bound (guards against budget overruns)

If either check fails, compilation fails with a specific cost-violation error showing the actual measured nox trace cost versus the declared bound.

### Compositional Cost Reasoning

Cost bounds compose additively. If `f` declares `#[cost(steps: 100..200)]` and `g` declares `#[cost(steps: 300..400)]`, then calling `f` then `g` has a step count in `[400..600]`. The type checker propagates these intervals through the call graph.

```trident
fn verify_batch(items: [Item; N]) -> bool
  #[cost(steps: N * 500..N * 800, jets: N * 200..N * 300)]
{
    for item in items {
        verify_single(item);  // #[cost(steps: 500..800, jets: 200..300)]
    }
    true
}
```

The $N$ factor is a dependent cost bound — the interval scales with the compile-time-known constant $N$. The compiler evaluates this symbolically and verifies the inner loop's cost contribution matches the outer bound.

### Optimization-Preserving Bounds

A subtle design constraint: the compiler must not apply an optimization that violates a declared cost bound, even if the optimization reduces total proof cost. If a function declares `#[cost(steps: 100..200)]` and an optimization would reduce the nox trace to 80 steps, the optimization is rejected — the function's interface promises at least 100 steps, and callers depend on that contract.

This may seem counterproductive. The reasoning: in a system where cost bounds are part of the API contract, an optimization that changes the observable cost profile is a breaking change. Callers use cost bounds for scheduling (table-aware types, parallel proving) and budget allocation. Silent cost changes break these invariants.

Developers who want aggressive optimization should declare wide bounds (or omit the lower bound). The lower bound should only be set when the caller has a semantic reason to expect a minimum cost.

### Developer Visibility

Proof-cost types make proof cost visible at the interface level. A developer integrating a library function sees its cost profile in the type signature — no source inspection required. This is the field-arithmetic analogue of knowing that a function is $O(n \log n)$ versus $O(n^2)$, but precise and mechanically verified rather than asymptotic and argued.

```trident
// Library function — caller sees cost at type level:
extern fn hemera_hash(input: [Field; 8]) -> Field
  #[cost(steps: 50..80, jets: 1)];

// IDE shows: "hemera_hash: ~65 nox steps, 1 jet invocation"
```

The hash jet uses hemera (Poseidon2, p=2^64-2^32+1) as the underlying primitive. Its cost is expressed in jet invocations, not internal hash rounds.

## Key Tradeoffs

**Static vs. dynamic cost**: For programs with data-dependent control flow (branches, loops with runtime-determined counts), static cost bounds must be conservative worst-case estimates. This may over-report cost for programs whose expensive path is rarely taken. A mitigation: allow probabilistic cost annotations for expected-case bounds, with separate worst-case bounds.

**Verification overhead**: Computing the actual nox trace cost of a function requires either executing it (expensive) or running the cost model over the TIR (approximate). The compiler uses the cost model during compilation — it is fast but not exact. Functions whose cost is near a bound boundary may require actual execution to verify definitively.

**Bound width**: Wide bounds are always satisfiable but useless for reasoning. Narrow bounds are precise but may be violated by small implementation changes. The ecosystem needs tooling to suggest appropriate bounds — see [[proof-cost-ide]] and [[trace-predictor]] for the proposed tooling that suggests bounds based on observed nox trace costs during development.

**Interaction with supercompilation**: Supercompilation may dramatically reduce a function's cost below its declared lower bound. When supercompilation is applied, the compiler should warn rather than fail — the developer likely needs to tighten the declared bounds.

## Implementation Sketch

Cost bounds live in the type checker and cost model. The cost of a nox function is the sum of its pattern applications (each of the 16 reduction patterns has a known cost weight) plus jet invocations (5 jets, each with a fixed cost from the nox cost table):

```rust
// typecheck/cost_types.rs
struct CostBound {
    steps: Range<u64>,  // nox reduction steps (pattern applications)
    jets:  Range<u64>,  // nox jet invocations
}

fn verify_cost_bound(
    func: &TirFunction,
    declared: &CostBound,
    cost_model: &CostModel,
) -> Result<(), CostViolation> {
    let actual = cost_model.estimate(func);
    let actual_steps = actual.reduction_steps;
    let actual_jets  = actual.jet_invocations;
    if actual_steps < declared.steps.start {
        return Err(CostViolation::BelowLowerBound { dim: "steps", actual: actual_steps, bound: declared.steps });
    }
    if actual_steps > declared.steps.end {
        return Err(CostViolation::ExceedsUpperBound { dim: "steps", actual: actual_steps, bound: declared.steps });
    }
    // same check for jets
    Ok(())
}
```

Cost composition runs as a type inference pass over the call graph, propagating interval arithmetic bottom-up from leaf functions to roots.
