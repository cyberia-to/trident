---
status: draft
author: mastercyb
area: type system
planned: 32K
---

# Table-Constraint Types for Parallel Proving

## Motivation

Triton VM's AET has multiple independent tables: Processor, Hash, Cascade, Lookup, RAM, U32, and others. A STARK proof over the full AET commits to the maximum height across all tables. If one table dominates, the others are padding — wasted proof capacity.

The insight: two program segments that touch disjoint table sets produce proofs that could in principle be generated in parallel, then composed. If the compiler knows at the type level which tables a function touches, it can schedule independently constrained segments for parallel proving and eliminate inter-segment synchronization where it isn't needed.

Table-constraint types encode table membership directly in the type system.

## Design

### Type Definitions

```trident
type HashFree<T> = T where tables_touched(T) ∩ {Hash, Cascade, Lookup} = ∅
type ArithOnly<T> = T where tables_touched(T) ⊆ {Processor, OpStack}
type RAMOnly<T>   = T where tables_touched(T) ⊆ {RAM, Processor}
```

These are refinements of function types. `HashFree<T>` means the value of type `T` was computed without touching any hash-related table. `ArithOnly<T>` means only arithmetic (Processor and OpStack) was used.

### Interaction Semantics

Two functions annotated with table constraints can be proven in parallel when their constraint sets are disjoint:

```trident
fn arithmetic_step(x: Field) -> ArithOnly<Field> { x * x + x }
fn hash_step(x: Field) -> HashFree<Field>  // impossible — hash IS hash table
fn hash_step(x: [Field; 8]) -> Hash<Field> { poseidon(x) }

// Parallel proving: arithmetic_step and hash_step touch disjoint tables
// Their AET rows are in separate tables — no contention, no synchronization
let (a, b) = par_prove(arithmetic_step(x), hash_step(data));
```

The type checker enforces the constraint: if a function is annotated `ArithOnly`, any call to a hash function inside its body is a type error. The annotation is a commitment, not just a hint.

### Subset Syntax

For more granular control, table constraint types use set operations:

```trident
type TableSet = {Processor, Hash, RAM, U32, Cascade, Lookup, OpStack}

// Subset constraint:
type ProcessorAndRAM<T> = T where tables_touched(T) ⊆ {Processor, RAM}

// Intersection constraint:
type NoRAM<T> = T where tables_touched(T) ∩ {RAM} = ∅

// Equality constraint:
type ExactlyHash<T> = T where tables_touched(T) = {Hash, Cascade, Lookup}
```

The compiler tracks table membership for every TIR expression and checks these constraints statically.

### Parallel Proving Infrastructure

When two segments have disjoint table constraints, the runtime can generate their AET rows concurrently on separate threads. The only synchronization point is the final proof composition step — which is cheap compared to trace generation.

```trident
// Runtime produces:
// Thread A: generates Processor + OpStack rows for arithmetic segment
// Thread B: generates Hash + Cascade rows for hash segment
// Main: combines and generates STARK proof over both
```

For programs with natural parallelism (batch operations, independent sub-proofs), table-aware types enable horizontal scaling of proving cost.

## Key Tradeoffs

**Annotation burden**: Requiring explicit table annotations on every function is verbose. The compiler should infer table constraints automatically and only require explicit annotation when the developer wants to enforce a contract. The inferred constraint appears in IDE tooltips for discoverability.

**Constraint conservatism**: Inferred constraints are always sound upper bounds (the compiler may infer a larger table set than actually used). For parallel proving, overly conservative inference reduces opportunities. The developer can tighten the annotation manually.

**Disjointness requirement**: True disjointness is stricter than needed for parallel proving. The Processor table is touched by almost every operation — if both segments touch Processor, they cannot be proven in parallel by this naive model. A more sophisticated model partitions Processor rows by segment, which requires the proof system to support this partitioning.

**Composition with cost types**: Table-constraint types and proof-cost types are complementary. A function might be `ArithOnly` (table constraint) with `cost [processor: 100..200]` (cost bound). The two annotations together give full information about the function's proof profile.

## Implementation Sketch

Table constraint inference runs after TIR construction as a dataflow analysis:

```rust
// typecheck/table_constraints.rs
#[derive(Clone, PartialEq)]
struct TableSet(BitSet<8>);  // one bit per AET table

impl TableSet {
    fn union(&self, other: &TableSet) -> TableSet { ... }
    fn intersect(&self, other: &TableSet) -> TableSet { ... }
    fn is_subset_of(&self, other: &TableSet) -> bool { ... }
    fn is_disjoint_from(&self, other: &TableSet) -> bool { ... }
}

fn infer_table_set(expr: &TirExpr) -> TableSet {
    match expr {
        Mul(a, b) => TableSet::processor().union(&infer(a)).union(&infer(b)),
        Hash(input) => TableSet::hash().union(&infer(input)),
        RamRead(addr) => TableSet::ram().union(&infer(addr)),
        // ...
    }
}
```

When the developer declares a table constraint annotation, the compiler checks that the inferred set is a subset of the declared set. Violation is a compile error with the specific table that leaked.
