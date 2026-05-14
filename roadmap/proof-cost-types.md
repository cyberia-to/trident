---
status: draft
author: mastercyb
area: type system
planned: 32K
---

# Proof-Cost Type Annotations

## Motivation

In conventional languages, function signatures tell you the input and output types. In Trident, that is insufficient. A function that takes a `Field` and returns a `Field` might cost 10 Processor rows or 10,000. The caller has no way to know without reading the implementation. In a system where proof cost is the primary resource, hiding cost behind opaque function boundaries is a design error.

Proof-cost types solve this by making cost a first-class part of every function's interface. A function that declares `cost [processor: 800..1200, hash: 50..100]` is contractually bound: the compiler verifies the implementation stays within bounds, and callers can reason about composition without inspecting internals.

## Design

### Cost Annotations on Function Signatures

```trident
fn transfer(a: Account, b: Account, amount: Field) -> Result<(), Error>
  cost [processor: 800..1200, hash: 50..100, ram: 200..400]
{
    // implementation
}
```

The `cost` clause specifies bounds on each AET table. The compiler statically verifies:

1. The implementation never produces fewer rows than the lower bound (guards against incorrect bounds claims)
2. The implementation never produces more rows than the upper bound (guards against budget overruns)

If either check fails, compilation fails with a specific cost-violation error showing the actual measured cost versus the declared bound.

### Compositional Cost Reasoning

Cost bounds compose additively. If `f` declares `cost [processor: 100..200]` and `g` declares `cost [processor: 300..400]`, then calling `f` then `g` has processor cost in `[400..600]`. The type checker propagates these intervals through the call graph.

```trident
fn verify_batch(items: [Item; N]) -> bool
  cost [processor: N * 500..N * 800, hash: N * 200..N * 300]
{
    for item in items {
        verify_single(item);  // cost [processor: 500..800, hash: 200..300]
    }
    true
}
```

The $N$ factor is a dependent cost bound — the interval scales with the compile-time-known constant $N$. The compiler evaluates this symbolically and verifies the inner loop's cost contribution matches the outer bound.

### Optimization-Preserving Bounds

A subtle design constraint: the compiler must not apply an optimization that violates a declared cost bound, even if the optimization reduces total proof cost. If a function declares `cost [processor: 100..200]` and an optimization would reduce processor cost to 80, the optimization is rejected — the function's interface promises processor cost is at least 100, and callers depend on that contract.

This may seem counterproductive. The reasoning: in a system where cost bounds are part of the API contract, an optimization that changes the observable cost profile is a breaking change. Callers use cost bounds for scheduling (table-aware types, parallel proving) and budget allocation. Silent cost changes break these invariants.

Developers who want aggressive optimization should declare wide bounds (or omit the lower bound). The lower bound should only be set when the caller has a semantic reason to expect a minimum cost.

### Developer Visibility

Proof-cost types make proof cost visible at the interface level. A developer integrating a library function sees its cost profile in the type signature — no source inspection required. This is the field-arithmetic analogue of knowing that a function is $O(n \log n)$ versus $O(n^2)$, but precise and mechanically verified rather than asymptotic and argued.

```trident
// Library function — caller sees cost at type level:
extern fn poseidon_hash(input: [Field; 8]) -> Field
  cost [processor: 200..300, hash: 800..1200];

// IDE shows: "poseidon_hash: Processor ≈250 rows, Hash ≈1000 rows"
```

## Key Tradeoffs

**Static vs. dynamic cost**: For programs with data-dependent control flow (branches, loops with runtime-determined counts), static cost bounds must be conservative worst-case estimates. This may over-report cost for programs whose expensive path is rarely taken. A mitigation: allow probabilistic cost annotations for expected-case bounds, with separate worst-case bounds.

**Verification overhead**: Computing the actual AET cost of a function requires either executing it (expensive) or running the cost model over the TIR (approximate). The compiler uses the cost model during compilation — it is fast but not exact. Functions whose cost is near a bound boundary may require actual execution to verify definitively.

**Bound width**: Wide bounds are always satisfiable but useless for reasoning. Narrow bounds are precise but may be violated by small implementation changes. The ecosystem needs tooling to suggest appropriate bounds — perhaps the REPL or proof explorer suggests bounds based on observed costs during development.

**Interaction with supercompilation**: Supercompilation may dramatically reduce a function's cost below its declared lower bound. When supercompilation is applied, the compiler should warn rather than fail — the developer likely needs to tighten the declared bounds.

## Implementation Sketch

Cost bounds live in the type checker and cost model:

```rust
// typecheck/cost_types.rs
struct CostBound {
    processor: Range<u64>,
    hash: Range<u64>,
    ram: Range<u64>,
    // ... other tables
}

fn verify_cost_bound(
    func: &TirFunction,
    declared: &CostBound,
    cost_model: &CostModel,
) -> Result<(), CostViolation> {
    let actual = cost_model.estimate(func);
    for table in Table::all() {
        let actual_rows = actual.rows(table);
        let bound = declared.bound(table);
        if actual_rows < bound.start {
            return Err(CostViolation::BelowLowerBound { table, actual: actual_rows, bound });
        }
        if actual_rows > bound.end {
            return Err(CostViolation::ExceedsUpperBound { table, actual: actual_rows, bound });
        }
    }
    Ok(())
}
```

Cost composition runs as a type inference pass over the call graph, propagating interval arithmetic bottom-up from leaf functions to roots.
