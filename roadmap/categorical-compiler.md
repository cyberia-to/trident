---
status: draft
author: mastercyb
area: math
planned: 8K
---

# Categorical Semantics for Compiler Correctness

## Motivation

Compiler correctness is typically argued via test suites and fuzzing — empirical evidence, not proof. For a language whose entire value proposition is mathematical verifiability, this is a contradiction. Trident's compiler should be correct by construction, where correctness is a theorem about the compiler's structure, not an observation about its outputs.

Category theory provides the framework. Trident types form a category. The compiler is a functor between categories. Functors preserve structure by definition. If we can prove the compiler functor preserves equivalences, compiler correctness follows from algebra.

## Design

### The Trident Category

Objects: Trident types (`Field`, `Bool`, `Vector<N>`, function types, etc.)
Morphisms: Trident functions (pure, with no side effects on the proof trace)
Composition: function composition
Identity: the identity function on each type

Two programs are equivalent (equal as morphisms) if they produce identical outputs for all inputs — the standard extensional equality.

### The TASM Category

Objects: stack types (sequences of field elements)
Morphisms: TASM instruction sequences
Composition: sequential execution
Identity: the empty instruction sequence

Two TASM sequences are equivalent if they produce identical stack states for all initial states.

### The Compiler Functor

The compiler $\mathcal{C}$ is a functor $\mathcal{C} : \mathbf{Trident} \to \mathbf{TASM}$:

- Maps each Trident type to its stack representation
- Maps each Trident function to a TASM instruction sequence

A functor must preserve composition: $\mathcal{C}(f \circ g) = \mathcal{C}(f) \circ \mathcal{C}(g)$.

**This is the compiler correctness property.** Compiling a composed function must equal composing the compiled functions. If this holds, then equivalent Trident programs compile to equivalent TASM — the compiler cannot introduce behavioral differences.

### Natural Transformations as Compiler Optimizations

An optimization pass is a natural transformation $\eta : \mathcal{C} \Rightarrow \mathcal{C}'$ between two compiler implementations. Naturality means the optimization commutes with program composition — optimizing a composed program gives the same result as optimizing each component and composing the results.

This is a precise statement of what it means for an optimization to be correct: it cannot break modularity.

### Adjunctions and Proof Obligations

Decompilation (TASM → TIR) is a functor in the opposite direction. A correct compiler/decompiler pair forms an adjunction — the unit $\eta : Id_{\mathbf{Trident}} \Rightarrow \mathcal{D} \circ \mathcal{C}$ and counit $\epsilon : \mathcal{C} \circ \mathcal{D} \Rightarrow Id_{\mathbf{TASM}}$ express that compilation and decompilation are inverse up to coherent isomorphism.

## Key Tradeoffs

**Proof granularity**: Proving the full functor law requires formalizing the entire Trident type system and TASM semantics in a proof assistant (Lean, Coq). This is a multi-year project. A practical intermediate: prove the functor law holds for each individual IR lowering pass, then compose the partial proofs.

**Extension to effects**: Pure Trident functions are morphisms. Effectful programs (those that touch the proof trace non-trivially) require enriched categories or indexed categories. The categorical framework scales, but the formalism becomes heavier.

## Implementation Path

1. Formalize Trident's core type system in Lean (types + function types, no dependent types first)
2. Formalize TASM semantics (stack machine, instruction denotations)
3. Prove each compiler pass preserves morphism equivalence
4. Compose pass-level proofs into a global functor theorem
5. Machine-check the proof as part of the build system (`lean --check compiler_correctness.lean`)
