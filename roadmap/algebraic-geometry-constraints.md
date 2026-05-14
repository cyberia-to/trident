---
status: draft
author: mastercyb
area: math
planned: 8K
---

# Algebraic Geometry for Constraint Minimization

**Related proposals:** [[categorical-compiler]], [[galois-optimization]], [[field-arithmetic-passes]]

## Motivation

zheng proves nox executions using SuperSpartan IOP: it converts the nox trace into a system of multilinear polynomial constraints, then proves that system via sumcheck over Brakedown PCS. The constraint system that SuperSpartan generates from the nox trace is the algebraic variety this proposal targets — the set of valid nox execution traces forms a zero locus of polynomials over the Goldilocks field.

Most proof systems generate constraints mechanically, one per instruction, without asking whether the resulting system is minimal. Algebraic geometry provides the tools to find the minimal constraint set defining the same variety — fewer constraints, cheaper sumcheck, identical security.

## Design

### Constraints as a Polynomial Ideal

SuperSpartan converts the nox trace into a constraint system: each step in the trace contributes polynomial equations over the trace witness variables. Each constraint is a polynomial $f_i(x_1, \ldots, x_n) = 0$ over the Goldilocks field. The set of all constraints generates a polynomial ideal $I = \langle f_1, \ldots, f_m \rangle$. The variety $V(I)$ is the set of trace assignments that satisfy all constraints simultaneously — equivalently, the set of valid nox executions.

Two constraint systems $I$ and $I'$ are equivalent if $V(I) = V(I')$. Many different constraint systems define the same variety — the compiler's goal is to find the one with the smallest generating set and lowest degree. Fewer and lower-degree constraints mean fewer sumcheck rounds and smaller Brakedown commitments in zheng.

### Redundant Constraint Detection

A constraint $f_j$ is redundant if $f_j \in I' = \langle f_1, \ldots, f_{j-1}, f_{j+1}, \ldots, f_m \rangle$. This is a membership test in a polynomial ideal, decidable via Gröbner basis computation.

**Compiler optimization**: after SuperSpartan generates the initial constraint set from the nox trace, compute a reduced Gröbner basis. Redundant generators that appear in the basis with leading coefficient 1 against other basis elements can be dropped. Each dropped constraint reduces the degree of the constraint polynomial, reducing the number of sumcheck rounds zheng must execute during proving.

### Minimal Generating Sets via Hilbert Basis

Hilbert's basis theorem guarantees that every polynomial ideal has a finite generating set. The minimal such set (in terms of number of generators and their degrees) corresponds to the minimal SuperSpartan constraint system for that nox computation.

For common patterns in Trident programs — arithmetic progressions, hemera hash round functions, loop iteration — the minimal generating sets can be precomputed and stored as templates. The compiler pattern-matches against templates rather than recomputing from scratch. These templates are analogous to the precomputed cost tables for nox patterns but operate at the constraint level rather than the step count level.

### Singular Point Detection

A point on the variety is singular if the Jacobian matrix of the constraint polynomials drops rank there. Singular points in the SuperSpartan constraint variety correspond to nox trace configurations where the proof system's guarantees weaken — the prover might accept an invalid trace that happens to satisfy all constraints at a singular point.

**Compiler check**: evaluate the Jacobian at the zero trace (all variables zero) and at the boundary conditions. If singular points exist within the valid trace domain, the constraint system needs strengthening — add constraints that resolve the singularity.

This check runs at compile time. A program that would produce a zheng proof with singular-point vulnerabilities fails compilation with a diagnostic pointing to the problematic constraint cluster.

### Dimension Theory for Proof Size

The Krull dimension of $V(I)$ determines the effective degrees of freedom in the nox execution trace — the number of "free" variables zheng must commit to in the Brakedown commitment. Lower dimension → smaller commitment → smaller proof.

The compiler can sometimes reduce dimension by identifying algebraic dependencies between witness columns in the SuperSpartan constraint matrix and encoding those dependencies as additional constraints that eliminate redundant columns. Fewer columns means a narrower Brakedown commitment, which reduces both commitment size and sumcheck witness complexity.

## Vision

The minimal constraint system for a Trident program is a scientific result — it characterizes the algebraic structure of that computation. When minimal constraint systems for common patterns (hemera rounds, [[bbg]] state transitions, token transfers) are stored as [[Atlas]] packages, the entire Trident ecosystem benefits retroactively. Any program whose constraint structure matches a known template gets the minimal system automatically. [[zheng]] proofs shrink. Focus costs drop. The [[cybergraph]] accumulates this knowledge permanently.

The mechanism is self-reinforcing: as more programs are proved and their constraint structures are analyzed, more templates are deposited in [[Atlas]]. Each new template reduces the cost of all future programs that share that structure. Common patterns — the hemera round function appears in every program that uses hashing — become progressively cheaper to prove as the template library grows. The ecosystem's proving cost decreases as a function of accumulated knowledge, not just hardware improvement.

Singular point detection has a security dimension beyond performance: a constraint system with singular points inside the valid trace domain has proof-system vulnerabilities at those points. Every Trident program compiled through this pass is audited for algebraic soundness at compile time, before it reaches [[Atlas]] or [[bbg]]. The constraint system is not just an optimization target — it is a security surface.

## Stack Integration

SuperSpartan in [[zheng]] generates the polynomial constraint system from the [[nox]] trace. This proposal's Gröbner basis analysis applies to that constraint system directly — it is a post-processing step after trace generation, before Brakedown commitment. The output is a reduced constraint set that [[zheng]]'s sumcheck processes more efficiently. The template library lives in [[Atlas]] as content-addressed packages, identified by [[hemera]] CIDs. The Krull dimension analysis feeds directly into [[zheng]]'s Brakedown witness column count — fewer columns means a narrower commitment, smaller proofs, and faster verification. [[galois-optimization]]'s Frobenius rewrites interact with this analysis: XField-heavy programs whose constraint systems include extension field structure benefit from both passes in sequence.

## Key Tradeoffs

**Gröbner basis computation cost**: Computing a Gröbner basis is worst-case doubly exponential in the number of variables. For large programs with many witness columns in the SuperSpartan constraint matrix, this becomes infeasible at compile time. The compiler must apply this analysis selectively — to hot constraint clusters (those that dominate the sumcheck cost) rather than the entire constraint system. The trace-length bottleneck identified by [[proof-explorer]] or [[trace-predictor]] points the compiler at the right clusters.

**Field-specific difficulties**: Gröbner basis theory is cleanest over algebraically closed fields. The Goldilocks field $\mathbb{F}_p$ is not algebraically closed — the variety over $\mathbb{F}_p$ may behave differently from the variety over $\overline{\mathbb{F}_p}$. The compiler must use algorithms adapted for prime fields (e.g., F4 or F5 variants with field-aware term ordering).

## Implementation Path

1. Implement polynomial ideal membership testing over Goldilocks using F4 algorithm
2. Add Gröbner basis computation for small SuperSpartan constraint clusters (≤10 variables)
3. Build a template library of minimal constraint systems for common Trident patterns (hemera round function, loop iteration, field inversions)
4. Add Jacobian singularity check as a compiler pass, triggered by `trident audit --audit`
5. Integrate dimension analysis into the SuperSpartan witness column elimination pass — fewer columns → smaller Brakedown commitment → faster zheng

See [[galois-optimization]] for how Frobenius structure in the extension field further reduces the constraint system for programs heavy on `XField` arithmetic.
