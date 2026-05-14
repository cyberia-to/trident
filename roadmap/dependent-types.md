---
status: draft
author: mastercyb
area: type system
planned: 32K
---

# Dependent Types over Field Dimensions

**Related proposals:** [[refinement-types]], [[proof-cost-types]], [[noun-types]]
**Reference:** [language.md §2 — Types](../reference/language.md)

> **Language evolution note**: language.md §2 defines the current type system: Field, Bool, U32, Digest, arrays, structs, and XField (§15). Dependent types over const parameters extend beyond what is in §2. This is a language evolution proposal — it describes where the type system should go, not what it currently supports.

## Motivation

Matrix dimension mismatches are runtime errors in every language that doesn't have dependent types. In proof systems, runtime errors are worse than in ordinary programs — a mismatched dimension doesn't produce a wrong answer the developer can debug; it produces a constraint violation that fails proof generation, with no clear indication of where the mismatch occurred.

Trident's execution model is native to field arithmetic. Dimensions are natural numbers, and natural numbers embed directly in the Goldilocks field. There is no gap between the language's numeric type and the dimension type. This makes dependent types over field dimensions natural, not bolted on. Dimension mismatches become compile-time errors. No runtime bounds checking is generated. No wasted Processor rows on dimension assertions.

## Design

### Dependent Type Syntax

```trident
type Vector<const N: Field> = [Field; N];
type Matrix<const R: Field, const C: Field> = [Vector<C>; R];
```

The type parameters `N`, `R`, `C` are `const` field elements — values that must be known at compile time. This restriction enables static dimension checking without runtime overhead.

### Dimension-Safe Operations

```trident
fn matmul<const M: Field, const N: Field, const P: Field>(
    a: Matrix<M, N>,
    b: Matrix<N, P>
) -> Matrix<M, P> {
    // Inner dimension N must match — enforced at call site by type checker
    // No runtime check needed
}

fn dot<const N: Field>(a: Vector<N>, b: Vector<N>) -> Field {
    // Dimension N must match — enforced at call site
}

fn concat<const A: Field, const B: Field>(
    u: Vector<A>,
    v: Vector<B>
) -> Vector<{ A + B }> {
    // Result dimension is A + B — computed at compile time in the type
}
```

The type checker verifies at every call site that dimension parameters match. If `A = 3` and `B = 4`, then `concat(u, v)` has type `Vector<7>` — computed statically.

### Field-Native Dimension Arithmetic

Because dimensions are field elements, dimension arithmetic is field arithmetic. This enables unusual but natural patterns:

```trident
// A neural network layer with field-native dimension types:
fn linear_layer<const IN: Field, const OUT: Field>(
    weights: Matrix<OUT, IN>,
    bias: Vector<OUT>,
    input: Vector<IN>
) -> Vector<OUT> {
    matmul(weights, input) + bias
}

// Batch processing with dependent output dimension:
fn batch_hash<const N: Field>(
    inputs: [Field; N]
) -> [Field; N] {  // output has same dimension as input
    inputs.map(|x| hash(x))
}
```

### Dimension Unification

When two expressions with unknown dimensions must have the same dimension (e.g., two vectors being added), the type checker unifies the dimension type variables. This is straightforward for constant dimensions and requires a small amount of type inference for expressions where dimensions depend on function arguments.

```trident
fn add_vectors<const N: Field>(a: Vector<N>, b: Vector<N>) -> Vector<N> {
    // Type checker unifies: a's dimension = b's dimension = N = result dimension
}

// At call site:
let u = Vector::<3>::zeros();
let v = Vector::<3>::ones();
let w = add_vectors(u, v);  // N = 3, inferred from u and v
```

### Interaction with the Proof System

Dimension mismatch elimination directly reduces proof cost. Every bounds check that would have been a runtime assertion (`assert!(a.len() == b.len())`) is now absent from the nox trace. For programs over matrices — which includes most neural network layers, polynomial evaluation, and linear algebra — this can eliminate tens to hundreds of nox reduction steps.

The zheng STARK proof implicitly proves that all type-checked dimension constraints were satisfied, because the proof is only valid if the program was well-typed. The dimension guarantee flows through the type system to the proof system without any additional constraint generation.

## Vision

Matrix dimensions in neural network programs written in Trident are dependent types. A `Linear<Field, 256, 512>` layer has its dimensions verified at compile time. When `std.nn` models are deployed as [[Atlas]] packages, their architecture is part of the type — there's no way to load a 512-wide model into a 256-wide slot at runtime. The [[cybergraph]]'s AI inference layer becomes dimensionally safe by construction.

This matters for a network where computation is memoized globally. When [[cybergraph]] caches the result of a neural inference call, it caches the proof alongside the answer. That proof certifies that the computation was performed by a correctly-typed model — the proof includes the dimension constraint. A cache hit on a `Linear<Field, 256, 512>` computation is a cache hit that can only be served by a query with matching dimensions. Dimension-unsafe operations cannot contaminate the global memo table.

Beyond neural networks: every polynomial evaluation, every linear algebra computation, every batch hash — all of these involve arrays whose lengths must match. Dependent types eliminate an entire class of constraint violations before they reach the [[zheng]] prover, reducing proof failures to a vanishing edge case.

## Stack Integration

[[cybergraph]] queries return proofs of the answer. Dependent types on the query return type (`Vector<N>` where N is part of the type) let [[soft3]] callers verify response dimensions at compile time. A `query(particle, dimension)` call that returns `Vector<128>` cannot be accidentally used where `Vector<256>` is expected — the type mismatch is a compile error at the [[soft3]] call site, before any network round-trip.

[[nox]]'s `ask(ν, object, formula, τ, a, v, t)` signature itself can carry dependent type constraints: `a` and `v` (alignment and variance parameters) must be within valid ranges — enforced at the type level. Every call to `ask` with dependent-typed arguments is statically verified before it reaches the VM.

[[warrior-cyber]]'s three backends (cpu/AMX, webgpu, metal) handle matrix operations natively. Dependent types on matrix dimensions enable the compiler to select the optimal kernel at compile time — a `Matrix<256, 256>` operation routes to AMX tile instructions, a `Matrix<8, 8>` operation routes to scalar code. No runtime dispatch, no branch on dimension, just the right code for the right shape.

## Key Tradeoffs

**Compile-time restriction**: Dimensions must be `const` — known at compile time. Programs that allocate arrays of size determined at runtime cannot use dependent types directly. A separate dynamic-dimension path exists (runtime-sized arrays with runtime bounds checks), but it foregoes the proof cost savings.

**Dependent arithmetic in types**: Type expressions like `Vector<{ A + B }>` require computing field arithmetic in the type system. For linear expressions, this is straightforward. For quadratic or higher expressions, the type checker may not be able to simplify them, leading to type annotation burden on the developer.

**Interaction with const generics**: The `const N: Field` syntax mirrors Rust's const generics but over a finite field rather than `usize`. The implementation must ensure that dimension arithmetic in types is performed in the Goldilocks field, not in the host language's native integers. A dimension of $p - 1$ wrapped to $0$ would be a subtle and catastrophic bug.

**Type inference**: Inferring dimension parameters across complex call chains requires a unification algorithm over field expressions. For most programs, the dimensions are obvious from context. For complex programs, the developer may need explicit dimension annotations to guide inference.

## Implementation Sketch

Dependent dimension types require a dimension expression evaluator in the type checker:

```rust
// typecheck/dependent.rs
#[derive(Clone, Debug, PartialEq)]
enum DimExpr {
    Const(FieldElement),
    Var(String),
    Add(Box<DimExpr>, Box<DimExpr>),
    Mul(Box<DimExpr>, Box<DimExpr>),
}

impl DimExpr {
    fn evaluate(&self, env: &HashMap<String, FieldElement>) -> Option<FieldElement> {
        match self {
            Const(v) => Some(*v),
            Var(name) => env.get(name).copied(),
            Add(a, b) => Some(a.evaluate(env)? + b.evaluate(env)?),
            Mul(a, b) => Some(a.evaluate(env)? * b.evaluate(env)?),
        }
    }

    fn unify(&self, other: &DimExpr, subst: &mut Substitution) -> Result<(), DimError> {
        // Structural unification over dimension expressions
    }
}
```

The dimension type checker runs before code generation. Its output is a substitution map from dimension variables to concrete field elements, used to generate correctly-sized nox patterns (loop bounds, array indexing, etc.) without runtime checks.
