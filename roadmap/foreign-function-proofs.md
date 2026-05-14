---
status: draft
author: mastercyb
area: interop
planned: 64K
---

# Foreign Function Proofs

## Motivation

Not every computation can be written in Trident. Legacy C libraries, GPU kernels, high-performance Rust crates — these will remain outside the Trident ecosystem for the foreseeable future. A proof system that can only prove Trident-native code is limited to what was written specifically for it.

Foreign function proofs extend the Trident trust boundary to external computations. A foreign function can be called from Trident, and if it provides a STARK proof of its own execution, the Trident program verifies that proof internally. The Trident proof then transitively covers the foreign call — without requiring the foreign function to be rewritten in Trident.

This is proof composition at the FFI boundary.

## Design

### The `extern verified fn` Declaration

```trident
extern verified fn external_hash(input: Field) -> Field
    with proof: StarkProof;
```

This declares that `external_hash` is implemented outside Trident but comes with a STARK proof guarantee. The `with proof: StarkProof` clause specifies that every call to this function must provide a STARK proof of its computation.

### Call Semantics

When a Trident program calls an `extern verified fn`:

```trident
// This call:
let result = external_hash(my_input);

// Expands to:
let (result, foreign_proof) = external_hash_raw(my_input);
// Where external_hash_raw returns both the result and its STARK proof

// Verify the proof inline:
let valid = verify_stark(foreign_proof, my_input, result);
assert!(valid);  // If invalid: STARK constraint violated → Trident proof invalid

// result is now safe to use — its computation is transitively proven
```

The expansion is automatic. The developer writes the call naturally; the compiler generates the proof verification step. The resulting Trident trace includes the verification — and the Trident STARK proof covers both the native computation and the proof of the foreign computation.

### What the Foreign Function Must Provide

The foreign function implementation must:
1. Execute its computation
2. Generate a STARK proof of that execution
3. Return both the result and the proof

This requires the foreign function to have its own proof-generation infrastructure. For Rust, this means using trisha or a compatible STARK prover. For C, a foreign prover library. The foreign function does not need to be written in Trident — it needs to produce a STARK proof.

```rust
// Rust implementation of external_hash:
pub fn external_hash(input: GoldilocksField) -> (GoldilocksField, StarkProof) {
    let result = my_hash_function(input);
    let proof = prove_hash_computation(input, result);  // using trisha
    (result, proof)
}
```

### Transitive Coverage

The Trident STARK proof covers:
1. All native Trident computation in the program
2. All `verify_stark` calls (the proof verification steps)
3. Transitively: all foreign computations whose proofs were verified

A verifier who accepts the Trident proof learns that all foreign function calls produced results consistent with their declared specifications — even without running the foreign functions or having access to them.

```
Trident proof covers:
├── Native computation A
├── verify_stark(foreign_proof_1, ...)  → foreign function 1 is transitively covered
├── Native computation B
└── verify_stark(foreign_proof_2, ...)  → foreign function 2 is transitively covered
```

### Proof Caching

Foreign function proofs can be cached. If `external_hash(42)` has been proven once, subsequent calls with the same input can reuse the cached proof — no re-execution of the foreign function needed. The cache maps `(function_id, input) → (result, proof)`.

```trident
// Automatic caching for pure extern verified functions:
let result1 = external_hash(42);  // executes, proves, caches
let result2 = external_hash(42);  // cache hit: reuses proof, no re-execution
```

The cache is valid only for pure functions (same input always produces same output with same proof). The `extern verified fn` declaration includes a `#[pure]` annotation for cacheable functions.

### Non-Trident Proof Systems

The foreign function's proof need not be a Triton VM STARK. If the foreign function produces a Miden proof, Plonk proof, or Groth16 proof, the Trident program can still verify it — using the appropriate verifier from the `zheng` verifier library or the `std/interop` module.

```trident
extern verified fn ethereum_snark_function(input: Field) -> Field
    with proof: Groth16Proof;  // Groth16, not STARK
// Compiler generates: verify_groth16(proof, input, result) step inside Trident
```

The proof format is part of the `extern verified fn` declaration. The compiler generates the appropriate verification step based on the declared proof format.

## Key Tradeoffs

**Foreign prover requirement**: The foreign function must generate proofs. This excludes legacy code that cannot be modified to add proof generation. For such code, the alternative is to port it to Trident (rewrite) or to trust it without proof (conventional FFI, bypassing the proof guarantee).

**Proof verification cost**: Each `verify_stark` call adds significant Processor rows to the Trident trace. A program that calls many foreign functions, each requiring proof verification, may have its trace dominated by verification overhead. The developer should batch foreign calls where possible and consider whether the transitive proof guarantee is worth the cost.

**Version coupling**: The foreign function's proof must be generated with a prover version compatible with the verifier embedded in Trident. Version mismatches produce invalid verifications even when the computation is correct. A strict versioning protocol is needed.

**Soundness of external provers**: Trident's transitive coverage is only as sound as the foreign prover's soundness. If the external STARK prover has a bug that allows invalid proofs to be accepted, Trident's verification will accept them too. The chain of trust goes: Trident STARK → Trident verifier → foreign STARK → foreign prover correctness.

## Implementation Sketch

```rust
// typecheck/extern_verified.rs
struct ExternVerifiedFn {
    name: String,
    inputs: Vec<Type>,
    output: Type,
    proof_type: ProofFormat,  // STARK, Groth16, Plonk, etc.
}

// tir/ffi.rs
fn lower_extern_verified_call(
    func: &ExternVerifiedFn,
    args: Vec<TirExpr>,
    tir: &mut TirBuilder,
) -> TirVar {
    // Emit the foreign call (runtime calls the external function)
    let (result, proof) = tir.emit(TasmOp::ExternCall(func.name.clone(), args));

    // Emit proof verification
    let verifier = match func.proof_type {
        ProofFormat::TritonStark => tir.inline_triton_verifier(),
        ProofFormat::Groth16 => tir.inline_groth16_verifier(),
        // ...
    };
    let valid = tir.emit(TasmOp::Call(verifier, vec![proof, args[0], result]));

    // Assert validity — becomes STARK constraint
    tir.emit_constraint(TirExpr::IsTrue(valid));

    result
}
```

The most important implementation decision is the proof format for the `StarkProof` type passed across the FFI boundary. It must be a stable, versioned binary format that both the foreign prover and the Trident verifier understand without any runtime negotiation.
