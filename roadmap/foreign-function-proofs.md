---
status: draft
author: mastercyb
area: interop
planned: 64K
---

# Foreign Function Proofs

**Related proposals:** [[proof-carrying-code]], [[cross-vm-proofs]], [[warrior-architecture]], [[cybergraph]], [[warrior-cyber]], [[Atlas]], [[zheng]]
**Reference:** [reference/ir.md — Asm passthrough (inline assembly precedent)](../reference/ir.md)

## Vision

Legacy code — Rust libraries, C cryptographic primitives, GPU kernels — enters the [[cybergraph]] through `extern verified fn`. Each foreign call produces a [[zheng]] proof of its execution. The proof is submitted as a cyberlink: the foreign function's input particle → the proven output particle. Over time, the [[cybergraph]] accumulates verified execution traces of the world's most important computations.

A researcher who wants to use a result doesn't need to re-run the computation — they call `ask(verify, result_cid)` and receive the cached proof. The global research computation graph becomes a layer of the [[cybergraph]]: every numerical library, every cryptographic primitive, every GPU kernel that has ever run under `extern verified fn` leaves a permanent, proven trace.

`extern verified fn` bridges [[warrior-cyber]] (which runs [[nox]]) and external proving systems. For Rust functions, the initial implementation uses the native backend (x86-64/ARM64) with a proof adapter: the Rust function executes natively, [[warrior-cyber]] wraps the execution in a [[zheng]] proof. For GPU kernels, the [[warrior-cyber]] metal backend calls out to aruminium (pure Metal, unimem zero-copy) — aruminium executes the kernel, warrior-cyber seals the result with a [[hemera]]-addressed proof and submits it to the [[cybergraph]] as a cyberlink. The `#[pure]` annotation enables proof caching: once `external_hash(42)` is proven and its result cyberlinked in the graph, subsequent calls hit the [[cybergraph]] cache at zero cost.

## Motivation

Not every computation can be written in Trident. Legacy C libraries, GPU kernels, high-performance Rust crates — these will remain outside the Trident ecosystem for the foreseeable future. A proof system that can only prove Trident-native code is limited to what was written specifically for it.

Foreign function proofs extend the Trident trust boundary to external computations. A foreign function can be called from Trident, and if it provides a proof of its own execution, the Trident program verifies that proof internally. The zheng proof of the Trident execution then transitively covers the foreign call — without requiring the foreign function to be rewritten in Trident.

This is proof composition at the FFI boundary. The precedent in the IR is the `Asm` passthrough for inline assembly (`reference/ir.md`): foreign opcodes are already threaded through the TIR pipeline. `extern verified fn` extends that passthrough with a proof obligation.

## Design

### The `extern verified fn` Declaration

```trident
extern verified fn external_hash(input: Field) -> Field
    with proof: ZhengProof;  // or Groth16Proof, PlonkProof, etc.
```

This declares that `external_hash` is implemented outside Trident but comes with a proof guarantee. The `with proof:` clause specifies the proof format — every call to this function must provide a proof of its computation in that format.

### Call Semantics

When a Trident program calls an `extern verified fn`, the compiler expands the call to include proof verification. The expanded nox trace includes the verification step, so zheng covers it transitively:

```trident
// This call:
let result = external_hash(my_input);

// Expands to:
let (result, foreign_proof) = external_hash_raw(my_input);
// external_hash_raw returns both the result and its proof

// Verify the proof inline (becomes part of the nox trace):
let valid = verify_proof(foreign_proof, my_input, result);
assert!(valid);  // If invalid: nox constraint violated → zheng proof invalid

// result is now safe to use — its computation is transitively covered by zheng
```

The expansion is automatic. The developer writes the call naturally; the compiler generates the verification step. The resulting nox trace includes the verification, and zheng proves that trace. The `extern verified fn` mechanism integrates with warrior-cyber's execution model: when warrior-cyber executes a `ProgramBundle` containing `extern verified fn` calls, it routes the foreign calls to registered native implementations and collects their proofs before the nox trace is sealed for proving.

### What the Foreign Function Must Provide

The foreign function implementation must:
1. Execute its computation
2. Generate a proof of that execution (zheng or a compatible proof system)
3. Return both the result and the proof

This requires the foreign function to have its own proof-generation infrastructure. For Rust, this means using trisha or a compatible prover. For C, a foreign prover library. The foreign function does not need to be written in Trident — it needs to produce a proof in a format that the Trident verifier can check.

```rust
// Rust implementation of external_hash:
pub fn external_hash(input: GoldilocksField) -> (GoldilocksField, ZhengProof) {
    let result = my_hash_function(input);
    let proof = zheng_prove_computation(input, result);  // using trisha
    (result, proof)
}
```

### Transitive Coverage

The zheng proof covers the complete nox trace, which includes:
1. All native Trident computation in the program
2. All `verify_proof` calls (the proof verification steps)
3. Transitively: all foreign computations whose proofs were verified

A verifier who accepts the zheng proof learns that all foreign function calls produced results consistent with their declared specifications — even without running the foreign functions or having access to them.

```
zheng proof covers the nox trace:
├── Native computation A
├── verify_proof(foreign_proof_1, ...)  → foreign function 1 is transitively covered
├── Native computation B
└── verify_proof(foreign_proof_2, ...)  → foreign function 2 is transitively covered
```

See [[cross-vm-proofs]] for the case where the "foreign function" is an entire foreign VM's execution rather than a single function.

### Proof Caching

Foreign function proofs can be cached. If `external_hash(42)` has been proven once, subsequent calls with the same input can reuse the cached proof — no re-execution of the foreign function needed. The cache maps `(function_id, input) → (result, proof)`.

```trident
// Automatic caching for pure extern verified functions:
let result1 = external_hash(42);  // executes, proves, caches
let result2 = external_hash(42);  // cache hit: reuses proof, no re-execution
```

The cache is valid only for pure functions (same input always produces same output with same proof). The `extern verified fn` declaration includes a `#[pure]` annotation for cacheable functions.

### Non-zheng Proof Systems

The foreign function's proof need not be a zheng proof. If the foreign function produces a Miden proof, Plonk proof, or Groth16 proof, the Trident program can still verify it — using the appropriate verifier from the `std/interop` module (the same verifiers developed under [[cross-vm-proofs]]).

```trident
extern verified fn ethereum_snark_function(input: Field) -> Field
    with proof: Groth16Proof;  // Groth16, not zheng
// Compiler generates: std::interop::verify_groth16(proof, input, result) in the nox trace
```

The proof format is part of the `extern verified fn` declaration. The compiler generates the appropriate verification step based on the declared proof format.

## Key Tradeoffs

**Foreign prover requirement**: The foreign function must generate proofs. This excludes legacy code that cannot be modified to add proof generation. For such code, the alternative is to port it to Trident (rewrite) or to trust it without proof (conventional FFI, bypassing the proof guarantee).

**Proof verification cost**: Each `verify_proof` call adds nox reduction steps to the trace (and correspondingly extends the zheng sumcheck). A program that calls many foreign functions, each requiring proof verification, may have its trace dominated by verification overhead. The developer should batch foreign calls where possible and consider whether the transitive proof guarantee is worth the cost.

**Version coupling**: The foreign function's proof must be generated with a prover version compatible with the verifier embedded in Trident. Version mismatches produce invalid verifications even when the computation is correct. A strict versioning protocol is needed.

**Soundness of external provers**: Trident's transitive coverage is only as sound as the foreign prover's soundness. If the external STARK prover has a bug that allows invalid proofs to be accepted, Trident's verification will accept them too. The chain of trust goes: Trident STARK → Trident verifier → foreign STARK → foreign prover correctness.

## Implementation Sketch

```rust
// typecheck/extern_verified.rs
struct ExternVerifiedFn {
    name: String,
    inputs: Vec<Type>,
    output: Type,
    proof_type: ProofFormat,  // Zheng, Groth16, Plonk, etc.
}

// tir/ffi.rs  (the Asm passthrough in reference/ir.md is the structural precedent)
fn lower_extern_verified_call(
    func: &ExternVerifiedFn,
    args: Vec<TirExpr>,
    tir: &mut TirBuilder,
) -> TirVar {
    // Emit the foreign call (warrior-cyber routes this to the native implementation)
    let (result, proof) = tir.emit(TirOp::ExternCall(func.name.clone(), args));

    // Emit proof verification into the nox trace
    let verifier = match func.proof_type {
        ProofFormat::Zheng   => tir.inline_zheng_verifier(),
        ProofFormat::Groth16 => tir.inline_groth16_verifier(),
        // ...
    };
    let valid = tir.emit(TirOp::Call(verifier, vec![proof, args[0], result]));

    // Assert validity — becomes a nox constraint, covered by zheng
    tir.emit_constraint(TirExpr::IsTrue(valid));

    result
}
```

The most important implementation decision is the proof format for the `ZhengProof` type (or foreign proof type) passed across the FFI boundary. It must be a stable, versioned binary format that both the foreign prover and the Trident verifier understand without runtime negotiation.
