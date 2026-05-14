---
status: draft
author: mastercyb
area: compiler
planned: 32K
---

# Advanced Compiler Analysis Passes

Related: [[field-arithmetic-passes]], [[polynomial-optimization-passes]], [[proof-cost-types]]

## Motivation

Passes 1–10 handle point-wise arithmetic and polynomial transforms. Passes 11–15 require deeper analysis: understanding extension field structure, tracking constant propagation across function boundaries, searching for optimal exponentiation sequences, eliminating dead operations that have proof-side effects, and normalizing algebraic expressions for CSE. These passes separate a competent field-arithmetic compiler from a world-class one. Combined, they can eliminate 30–50% of the nox execution trace for programs heavy on cryptographic constants.

All passes operate at the TIR level (see [`../reference/ir.md`](../reference/ir.md) for the full 54-op spec). Proof cost is counted in nox reduction steps and jet invocations.

## Design

### Pass 11: Extension Field Strength Reduction

In $\mathbb{F}_{p^2}$ (the quadratic extension; see [`../reference/language.md`](../reference/language.md) §15 for the `ExtField` type), general multiplication requires 3 base-field multiplies via Karatsuba. But specific cases are cheaper:

- **Base-field scalar times extension element**: 2 multiplies (no cross term: $(a_0, 0) \cdot (b_0, b_1) = (a_0 b_0, a_0 b_1)$)
- **Squaring in $\mathbb{F}_{p^2}$**: 2 multiplies using $(a+b)(a-b) = a^2 - b^2$

The compiler tracks the type of each operand. When one operand of an extension multiply is known to be in the base field (type `Field` rather than `ExtField`), it emits the 2-multiply variant. When both operands are the same expression, it emits the squaring circuit.

```trident
let a: Field = ...;           // base-field element
let b: ExtField = ...;        // extension element
let c = b * a;                // 2 multiplies, not 3
let d = b * b;                // squaring: 2 multiplies, not 3
let e = b * some_other_ext;   // general: 3 multiplies
```

The pass runs after type inference so operand types are known.

### Pass 12: Constant Expression Evaluation

Standard constant folding extended to full Goldilocks field arithmetic. The compiler evaluates any expression whose inputs are all compile-time constants, including:

- `3 * 5` → `15`
- `invert(7)` → the field element $7^{-1} \bmod p$ (precomputed)
- `pow(2, 32)` → the actual field value $2^{32} \bmod p$
- `sqrt(4)` → `2` (verified quadratic residue at compile time)

Constant propagation extends through function boundaries: if all arguments to a function are constants, the function is evaluated at compile time and replaced by a single field element. For programs heavy on named cryptographic constants (round constants, generator values, fixed points), this can eliminate 30–50% of the trace before any runtime computation occurs.

```trident
const ALPHA: Field = 7;
const BETA: Field = invert(ALPHA);  // compiler computes 7^{-1} mod p
// BETA is now a literal constant in the nox trace output — zero runtime cost
```

### Pass 13: Addition Chain Optimization for Known Exponents

Binary exponentiation of $x^k$ uses $\lfloor \log_2 k \rfloor$ squarings plus $\text{popcount}(k) - 1$ multiplications. Optimal addition chains often do better.

For $k < 2^{15}$, optimal chains are precomputed and table-looked-up at compile time. For larger $k$, the compiler runs a heuristic search (Brauer chains or windowed method) to find near-optimal chains.

The critical Goldilocks case: $p - 2 = 2^{64} - 2^{32} - 1$ (used for every field inversion via Fermat). Its structure yields a known optimal chain of ~95 multiplications — the compiler hardcodes this rather than discovering it dynamically, saving ~15 multiplications per inversion versus generic binary exponentiation.

```
// Binary method for p-2: ~128 squarings + ~32 multiplies ≈ 160 ops
// Optimal chain for p-2: ~95 ops — exploits 2^64 ≡ 2^32-1 structure
// Savings: ~65 nox reduction steps per inversion call
```

### Pass 14: Dead Field Operation Elimination

Standard dead code elimination with a field-arithmetic wrinkle. In a proof system, a computed value may be "unused" in the program's computational sense but still required for the STARK trace. The compiler must distinguish three cases:

1. **Truly dead**: result unused, no STARK constraint depends on the computation. Eliminate entirely.
2. **Proof-relevant**: result unused in program logic, but a STARK constraint (from a `requires`/`ensures` clause or loop invariant) depends on the value being in the trace. Keep.
3. **Partially dead**: a multi-component value (e.g., extension field element) where only some components are used. Eliminate the unused components, keep the rest.

The pass requires cooperation with the constraint compiler: before eliminating an operation, it queries whether any generated constraint references that operation's TIR node ID.

### Pass 15: Algebraic Common Subexpression Elimination

Standard CSE catches syntactically identical subexpressions. Algebraic CSE catches semantically identical ones:

- **Commutativity**: `a*b + c*d` and `c*d + a*b` are the same — normalize by sorting operands canonically before CSE.
- **Distributivity**: `a*(b+c)` and `a*b + a*c` are the same — choose the factored or distributed form based on nox proof cost impact.

The compiler canonicalizes field expressions into a normal form (operands sorted by hash, additions before multiplications at the same precedence level) before running CSE. This catches equivalences that syntactic CSE misses entirely.

For the distributivity decision:
- Factored form (`a*(b+c)`): fewer multiplications, deeper dependency chain
- Distributed form (`a*b + a*c`): more multiplications, enables parallelism

The compiler chooses based on current nox reduction step pressure. If multiply-heavy patterns dominate, prefer factored. If the current program has sequential dependency chains limiting parallelism, prefer distributed.

```trident
// Normalized representations:
a * b + b * a     → 2 * (a * b)        // commutativity detected
(x + 1) * (x - 1) → x*x - 1          // distributivity → factor removes multiply
a*b + a*c + a*d   → a*(b+c+d)         // 3 multiplies → 1 multiply + 2 adds
```

## Key Tradeoffs

**Pass 11**: The pass requires accurate type-level information. If extension field provenance is lost (e.g., through an opaque function boundary), the compiler must conservatively emit the 3-multiply general case.

**Pass 12**: Constant folding across function calls requires inlining the callee. The compiler inlines only when all arguments are constants and the function is small (body size threshold configurable). For large functions, the overhead of keeping the function in the trace may exceed the constant-folding savings.

**Pass 13**: Addition chain search for large exponents is NP-hard in general. The compiler bounds the search time (e.g., 10ms per exponent) and uses the best chain found within that budget. For the critical $p-2$ case, the chain is hardcoded and the search is skipped.

**Pass 14**: The proof-relevance query creates a dependency between the optimization pass and the constraint compiler. This bidirectional dependency must be managed carefully — ideally, constraint generation runs first to mark relevant nodes, then dead elimination uses the marks.

**Pass 15**: Canonical normalization of algebraic expressions is expensive for large expressions (the normalization itself may touch every subterm). The pass should be applied selectively to hot loops and cryptographic circuits, not to every expression in the program.

## Implementation Sketch

Passes 11–13 are local (per-expression) and straightforward to implement as TIR rewrite rules (see [`../reference/ir.md`](../reference/ir.md)). Passes 14–15 require global analysis:

```rust
// Pass 14 — tir/passes/dead_field_elim.rs
fn is_proof_relevant(node: TirNodeId, constraints: &ConstraintSet) -> bool {
    constraints.references_node(node)
}

// Pass 15 — tir/passes/algebraic_cse.rs
fn canonicalize(expr: &TirExpr) -> NormalForm {
    match expr {
        Add(a, b) => NormalForm::sum(sorted([canonicalize(a), canonicalize(b)])),
        Mul(a, b) => NormalForm::product(sorted([canonicalize(a), canonicalize(b)])),
        // ...
    }
}
```

All 15 algebraic passes together form the `AlgebraicPassSuite`. The suite runs in dependency order, iterating until convergence (at most 3 iterations in practice — passes rarely enable more than two rounds of new simplifications). The nox proof cost model (reduction steps + jet invocations) is the authoritative guide for deciding when to apply vs skip a pass — see [[proof-cost-types]] for the type-level representation of these costs that drives pass decisions.
