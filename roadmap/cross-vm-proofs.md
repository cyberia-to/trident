---
status: draft
author: mastercyb
area: interop
planned: 128K
---

# Cross-VM Recursive Proof Composition

## Motivation

Different proof systems serve different ecosystems. Triton VM is optimal for Trident's algebraic computation model. Miden VM is prevalent in the Polygon ecosystem. SP1 and OpenVM serve Ethereum-adjacent applications. A proof that only exists within one VM is an island — it cannot be composed with proofs from other systems.

Cross-VM proof composition enables a Trident proof to be verified inside another VM's program, producing a composed proof that spans both execution environments. This is recursive proof composition across heterogeneous VMs: the outer VM's proof transitively covers the inner VM's computation. Proof systems become interoperable at the cryptographic level, not just at the API level.

## Design

### The `current_proof()` Intrinsic

```trident
// On Triton VM:
let result = compute_something(input);
let proof  = current_proof();  // intrinsic: the STARK proof of this execution so far
```

`current_proof()` returns the STARK proof of the current execution up to the call site. This proof is a field element sequence that can be passed as data to other programs — including programs running on other VMs.

The proof is a first-class value in Trident. It can be stored, passed to functions, included in public outputs, and verified. It is a field element sequence under the hood — native to Trident's execution model.

### Verification Inside Another VM

```trident
// On Triton VM — produces result and proof:
let triton_result = compute_something(input);
let triton_proof  = current_proof();
// This executes on Triton VM, produces (triton_result, triton_proof)
```

```
// On Miden VM (pseudocode):
let triton_result = import_from_bundle(triton_bundle);
let verified = verify_triton_proof(triton_proof, triton_result, triton_public_inputs);
assert!(verified);
// Miden VM execution includes the Triton proof as data
// Miden's STARK proof covers the verification computation
// → Miden's proof transitively proves the Triton computation
```

### The Recursive Proof Structure

The composition creates a proof chain:

```
Triton proof P1: "compute_something(input) = triton_result"
    ↓  (P1 is embedded as data in Miden execution)
Miden proof P2: "verify_triton_proof(P1, ...) returned true"
    ↓  (transitivity)
P2 proves: "compute_something(input) = triton_result" via Triton VM
```

A verifier who checks P2 (the Miden proof) learns both that Miden ran correctly and that the Triton computation it verified was correct. The Triton computation is covered without the verifier having a Triton VM verifier — only a Miden VM verifier is needed.

### The `zheng` Verifier

For Trident-side verification of cross-VM proofs, a Trident program called `zheng` implements STARK verification for major proof systems. `zheng` runs on Triton VM, takes a foreign VM's STARK proof as input, and verifies it. The result is a Triton proof of foreign proof validity.

```trident
// zheng.tri (verifier component)
fn verify_miden_proof(
    miden_proof: StarkProof,
    public_outputs: [Field; N],
) -> bool {
    // STARK verification algorithm for Miden's proof system
    // Running on Triton VM
    // This function's execution produces a Triton proof
    miden_verify(miden_proof, public_outputs)
}
```

When `zheng` is compiled and its execution proven, the resulting Triton proof is a proof that the Miden proof was valid. Recursive composition in the opposite direction.

### Multi-Layer Composition

Composition can chain across more than two VMs:

```
Triton proof P1: "f(input) = x"
    ↓ verified inside SP1
SP1 proof P2: "Triton proof P1 is valid"
    ↓ verified inside Ethereum EVM (via Groth16 recursion)
EVM proof P3: "SP1 proof P2 is valid"
    ↓
On-chain settlement: EVM verifies P3 in a single transaction
```

The Ethereum transaction proves, transitively, that `f(input) = x` — computation that originated on Triton VM, composed through SP1, settled on Ethereum. No single VM runs the full computation. Each layer handles the verification it is best suited for.

## Key Tradeoffs

**Verifier implementation cost**: Implementing a correct STARK verifier for each target VM is substantial engineering work. Each verifier is a complex program that must be verified correct itself. The `zheng` verifier is the most critical component and requires the most validation effort.

**Proof size across layers**: Each composition layer adds proof overhead. A Triton proof embedded in a Miden execution is data that Miden must process and prove. Larger proofs are more expensive to verify and embed. Neural proof compression (separate proposal) directly reduces this cost.

**Field compatibility**: Different VMs operate over different prime fields. Triton VM uses Goldilocks; Miden uses Goldilocks; SP1 uses BabyBear; EVM uses BN254. Cross-field verification requires either:
- A verifier that translates between fields (expensive)
- A common proof format that is field-agnostic (future work)
The proposal currently assumes field-compatible VMs for direct composition, and requires translation layers for field-incompatible ones.

**Soundness of recursive verification**: Recursive STARK proofs have soundness that depends on the soundness of the inner verifier implementation. A bug in the `zheng` verifier would allow invalid inner proofs to appear valid in the outer proof. The verifier must be formally verified — ideally using Trident's own verification features to verify the verifier.

**Planned release: 128K**: This is the most ambitious proposal in the roadmap. It requires the `zheng` verifier implemented in Trident, the self-hosting compiler to compile `zheng` correctly, and stable interoperability agreements with other VM ecosystems. The 128K milestone reflects this depth.

## Implementation Sketch

```rust
// The `current_proof()` intrinsic is implemented in the runtime:
// runtime/intrinsics.rs
pub fn current_proof(state: &ExecutionState) -> FieldSequence {
    state.current_stark_proof()
}

// The cross-VM verification call:
// std/interop/verify.tri (Trident source)
extern fn verify_triton_proof(
    proof: StarkProof,
    public_inputs: [Field; N],
    public_outputs: [Field; M],
) -> bool with proof: StarkProof;
// When running on Miden: this calls the Miden-native Triton verifier
// When running on Triton: this calls zheng

// zheng.tri (the verifier for external proof systems — planned for 128K)
fn verify_sp1_proof(
    proof: ForeignStarkProof,
    outputs: [Field; N],
) -> bool {
    // Implement SP1's STARK verification algorithm in Trident
    // This is ~10,000 lines of Trident code
    sp1_fri_verify(proof) && sp1_constraint_check(proof, outputs)
}
```

The development sequence: (1) implement `current_proof()` intrinsic in the runtime, (2) implement cross-VM proof passing in the bundle format, (3) implement `zheng` for Miden (simplest, same field), (4) extend to SP1 (different field, requires translation layer), (5) extend to EVM-compatible systems.
