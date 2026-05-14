---
status: draft
author: mastercyb
area: compiler
planned: 32K
---

# Field Arithmetic Simplification Passes

## Motivation

No general-purpose compiler has ever exploited the algebraic structure of its execution domain because general-purpose domains (x86, ARM, WASM) have no algebra worth exploiting. Trident operates over the Goldilocks field $p = 2^{64} - 2^{32} + 1$. This prime has extraordinary structure: a golden-ratio identity $2^{64} \equiv 2^{32} - 1 \pmod{p}$, primitive $2^{32}$th roots of unity, and a generator of 7. Each of these properties is a compiler optimization waiting to be implemented. Passes 1–5 target field arithmetic directly — before any polynomial or transform reasoning enters.

## Design

### Pass 1: Fermat Reduction

By Fermat's little theorem, $a^{p-1} \equiv 1$ for $a \neq 0$. Any exponent $k$ in `pow(x, k)` reduces to $k \bmod (p-1)$ without changing the result. The compiler rewrites large-exponent power calls at the IR level before code generation.

```trident
// Source
let y = pow(x, p);          // exponent p reduces mod p-1 → 1
// After pass: y = x        // Frobenius is identity in base field

let z = pow(x, 2*p - 1);   // reduces to pow(x, p) → x*x
// After pass: z = x * x
```

For iterative algorithms that accumulate astronomically large exponents via recursive formulas, this reduces millions of symbolic loop steps to a handful of real multiplications.

### Pass 2: Inversion via Exponentiation

The identity $a^{-1} \equiv a^{p-2} \pmod{p}$ means `invert(x)` and `pow(x, p-2)` are interchangeable. The compiler does not blindly prefer one form. It profiles the current AET table balance and chooses whichever representation yields better table utilization.

The critical Goldilocks-specific optimization: $p - 2 = 2^{64} - 2^{32} - 1$ has known structure. Its optimal addition chain uses approximately 95 multiplications — the compiler hardcodes this chain rather than falling through to generic binary exponentiation, saving ~15 multiplications per inversion call.

### Pass 3: Strength Reduction via Root-of-Unity Shifts

Multiplication by a constant $C$ that is a small power of 2 becomes a bit shift — zero Processor rows for the multiply. The compiler extends this beyond pure powers of 2: for any constant whose Hamming weight $w < 6$, it decomposes multiplication into $w-1$ additions and shifts.

Goldilocks-specific: because $2^{64} \equiv 2^{32} - 1 \pmod{p}$, even shifts of $\geq 64$ bits reduce cheaply via one shift and one subtraction. The compiler maintains a table of Goldilocks roots of unity:

```
ω₆   = 2³²    (multiplication = 32-bit shift)
ω₁₂  = 2¹⁶    (multiplication = 16-bit shift)
ω₁₉₂ = 2       (multiplication = 1-bit shift)
```

When `x * C` appears and $C$ matches a root of unity or a short combination thereof, the multiply is eliminated entirely.

### Pass 4: Batch Inversion (Montgomery's Trick)

Given $k$ field elements to invert, Montgomery's trick computes all inverses using exactly 1 inversion and $3(k-1)$ multiplications — versus $k$ inversions naively. The compiler auto-detects multiple `invert()` calls within the same scope and rewrites them transparently:

```
prefix[0] = a[0]
for i in 1..k: prefix[i] = prefix[i-1] * a[i]
inv_all = invert(prefix[k-1])    // SINGLE inversion
for i in (k-1)..1:
  result[i] = prefix[i-1] * inv_all
  inv_all = inv_all * a[i]
result[0] = inv_all
```

The programmer writes `let y = invert(x)` in multiple places. The compiler sees through scope boundaries, collects the set of inversion calls, and emits batch form. For $k = 10$, the AET savings approach 10×.

### Pass 5: Quadratic Residue Short-Circuit

The Euler criterion states $a^{(p-1)/2} = 1$ iff $a$ is a quadratic residue. When the compiler sees `sqrt(x)` or `has_sqrt(x)`, it checks whether $x$ is a compile-time constant or derivable from constant propagation. If so, it resolves the sqrt feasibility check at compile time — no runtime branch needed.

For conditionals guarding on `has_sqrt(x)` where $x$ is known, the entire branch can be eliminated, replacing a dynamic check with a compile-time constant boolean.

## Key Tradeoffs

**Pass 2 (inversion form)**: Choosing between the `invert` instruction and the exponentiation chain depends on the current bottleneck table. If Processor rows dominate, the explicit `invert` instruction (which uses fewer total rows than 95 squarings) may be preferable. The pass must query the cost model, not apply a fixed rule.

**Pass 4 (batch inversion)**: Batching across scope boundaries requires alias analysis. The compiler must prove that inversion calls are independent (no data dependency between them) before reordering. False positives produce incorrect results; the pass must be conservative.

**Pass 3 (Hamming weight threshold)**: The break-even point where decomposition beats a general multiply depends on the specific TASM multiply cost. The threshold of $w < 6$ is a starting estimate that should be calibrated against actual AET measurements.

## Implementation Sketch

All five passes operate at the TIR (Trident IR) level, before lowering to TASM. They are implemented as algebraic rewrite rules over TIR nodes:

```rust
// Pass 1 — in tir/passes/fermat_reduction.rs
fn reduce_exponent(k: FieldElement) -> FieldElement {
    k % (GOLDILOCKS_P - 1)
}

// Pass 4 — in tir/passes/batch_inversion.rs
fn collect_invert_calls(block: &TirBlock) -> Vec<TirNodeId> {
    block.nodes().filter(|n| n.op == Op::Invert).collect()
}
fn emit_montgomery_batch(calls: Vec<TirNodeId>, block: &mut TirBlock) { ... }
```

Passes are ordered: Fermat reduction first (simplifies exponents before chain optimization), then strength reduction (constants may be revealed by Fermat), then batch inversion (requires all invert calls to be visible), then quadratic residue short-circuit (depends on constant propagation results).
