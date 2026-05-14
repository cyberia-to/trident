---
status: draft
author: mastercyb
area: compiler
planned: 32K
---

# Field Arithmetic Simplification Passes

Related: [[polynomial-optimization-passes]], [[compiler-analysis-passes]], [[supercompilation]], [[nox]], [[zheng]], [[bbg]], [[Atlas]], [[cybergraph]], [[algebraic-identity-explorer]]

## Stack Integration

These passes sit at the junction of three subsystems. The rule database they implement is deployed as an [[Atlas]] package — `atlas.cyber/trident-passes/arithmetic` — so every compiler instance in the network shares the same verified identity table without shipping it per-binary. When the [[algebraic-identity-explorer]] discovers a new algebraic identity empirically, it updates that package; all subsequent compilations against the updated package automatically apply the new rule, and previously compiled programs can be recompiled retroactively against it.

The cost model driving each pass — which form of inversion to choose, what the Hamming-weight threshold should be — is denominated in [[nox]] reduction steps, because reduction steps are what [[zheng]] must arithmetize and [[bbg]] charges from the focus budget τ. A program that inverts 10 field elements in a tight loop consumes 10× the focus if batch inversion is not applied; with Pass 4 active, the same program costs τ for a single inversion. This directly affects which computations neurons in the [[cybergraph]] can afford to run.

The strata library (field arithmetic implementation) provides the constant tables that Pass 3 consults at compile time. Strata's root-of-unity table is the canonical source; the compiler references it as a content-addressed particle via [[hemera]], so the table's identity is verifiable.

The compiler itself, once self-hosted in Trident and compiled with `--engine nox`, is a [[nox]] program. Every compilation it performs is recorded as a cyberlink `compilation_hash → optimized_artifact` in [[cybergraph]] — permanent, memoized, verifiable.

## Motivation

No general-purpose compiler has ever exploited the algebraic structure of its execution domain because general-purpose domains (x86, ARM, WASM) have no algebra worth exploiting. Trident operates over the Goldilocks field $p = 2^{64} - 2^{32} + 1$, implemented in nebu and strata. This prime has extraordinary structure: a golden-ratio identity $2^{64} \equiv 2^{32} - 1 \pmod{p}$, primitive $2^{32}$th roots of unity, and a generator of 7. Each of these properties is a compiler optimization waiting to be implemented. Passes 1–5 target field arithmetic directly — before any polynomial or transform reasoning enters.

All five passes operate at the TIR level (see [`../reference/ir.md`](../reference/ir.md) for the full 54-op, 4-tier TIR spec). Proof cost in nox/zheng is measured in nox reduction steps and jet invocations — not AET table heights, which are a Triton VM concept.

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

The identity $a^{-1} \equiv a^{p-2} \pmod{p}$ means `invert(x)` and `pow(x, p-2)` are interchangeable. The compiler does not blindly prefer one form. It profiles the current nox proof cost (reduction step count) and chooses whichever representation yields fewer total steps.

The critical Goldilocks-specific optimization: $p - 2 = 2^{64} - 2^{32} - 1$ has known structure. Its optimal addition chain uses approximately 95 multiplications — the compiler hardcodes this chain rather than falling through to generic binary exponentiation, saving ~15 multiplications per inversion call.

### Pass 3: Strength Reduction via Root-of-Unity Shifts

Multiplication by a constant $C$ that is a small power of 2 becomes a bit shift — zero nox reduction steps for the multiply. The compiler extends this beyond pure powers of 2: for any constant whose Hamming weight $w < 6$, it decomposes multiplication into $w-1$ additions and shifts.

Goldilocks-specific: because $2^{64} \equiv 2^{32} - 1 \pmod{p}$, even shifts of $\geq 64$ bits reduce cheaply via one shift and one subtraction. The compiler maintains a table of Goldilocks roots of unity provided by nebu/strata:

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

The programmer writes `let y = invert(x)` in multiple places. The compiler sees through scope boundaries, collects the set of inversion calls, and emits batch form. For $k = 10$, the reduction in nox reduction steps approaches 10×.

### Pass 5: Quadratic Residue Short-Circuit

The Euler criterion states $a^{(p-1)/2} = 1$ iff $a$ is a quadratic residue. When the compiler sees `sqrt(x)` or `has_sqrt(x)`, it checks whether $x$ is a compile-time constant or derivable from constant propagation. If so, it resolves the sqrt feasibility check at compile time — no runtime branch needed.

For conditionals guarding on `has_sqrt(x)` where $x$ is known, the entire branch can be eliminated, replacing a dynamic check with a compile-time constant boolean.

## Key Tradeoffs

**Pass 2 (inversion form)**: Choosing between the `invert` instruction and the exponentiation chain depends on the current nox reduction step bottleneck. The explicit `invert` instruction (a nox jet) may use fewer total steps than 95 squarings unrolled. The pass must query the cost model, not apply a fixed rule.

**Pass 4 (batch inversion)**: Batching across scope boundaries requires alias analysis. The compiler must prove that inversion calls are independent (no data dependency between them) before reordering. False positives produce incorrect results; the pass must be conservative.

**Pass 3 (Hamming weight threshold)**: The break-even point where decomposition beats a general multiply depends on the nox multiply pattern cost (nebu field multiply). The threshold of $w < 6$ is a starting estimate that should be calibrated against actual nox trace measurements.

## Implementation Sketch

All five passes operate at the TIR level (see [`../reference/ir.md`](../reference/ir.md)), before lowering to nox patterns. They are implemented as algebraic rewrite rules over TIR nodes:

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

Passes are ordered: Fermat reduction first (simplifies exponents before chain optimization), then strength reduction (constants may be revealed by Fermat), then batch inversion (requires all invert calls to be visible), then quadratic residue short-circuit (depends on constant propagation results). Supercompilation ([[supercompilation]]) runs before this suite and can expose additional constant structure that these passes then exploit.

## Vision

Far in the future, the algebraic pass suite is a living, planetary resource. Every Trident program compiled anywhere runs against `atlas.cyber/trident-passes/arithmetic`, a package that has accumulated thousands of field identities discovered by the [[algebraic-identity-explorer]] running continuously across the network. No individual compiler developer maintains this table — the network grows it.

When a new identity is added to the [[Atlas]] package, the [[cybergraph]] marks every `compilation_hash → optimized_artifact` cyberlink derived from a previous version as stale. Programs registered with a recompile policy automatically queue against the updated package. The next time a neuron calls `ask(ν, program_cid, formula)`, [[nox]] checks the [[cybergraph]] cache first — if the recompiled artifact is already there, the answer costs zero compute. If not, [[warrior-cyber]] runs the recompilation, [[zheng]] proves it, and the new `compilation_hash → optimized_artifact` link is written permanently.

The result: bringing a program closer to the theoretical minimum proving cost for the Goldilocks field is not a one-time compile-time event. It is an ongoing process, driven by the network, verified by [[zheng]], recorded in [[cybergraph]], and charged from the [[bbg]] focus budget only when genuinely new work occurs. Programs get cheaper over time without their authors doing anything.
