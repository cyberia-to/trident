---
status: draft
author: mastercyb
area: compiler
planned: 32K
---

# Polynomial and Transform Optimization Passes

Related: [[field-arithmetic-passes]], [[supercompilation]]

## Motivation

Field arithmetic is point-wise computation. Polynomial arithmetic is structured computation over sequences. The Goldilocks field supports NTTs of size up to $2^{32}$ with butterfly operations that reduce to bit shifts. Passes 6–10 exploit this polynomial layer — turning $O(n^2)$ convolutions into $O(n \log n)$ NTTs, fusing multi-exponentiations, and caching Lagrange basis polynomials across evaluations. Programs that touch any polynomial arithmetic — which includes the proof infrastructure itself — benefit immediately.

These passes operate at the TIR level (see [`../reference/ir.md`](../reference/ir.md)). Proof cost is counted in nox reduction steps and jet invocations — not AET table heights.

## Design

### Pass 6: Schwartz-Zippel Probabilistic Equivalence

Two polynomials of degree $d$ over a field of size $p$ agree on a random point with probability $\leq d/p$. For Goldilocks with $p \approx 2^{64}$, any polynomial of degree up to $2^{32}$ has false-positive probability below $2^{-32}$.

The compiler uses this for speculative simplification: when the optimizer proposes a complex field expression rewrite, it validates by evaluating both forms at a random point. Agreement with probability $> 1 - 2^{-32}$ is sufficient confidence for all practical purposes. This gives the compiler a probabilistic equivalence oracle that costs one field evaluation — replacing symbolic proof search.

```trident
// Optimizer proposes: a*(b+c) == a*b + a*c
// Validate: eval both at random r, check agreement
// Cost: 2 field operations. Confidence: 1 - 2^-32.
```

### Pass 7: NTT Auto-Vectorization

The compiler identifies convolution loops:

```trident
// Detected pattern:
for i in 0..n {
  for j in 0..n {
    result[i+j] += a[i] * b[j];
  }
}
// Rewritten to:
let result = ntt_convolve(a, b);  // O(n log n) NTT-based
```

Detection uses a structural pattern match on the nested-loop AST: inner body must be `result[i+j] += a[i] * b[j]` or equivalent. Once detected, the compiler substitutes NTT-based convolution.

The Goldilocks advantage is decisive here. NTT butterfly operations require multiplication by roots of unity. In Goldilocks, those roots are powers of 2 — so each butterfly is a shift plus add rather than multiply plus add. For $n = 256$: $65{,}536$ multiplications become 2048 shift-add operations. This is a 32× reduction in nox reduction step count for the multiply-intensive inner loop. When the NTT is large enough to warrant it, the compiler may emit a call to nox's `ntt` jet (a Layer 3 jet), which executes the entire transform in a single verifiable step.

### Pass 8: Multi-Exponentiation Fusion

When multiple `pow()` results are multiplied together, Shamir's trick computes them jointly:

```trident
// Source:
let r = pow(a, x) * pow(b, y);
// Naive: two addition chains, ~128 squarings + 64 multiplies each
// Fused (Shamir): ~64 squarings + 48 multiplies total
```

The compiler detects `pow(...)` nodes whose results flow into multiply nodes and triggers fusion. For $k > 4$ simultaneous exponentiations, it applies Pippenger's algorithm which reduces cost from $O(k \cdot \log p)$ to $O(k \cdot \log p / \log k)$.

The detection runs on the TIR dataflow graph: find `mul` nodes where both operands are `pow` nodes with no intervening dependencies. Extract the base-exponent pairs and emit a fused multi-exponentiation intrinsic.

### Pass 9: Vanishing Polynomial Optimization

For a coset of a multiplicative subgroup $D = g \cdot H$ where $|H| = n$, the vanishing polynomial $Z_D(x) = x^n - g^n$ — computable in $O(\log n)$ via repeated squaring rather than the naive $O(n)$ product. This appears in any code that touches polynomial commitment infrastructure. In the zheng proof system, commitments use Brakedown PCS with sumcheck — not FRI. Vanishing polynomial evaluations appear in sumcheck rounds, where this optimization directly reduces the prover's per-round cost.

The compiler recognizes evaluations of the form $\prod_{d \in D}(x - d)$ where $D$ has subgroup-coset structure and replaces with the fast form. The detection requires the compiler to track which domains are cosets of known multiplicative subgroups — maintained in a domain registry populated during compilation.

### Pass 10: Lagrange Basis Caching

Lagrange interpolation over power-of-2 domains runs in $O(n \log n)$ via NTT rather than $O(n^2)$ naively. Beyond asymptotic complexity, the Lagrange basis polynomials for a fixed domain can be precomputed once and reused across all interpolations over that domain.

The compiler detects polynomial interpolation calls over the same domain, extracts the first call's basis computation, and replaces subsequent calls with a cached lookup:

```trident
// First interpolation over domain D:
let p1 = interpolate(values_1, D);  // computes NTT basis, caches it
// Second interpolation over same D:
let p2 = interpolate(values_2, D);  // reuses cached basis
// Compiler inserts: if basis_cache[D] is valid, skip NTT setup
```

Cache invalidation is compile-time: if the domain $D$ is a compile-time constant (common in proof infrastructure code), the basis is computed once and embedded as constants in the nox trace output. This is particularly effective for Brakedown PCS — the Lagrange basis over the commitment domain is fixed for a given polynomial degree, so it can be cached globally across all proof computations at that degree.

## Key Tradeoffs

**Pass 7 (NTT auto-vectorization)**: For small $n$ (say $n < 8$), the $O(n^2)$ naive convolution may be cheaper due to NTT setup overhead. The compiler must measure the crossover point in nox reduction steps, not the asymptotic formula. A safe threshold is $n \geq 16$. For large $n$, prefer the nox `ntt` jet over an unrolled butterfly tree — the jet reduces the entire transform to a single verifiable step.

**Pass 8 (multi-exponentiation)**: Shamir's trick and Pippenger require the exponents to be known or at least computable before the squaring loop begins. When exponents are runtime values, the fusion still applies (the joint loop is correct regardless), but the compiler must schedule the precomputation of lookup tables correctly.

**Pass 9 (vanishing polynomial)**: The fast form $x^n - g^n$ requires that the domain is genuinely a coset. If the domain is an arbitrary set of points (not a coset), the fast form does not apply. The compiler must validate coset structure before applying the optimization — incorrect application produces wrong polynomial values.

**Pass 10 (Lagrange caching)**: Cache validity requires the domain to be identical across calls. If the domain is parameterized at runtime, the compiler falls back to per-call computation. The optimization is most powerful for compile-time-fixed domains, which covers STARK proof infrastructure code entirely.

## Implementation Sketch

Pass 7 integrates with the loop analysis infrastructure that already exists for bounds checking. Pass 8 requires a new dataflow pattern matching phase after TIR construction. Passes 9–10 require a domain registry:

```rust
// tir/passes/lagrange_cache.rs
struct DomainRegistry {
    cached: HashMap<Domain, BasisPolynomials>,
}

fn optimize_interpolation(call: &TirCall, registry: &mut DomainRegistry) {
    if let Some(domain) = call.static_domain() {
        if registry.cached.contains_key(&domain) {
            // Replace call with cached-basis variant
        } else {
            // Compute basis, cache it, mark call as cache-populating
            registry.cached.insert(domain, compute_basis(&domain));
        }
    }
}
```

All five passes share the invariant that they never change program semantics — only nox proof cost (reduction steps + jet invocations). Each pass should be individually togglable for debugging cost contributions. See [[field-arithmetic-passes]] for the point-wise passes that run before this suite, and [[supercompilation]] for the global pass that runs before all of them.
