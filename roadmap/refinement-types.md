---
status: draft
author: mastercyb
area: type system
planned: 32K
---

# Refinement Types over Field Arithmetic

**Related proposals:** [[contracts]], [[linear-types-crypto]], [[dependent-types]]
**Reference:** [language.md §7 — Attributes (#[requires], #[ensures])](../reference/language.md)

## Motivation

Many field-arithmetic bugs are value-range bugs: division by zero when a denominator is expected nonzero, probability values that overflow their intended range, signed values that cross the field's midpoint. Conventional type systems cannot express these constraints — `Field` and `Field` are the same type regardless of value. Refinement types add predicates to types, making the constraint part of the type itself.

In Trident, refinement types have a unique property: they compile to nox constraints checked by zheng. Proving that the program's execution satisfies the predicate is part of the STARK proof itself — specifically, zheng's SuperSpartan sumcheck over the nox execution trace. There is no separate verification step, no SMT solver at runtime, no assertion that can be disabled. The proof of execution IS the proof that all refinements were satisfied.

## Design

### Refined Types

```trident
type Positive     = { x: Field | x > 0 && x < p/2 };
type Probability  = { x: Field | x >= 0 && x <= SCALE_FACTOR };
type NonZero      = { x: Field | x != 0 };
type InRange<A, B> = { x: Field | x >= A && x <= B };
```

The predicate is a boolean expression over the value `x`. Any expression valid in Trident can be used as a refinement predicate, including field arithmetic, function calls, and comparisons.

### Usage

```trident
fn safe_divide(a: Field, b: NonZero) -> Field {
    a * invert(b)  // invert is safe — b cannot be zero by type
}

fn probability_add(p: Probability, q: Probability) -> Probability {
    // Compiler checks: p + q could exceed SCALE_FACTOR
    // ERROR: result not proven to be Probability without additional constraint
    p + q  // type error unless we can prove the sum stays in range
}

fn clamped_add(p: Probability, q: Probability) -> Probability {
    let sum = p + q;
    if sum > SCALE_FACTOR { SCALE_FACTOR } else { sum }  // now provably Probability
}
```

### Compilation to nox Constraints

When a `NonZero` value is used, the compiler generates no additional nox reduction steps for the check — the constraint is synthesized at the zheng constraint-generation level, not in the nox reduction sequence. Checking $b \neq 0$ for a `NonZero` argument translates to a constraint polynomial that evaluates to zero only when $b$ is zero — and a valid zheng proof means this constraint is never triggered.

More precisely: the zheng verifier, upon receiving the STARK proof, checks that all constraint polynomials are satisfied over the nox execution trace. A violated refinement means a constraint polynomial that should be zero is nonzero — which means the proof is invalid. Invalid proofs are rejected. The refinement is enforced by the proof system itself, not by the program's reduction logic.

```trident
// Source:
fn f(x: NonZero) -> Field { invert(x) }

// Generated constraint (not a nox pattern — a zheng constraint polynomial):
// For every nox trace row where f is evaluated:
//   x_argument ≠ 0   →   (x_argument - 0) has a multiplicative inverse
//   Encoded as:   x_argument * x_argument_inv - 1 = 0
```

### Predicate Subsumption

The type checker can discharge refinement checks statically when it can prove the predicate holds from the types of the arguments. If a function argument is typed `Positive` and the predicate `Positive ⊆ NonZero` holds (which it does — positive values are nonzero), then passing a `Positive` value where `NonZero` is expected requires no additional constraint.

The compiler maintains a subtype lattice over refinements and uses it to avoid generating constraints that are already implied by the argument types.

## Vision

[[CORE]]'s conservation laws are refinement type predicates. `TSP-1: sum(balances) = supply` — written as `#[ensures(sum(balances) == supply)]` on the transfer function — compiles to a [[nox]] constraint that [[zheng]] verifies on every execution. The conservation law doesn't just hold by convention or by test suite; it holds by math, proven fresh for every transaction.

This is what "trust the math, not the institution" means in practice. No auditor needs to check that the transfer function maintains supply conservation — the [[zheng]] proof does it, automatically, for every execution, forever. A protocol that encodes TSP-1 as a refinement type cannot have a supply inflation bug. The constraint is mathematically impossible to violate while producing a valid proof.

The same principle extends across [[bbg]]'s full state machine. Every state predicate that matters — balance bounds, ownership uniqueness (TSP-2), focus budget constraints — can be expressed as a refinement type predicate and compiled to a [[zheng]] constraint. The network's invariants become the language's type invariants.

## Stack Integration

[[bbg]] state transitions enforce conservation laws structurally. When transfer functions carry refinement-type conservation constraints, [[bbg]] can reject any state update whose proof doesn't include the conservation check. No governance vote needed — the math blocks bad state. A malformed state transition that violates `sum(balances) == supply` produces an invalid [[zheng]] proof; the proof is rejected; the state transition is rolled back. The network is self-enforcing.

[[CORE]]'s 16 reduction patterns, once written in Trident with refinement types on their inputs and outputs, carry machine-verified correctness properties. The pattern `wut` (pattern 2: `*[a b] → *[a b]`) carries a refinement that the output is always a well-formed noun — verified by [[zheng]] on every application.

[[soft3]]'s `verify()` call checks the [[zheng]] proof, which includes all refinement constraint verifications. A caller who receives a verified proof from `soft3.verify()` knows that all annotated preconditions and postconditions held during that execution — including any conservation laws declared on the called functions.

## Key Tradeoffs

**Predicate expressibility**: Refinements limited to efficiently-checkable predicates (no arbitrary recursion in the predicate). For predicates that require full proof machinery to verify (e.g., "x is a prime"), the refinement itself is not a compile-time type check but a STARK constraint generated at runtime — still valuable, but not statically eliminated.

**Constraint cost**: Each non-discharged refinement generates additional zheng constraints. For high-frequency operations, this adds to the nox trace size and hence proof cost. The compiler should report refinement constraint cost in the proof cost breakdown (see [[proof-cost-types]]) so developers can identify expensive predicates.

**SMT solver integration**: For static discharge of refinements, an SMT solver (Z3 or CVC5) could verify predicate implication at compile time. This is expensive for complex predicates. The compiler starts with a simple syntactic subsumption check and falls back to constraint generation when that fails.

**Interaction with constant folding**: If a refined value is a compile-time constant, the refinement predicate can be evaluated at compile time. No constraint generated — zero runtime cost. For programs heavy on constant values, most refinements disappear entirely.

## Implementation Sketch

Refinement types integrate with the type checker and constraint compiler:

```rust
// typecheck/refinement.rs
struct RefinedType {
    base: Type,
    predicate: Predicate,
}

enum Predicate {
    Gt(FieldExpr, FieldExpr),
    Lt(FieldExpr, FieldExpr),
    Eq(FieldExpr, FieldExpr),
    Ne(FieldExpr, FieldExpr),
    And(Box<Predicate>, Box<Predicate>),
    // ...
}

fn check_subtype(sub: &RefinedType, sup: &RefinedType) -> SubtypeResult {
    // Try syntactic inclusion first
    // Fall back to constraint generation
}

// cost/constraints.rs — generates zheng constraints for non-discharged refinements
fn generate_refinement_constraint(pred: &Predicate, row_id: NoxTraceRowId) -> ZhengConstraint {
    // Translates predicate to a SuperSpartan constraint polynomial over the nox trace
}
```

The refinement system is designed to fail loudly and early: if a predicate cannot be discharged statically and generating a STARK constraint for it would be too expensive, the compiler reports this explicitly rather than silently generating expensive proof overhead.
