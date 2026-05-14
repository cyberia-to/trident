---
status: draft
author: mastercyb
area: type system
planned: 32K
---

# Table-Constraint Types for Parallel Proving

**Related proposals:** [[proof-cost-types]], [[compiler-ensemble]], [[lazy-proving]]
**Reference:** [language.md §2 — Types](../reference/language.md), [ir.md](../reference/ir.md)

## Motivation

In nox, a program's execution trace is the STARK witness: each reduction step and jet invocation writes to the trace. Some operations are purely arithmetic (field additions, multiplications via pattern 5); others invoke jets (hemera hash, u32 arithmetic). Two program segments that use disjoint subsets of nox operations produce independent subtrace segments that can be proved in parallel by separate warrior-cyber instances, then composed.

The insight: if the compiler knows at the type level which nox operations a function uses, it can schedule independently constrained subtrace segments for parallel proving and eliminate inter-segment synchronization where it isn't needed.

Table-constraint types encode nox operation membership directly in the type system.

## Design

### Type Definitions

```trident
type HashFree<T> = T where ops_used(T) ∩ {jet::hash} = ∅
type ArithOnly<T> = T where ops_used(T) ⊆ {pattern::arith}
type JetHeavy<T>  = T where ops_used(T) ∩ {jet::hash, jet::u32, jet::ram} ≠ ∅
```

These are refinements of function types. `HashFree<T>` means the value of type `T` was computed without invoking the hemera hash jet. `ArithOnly<T>` means only arithmetic nox patterns were applied (no jet invocations).

### Interaction Semantics

Two functions annotated with operation constraints can be proved in parallel when their constraint sets are disjoint:

```trident
fn arithmetic_step(x: Field) -> ArithOnly<Field> { x * x + x }
fn hash_step(x: [Field; 8]) -> JetHeavy<Field> { hemera(x) }

// Parallel proving: arithmetic_step uses only reduction patterns,
// hash_step uses the hemera jet — disjoint nox operation sets.
// Each function gets proved by a separate warrior-cyber instance.
let (a, b) = par_prove(arithmetic_step(x), hash_step(data));
```

The type checker enforces the constraint: if a function is annotated `ArithOnly`, any jet invocation inside its body is a type error. The annotation is a commitment, not just a hint.

### Subset Syntax

For more granular control, operation constraint types use set operations over the nox operation taxonomy (16 patterns + 5 jets):

```trident
// Subset constraint: only arithmetic patterns, no jets
type ArithOnly<T> = T where ops_used(T) ⊆ {pattern::arith}

// Intersection constraint: no RAM jet
type NoRAMJet<T> = T where ops_used(T) ∩ {jet::ram} = ∅

// Equality constraint: exactly the hemera hash jet
type ExactlyHash<T> = T where ops_used(T) = {jet::hash}
```

The compiler tracks operation membership for every TIR expression and checks these constraints statically.

### Parallel Proving Infrastructure

When two segments have disjoint operation constraints, separate warrior-cyber instances can generate their nox subtraces concurrently. The only synchronization point is the final zheng proof composition step — which is cheap compared to subtrace generation.

```trident
// Runtime produces:
// Warrior A: generates arithmetic subtrace (nox patterns only)
// Warrior B: generates hash subtrace (hemera jet invocations)
// Coordinator: combines subtraces, runs zheng SuperSpartan over both
```

For programs with natural parallelism (batch operations, independent sub-proofs), operation-constraint types enable horizontal scaling of proving cost across warrior-cyber instances.

## Vision

A program with table-aware types can be split into independent subtrace segments proved by separate [[warrior-cyber]] instances in parallel. The [[zheng]] proofs compose — SuperSpartan supports subtrace composition via the sumcheck protocol. A program that would take 10 seconds to prove on a single warrior takes 1 second when split across 10 warriors in a [[cybergraph]]-coordinated proving network. This is the foundation for distributed proving.

Today, proving is a serial bottleneck: one warrior, one trace, one proof. Table-constraint types break that bottleneck structurally, at the type level. The compiler encodes parallelism opportunities into the types themselves — the type checker verifies that segments are provably independent, so the proving network doesn't need to coordinate at runtime. Each segment's independence is a compile-time certificate.

At scale, this transforms [[bbg]]'s throughput. A complex state transition that involves arithmetic sub-computations and hemera hash sub-computations can prove both sides simultaneously. The on-chain verification time remains constant — [[zheng]] verifies the composed proof in the same time as any single proof.

## Stack Integration

[[bbg]] orchestrates the proving network. Each [[warrior-cyber]] instance receives its subtrace assignment, generates a partial [[zheng]] proof, and submits it as a cyberlink into [[cybergraph]]: `formula → partial_proof_commitment`. BBG aggregates the partial proofs — the aggregation itself is a state transition with a conservation check that the subtrace boundaries are consistent.

Focus cost is split across participating warriors. A computation that consumes τ total focus budget can be distributed: warrior A receives τ/2, warrior B receives τ/2, each proves its half independently. BBG tracks the total focus expenditure across the distributed proving round, enforcing TSP-2 (each subtrace owned by exactly one warrior at a time).

[[soft3]]'s `query()` interface remains unchanged from the caller's perspective — the distribution is transparent. The `verify()` call checks the composed proof, which is structurally identical to a single-warrior proof. Table-constraint types make parallelism an implementation detail, not an API concern.

## Key Tradeoffs

**Annotation burden**: Requiring explicit operation annotations on every function is verbose. The compiler should infer operation constraints automatically and only require explicit annotation when the developer wants to enforce a contract. The inferred constraint appears in IDE tooltips for discoverability.

**Constraint conservatism**: Inferred constraints are always sound upper bounds (the compiler may infer a larger operation set than actually used). For parallel proving, overly conservative inference reduces opportunities. The developer can tighten the annotation manually.

**Disjointness requirement**: True disjointness is stricter than needed for parallel proving. Pure arithmetic patterns appear in almost all nox evaluation — if both segments share only arithmetic patterns, a more sophisticated model can still partition the trace by segment boundary. This requires the zheng proof system to support subtrace composition, which it does via the SuperSpartan sumcheck.

**Composition with cost types**: Operation-constraint types and proof-cost types ([[proof-cost-types]]) are complementary. A function might be `ArithOnly` (operation constraint) with `#[cost(steps: 100..200)]` (cost bound). The two annotations together give full information about the function's nox proof profile.

## Implementation Sketch

Operation constraint inference runs after TIR construction as a dataflow analysis over nox patterns and jets:

```rust
// typecheck/table_constraints.rs
#[derive(Clone, PartialEq)]
struct OpSet(BitSet<21>);  // 16 nox patterns + 5 jets

impl OpSet {
    fn union(&self, other: &OpSet) -> OpSet { ... }
    fn intersect(&self, other: &OpSet) -> OpSet { ... }
    fn is_subset_of(&self, other: &OpSet) -> bool { ... }
    fn is_disjoint_from(&self, other: &OpSet) -> bool { ... }
}

fn infer_op_set(expr: &TirExpr) -> OpSet {
    match expr {
        Mul(a, b) => OpSet::pattern_arith().union(&infer(a)).union(&infer(b)),
        Hash(input) => OpSet::jet_hash().union(&infer(input)),
        RamRead(addr) => OpSet::jet_ram().union(&infer(addr)),
        // ...
    }
}
```

When the developer declares an operation constraint annotation, the compiler checks that the inferred set is a subset of the declared set. Violation is a compile error with the specific nox operation that leaked.
