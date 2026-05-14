---
status: draft
author: mastercyb
area: math
planned: 8K
---

# Categorical Semantics for Compiler Correctness

**Related proposals:** [[galois-optimization]], [[algebraic-geometry-constraints]]
**Reference:** [reference/ir.md — TIR (54 ops, 4 tiers)](../reference/ir.md)

## Motivation

Compiler correctness is typically argued via test suites and fuzzing — empirical evidence, not proof. For a language whose entire value proposition is mathematical verifiability, this is a contradiction. Trident's compiler should be correct by construction, where correctness is a theorem about the compiler's structure, not an observation about its outputs.

Category theory provides the framework. Trident types form a category. The compiler is a functor between categories. Functors preserve structure by definition. If we can prove the compiler functor preserves equivalences, compiler correctness follows from algebra.

The compiler pipeline already defines concrete layers: `Trident → KIR → TIR → LIR → nox`. TIR has 54 ops across 4 tiers (`reference/ir.md`). Categorical semantics would prove that each lowering step — in particular the TIR lowering functor — preserves morphism equivalences. That is the theorem this proposal targets.

## Design

### The Trident Category

Objects: Trident types (`Field`, `Bool`, `Vector<N>`, function types, etc.)
Morphisms: Trident functions (pure, with no side effects on the proof trace)
Composition: function composition
Identity: the identity function on each type

Two programs are equivalent (equal as morphisms) if they produce identical outputs for all inputs — the standard extensional equality.

### The nox Category

Objects: nox state types (sequences of field elements on the nox stack)
Morphisms: nox instruction sequences (drawn from nox's 16 patterns + 1 hint + 5 jets)
Composition: sequential reduction
Identity: the empty reduction sequence

Two nox sequences are equivalent if they produce identical state for all initial states.

### The TIR Category (Intermediate Layer)

The TIR with its 54 ops and 4 tiers (`reference/ir.md`) forms an intermediate category. The full functor chain is:

$$\mathbf{Trident} \xrightarrow{\mathcal{C}_1} \mathbf{TIR} \xrightarrow{\mathcal{C}_2} \mathbf{nox}$$

Each step must be a functor. $\mathcal{C}_1$ (the TIR lowering functor) is where most of the interesting algebraic content lives: it maps Trident's type-level operations to TIR ops, and categorical semantics would prove this mapping preserves all program equivalences. $\mathcal{C}_2$ (the nox codegen functor) translates the 54 TIR ops to nox instruction sequences.

### The Compiler Functor

The composed compiler $\mathcal{C} = \mathcal{C}_2 \circ \mathcal{C}_1$ is a functor $\mathcal{C} : \mathbf{Trident} \to \mathbf{nox}$:

- Maps each Trident type to its nox stack representation
- Maps each Trident function to a nox instruction sequence

A functor must preserve composition: $\mathcal{C}(f \circ g) = \mathcal{C}(f) \circ \mathcal{C}(g)$.

**This is the compiler correctness property.** Compiling a composed function must equal composing the compiled functions. If this holds, equivalent Trident programs compile to equivalent nox instruction sequences — the compiler cannot introduce behavioural differences. See [[galois-optimization]] for how Galois-theoretic rewrites in the TIR must themselves be natural transformations to remain sound.

### Natural Transformations as Compiler Optimizations

An optimization pass is a natural transformation $\eta : \mathcal{C} \Rightarrow \mathcal{C}'$ between two compiler implementations. Naturality means the optimization commutes with program composition — optimizing a composed program gives the same result as optimizing each component and composing the results.

This is a precise statement of what it means for an optimization to be correct: it cannot break modularity.

### Adjunctions and Proof Obligations

Decompilation (nox → TIR) is a functor in the opposite direction. A correct compiler/decompiler pair forms an adjunction — the unit $\eta : Id_{\mathbf{Trident}} \Rightarrow \mathcal{D} \circ \mathcal{C}$ and counit $\epsilon : \mathcal{C} \circ \mathcal{D} \Rightarrow Id_{\mathbf{nox}}$ express that compilation and decompilation are inverse up to coherent isomorphism.

## Vision

When the categorical compiler proof is complete, the Trident compiler is the first compiler in history whose correctness is a theorem rather than a test suite result. The proof lives in the [[cybergraph]] as a particle — the [[hemera]] particle of the Lean proof file. Any auditor anywhere can verify the proof by fetching the particle and checking it. The compiler's correctness is not maintained by a team — it is maintained by mathematics, permanently, immutably, in the global knowledge graph.

The functor theorem applies transitively. Every [[Atlas]] package compiled with Trident benefits automatically: its compilation is correct because the compiler is proved correct. The audit effort for any deployed program collapses to verifying the program's own contracts — not the compiler's behavior, not the lowering passes, not the [[nox]] codegen. Those are all covered by the functor proof. An auditor who trusts the Lean proof needs to read only the source file.

This is the end-state of [[CORE]] as self-verifying substrate: the 16 [[nox]] patterns are provable, the compiler that targets them is proved correct, and every program in the ecosystem inherits both proofs for free.

## Stack Integration

The compiler functor theorem spans the full pipeline from Trident types to [[nox]] instruction sequences (the 16 patterns + 1 hint + 5 jets). Each [[galois-optimization]] rewrite that fires at the TIR level must be a natural transformation between compiler functors — the categorical framework is the criterion for whether an optimization is sound. [[zheng]] verifies the output of each compilation via SuperSpartan over the [[nox]] trace; the functor proof guarantees the trace is the correct trace. These two verification layers — categorical proof of compiler structure, and proof-system verification of program execution — are complementary and non-redundant.

## Key Tradeoffs

**Proof granularity**: Proving the full functor law requires formalizing the entire Trident type system and nox semantics in a proof assistant (Lean, Coq). This is a multi-year project. A practical intermediate: prove the functor law holds for each individual IR lowering pass (there are 54 TIR ops to cover — `reference/ir.md`), then compose the partial proofs.

**Extension to effects**: Pure Trident functions are morphisms. Effectful programs (those that touch the proof trace non-trivially) require enriched categories or indexed categories. The categorical framework scales, but the formalism becomes heavier.

## Implementation Path

1. Formalize Trident's core type system in Lean (types + function types, no dependent types first)
2. Formalize nox semantics (stack machine, 16 patterns + 1 hint + 5 jets, instruction denotations)
3. Formalize TIR semantics (54 ops, 4 tiers — `reference/ir.md` is the specification)
4. Prove $\mathcal{C}_1$ (TIR lowering) preserves morphism equivalence for each of the 54 TIR ops
5. Prove $\mathcal{C}_2$ (nox codegen) preserves morphism equivalence
6. Compose pass-level proofs into a global functor theorem for $\mathcal{C} = \mathcal{C}_2 \circ \mathcal{C}_1$
7. Machine-check the proof as part of the build system (`lean --check compiler_correctness.lean`)

See [[algebraic-geometry-constraints]] for how the constraint system that zheng verifies relates to the nox semantics formalized in step 2.
