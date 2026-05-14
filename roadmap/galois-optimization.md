---
status: draft
author: mastercyb
area: math
planned: 16K
---

# Galois Theory for Extension Field Optimization

**Related proposals:** [[algebraic-geometry-constraints]], [[field-arithmetic-passes]], [[compiler-analysis-passes]]
**Reference:** [language.md §15 — XField type, `*.` operator, dot-step builtins](../reference/language.md)

## Motivation

When Trident operates over extension fields ($\mathbb{F}_{p^2}$, $\mathbb{F}_{p^4}$), arithmetic costs multiply. Extension field multiplication in $\mathbb{F}_{p^2}$ requires 3 base-field multiplications (Karatsuba), vs 1 for base field. But the Galois group of the extension carries structure the compiler can exploit — structure that dramatically reduces the effective cost of many extension field operations.

`language.md §15` already defines the `XField` type (the quadratic extension of the base field), the `*.` operator for extension field multiplication, and the dot-step builtins. This proposal adds Galois-theoretic rewrite rules on top of that existing type. The Frobenius automorphism is directly relevant: hemera (Poseidon2) operates internally over the extension field embedding, and Frobenius structure there can be exploited for cheaper round permutations.

No existing compiler applies Galois theory to code generation. Trident can, because it controls the full pipeline from source to field arithmetic.

## Design

### Frobenius Automorphisms Are Free

The Frobenius map $\phi: x \mapsto x^p$ is an automorphism of $\mathbb{F}_{p^2}$ (it permutes elements while preserving the field structure). In the base field $\mathbb{F}_p$, the Frobenius is the identity — $x^p = x$ for all $x \in \mathbb{F}_p$.

In $\mathbb{F}_{p^2}$ with elements $(a_0, a_1)$ expressed using `XField` (`language.md §15`):

$$\phi(a_0, a_1) = (a_0 + a_1 \cdot (p \bmod \text{irred}), \ldots)$$

For many irreducible polynomials, the Frobenius has a simple closed form — conjugation: $\phi(a_0, a_1) = (a_0, -a_1)$. This is a sign flip, not a multiplication. Cost: 1 negation in the base field vs. the Fermat-based approach.

**Compiler optimization**: whenever the program computes `pow(x, p)` for an `XField` element `x`, the compiler replaces it with the Frobenius application (conjugation), saving 2 base-field multiplications per call. This rewrite fires at the TIR level and is compatible with the existing field-arithmetic passes ([[field-arithmetic-passes]]).

The Frobenius is also relevant to hemera's internal structure: Poseidon2 uses the extension field embedding for its MDS matrix, and Frobenius-aware simplification can reduce the number of `*.` operations in the round function.

### Norm and Trace Maps

The norm $N(a) = a \cdot \phi(a) = a_0^2 - \text{irred\_const} \cdot a_1^2$ and trace $T(a) = a + \phi(a) = 2a_0$ are both base-field elements cheaply computable from extension field elements.

When the program computes `norm(x)` or `trace(x)`, the compiler generates the direct formula instead of full extension field multiply. For the norm: 2 squarings + 1 multiplication + 1 subtraction (vs 3 general multiplications).

### Galois Group Structure for Inversion

Inversion in $\mathbb{F}_{p^2}$:

$$a^{-1} = \frac{\phi(a)}{N(a)} = \frac{(a_0, -a_1)}{a_0^2 - c \cdot a_1^2}$$

where $c$ is the constant from the irreducible polynomial $x^2 - c$. This requires:
1. 2 squarings (for $a_0^2$ and $a_1^2$)
2. 1 multiplication ($c \cdot a_1^2$)
3. 1 inversion in the base field ($N(a)^{-1}$)
4. 2 multiplications (scaling by $N(a)^{-1}$)

Total: 3 squarings + 2 multiplications + 1 base-field inversion. Versus the naive approach (exponentiation by $p^2 - 2$), which requires the full Fermat addition chain.

### Subfield Element Detection

Elements of the form $(a_0, 0)$ are in the base field embedded in the extension. Multiplying by a subfield element costs 2 multiplications (not 3). The compiler tracks which values are provably in the base subfield via type propagation and generates the cheaper multiply form.

### Tower Field Optimization

For $\mathbb{F}_{p^4} = \mathbb{F}_{p^2}[y] / (y^2 - \alpha)$, the Galois group has 4 elements. The compiler applies Galois-theoretic shortcuts at each level of the tower: Frobenius at the top level folds into a Frobenius at the bottom level, both of which are just conjugations.

## Vision

[[hemera]] (Poseidon2) uses extension field arithmetic internally — its round function involves XField elements. When Galois-theoretic optimizations apply to hemera's round computation in Trident, every hemera call in every Trident program gets cheaper. Since hemera is the identity function of the entire [[cybergraph]] — every particle address is a hemera output — making hemera cheaper reduces the cost of every knowledge graph operation. The compounding effect: cheaper addresses mean more particles are created, which means a denser graph, which means more memoization, which means fewer computations execute. The graph accelerates itself.

The Frobenius automorphism is not an abstract algebraic curiosity in this context — it is a concrete optimization that propagates through the entire ecosystem. Every `hash()` call in every Trident program, every content-addressed particle in the [[cybergraph]], every [[bbg]] state transition that touches a hemera commitment: all of them benefit when the XField round function gets cheaper. The optimization lives at the TIR level, invisible to the programmer, but its effect is felt everywhere.

## Stack Integration

These rewrites apply at the TIR level, before [[warrior-cyber]] lowers TIR to nox. The goal is explicit: Frobenius rewrites at the TIR level reduce the number of nox reduction steps for XField operations — fewer base-field multiplications in the nox trace means a shorter STARK witness for [[zheng]]. The Frobenius-aware optimization of XField operations interacts with the `*.` operator and dot-step builtins (language.md §15), which are the language-level interface to the field extension used in hemera. [[zheng]] still verifies correctness of the rewritten trace via SuperSpartan — the optimization must be algebraically sound or the constraint check fails. Each rewrite rule should carry a Lean proof, as described in [[categorical-compiler]], confirming it is a natural transformation between compiler functors. The [[algebraic-geometry-constraints]] Gröbner basis analysis applies to the rewritten constraint system afterward, potentially finding additional reductions.

## Key Tradeoffs

**Irreducible polynomial choice**: The specific Frobenius formula depends on the irreducible polynomial defining the extension. The compiler must know which irreducible polynomial is in use and precompute the Frobenius formula at compile time. Different polynomial choices yield different Galois group presentations.

**Proof overhead**: Frobenius optimizations change the nox instruction sequence. The zheng prover still verifies correctness via SuperSpartan over the rewritten trace, but the optimization must be algebraically sound — otherwise the constraint check fails. The compiler should carry a Lean proof of each Galois identity it applies. See [[categorical-compiler]] for the framework in which these rewrites are natural transformations between compiler functors.

## Implementation Path

1. Fix the irreducible polynomial for $\mathbb{F}_{p^2}$ in the field module (matches the `XField` type in `language.md §15`)
2. Derive the Frobenius formula symbolically at compile time from the fixed irreducible polynomial
3. Add TIR-level detection of `pow(x, p)` for `XField` elements and replace with conjugation
4. Implement norm/trace rewrite rules at the TIR level
5. Add subfield element tracking via type annotations (elements provably in the base subfield get the 2-multiplication form via the `*.` operator specialisation)
6. Validate each optimization by zheng-proving the identity over sample inputs — the nox trace with and without the rewrite must produce the same public outputs
7. Integrate with [[compiler-analysis-passes]] Pass 11 (extension field strength reduction) which targets the same `XField` arithmetic
