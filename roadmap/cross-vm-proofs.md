---
status: draft
author: mastercyb
area: interop
planned: 128K
---

# Cross-VM Recursive Proof Composition

**Related proposals:** [[proof-carrying-code]], [[foreign-function-proofs]], [[cyber-stack-adoption]]
**Reference:** [language.md §16 — proof_block](../reference/language.md), [reference/ir.md](../reference/ir.md)

## Motivation

Different proof systems serve different ecosystems. nox + zheng is optimal for Trident's algebraic computation model. Miden VM is prevalent in the Polygon ecosystem. SP1 and OpenVM serve Ethereum-adjacent applications. A proof that only exists within one VM is an island — it cannot be composed with proofs from other systems.

Cross-VM proof composition enables a zheng proof to be verified inside another VM's program, producing a composed proof that spans both execution environments. This is recursive proof composition across heterogeneous VMs: the outer VM's proof transitively covers the inner VM's computation. Proof systems become interoperable at the cryptographic level, not just at the API level.

### Prior art in the language: `proof_block`

`language.md §16` already defines `proof_block` as a Tier 3 construct for STARK-in-STARK recursion within the same nox/zheng stack. This proposal extends that mechanism across VM boundaries. The single-VM `proof_block` case is the simpler foundation; cross-VM composition adds the foreign verifier layer on top.

## Design

### The `current_proof()` Intrinsic

```trident
// On nox:
let result = compute_something(input);
let proof  = current_proof();  // proposed intrinsic: the zheng proof of this execution so far
```

`current_proof()` is a proposed intrinsic that returns the zheng proof of the current nox execution up to the call site. The proof is a field element sequence (Brakedown commitment + sumcheck transcript) that can be passed as data to other programs — including programs running on other VMs.

The proof is a first-class value in Trident. It can be stored, passed to functions, included in public outputs, and verified. Under the hood it is a sequence of field elements — native to Trident's execution model.

### Verification Inside Another VM

```trident
// On nox — produces result and proof:
let nox_result = compute_something(input);
let nox_proof  = current_proof();
// Executes on nox, produces (nox_result, nox_proof as zheng proof)
```

```
// On Miden VM (pseudocode):
let nox_result = import_from_bundle(nox_bundle);
let verified = verify_zheng_proof(nox_proof, nox_result, nox_public_inputs);
assert!(verified);
// Miden VM execution includes the zheng proof as data
// Miden's own proof covers the verification computation
// → Miden's proof transitively proves the nox computation
```

### The Recursive Proof Structure

The composition creates a proof chain:

```
zheng proof P1: "compute_something(input) = nox_result"  (nox execution)
    ↓  (P1 is embedded as data in Miden execution)
Miden proof P2: "verify_zheng_proof(P1, ...) returned true"
    ↓  (transitivity)
P2 proves: "compute_something(input) = nox_result" via nox
```

A verifier who checks P2 (the Miden proof) learns both that Miden ran correctly and that the nox computation it verified was correct — without needing a standalone zheng verifier. See [[proof-carrying-code]] for how the zheng proof travels inside a `ProgramBundle`.

### The zheng Verifier Written in Trident

For Trident-side verification of cross-VM proofs, the zheng verifier for foreign proof systems is implemented as a `.tri` program running on nox. The program takes a foreign VM's proof as input, verifies it using the foreign system's verification algorithm, and produces a zheng proof that the foreign proof was valid. This is the 128K mechanism: the zheng verifier written in `.tri`, compiled to a `ProgramBundle` by `trident build`, distributed via Atlas.

```trident
// zheng_miden.tri — verifier for Miden proofs, runs on nox
fn verify_miden_proof(
    miden_proof: StarkProof,
    public_outputs: [Field; N],
) -> bool {
    // Miden STARK verification algorithm, implemented in Trident
    // Executing on nox produces a nox trace
    // zheng proves that trace → result is a zheng proof of Miden proof validity
    miden_verify(miden_proof, public_outputs)
}
```

When this program executes and zheng proves the nox trace, the resulting zheng proof is a proof that the Miden proof was valid. Recursive composition in the opposite direction. See [[foreign-function-proofs]] for the case where the foreign function is not a full VM but a single external function.

### Multi-Layer Composition

Composition can chain across more than two VMs:

```
zheng proof P1: "f(input) = x"   (nox execution)
    ↓ verified inside SP1
SP1 proof P2: "zheng proof P1 is valid"
    ↓ verified inside Ethereum EVM (via Groth16 recursion)
EVM proof P3: "SP1 proof P2 is valid"
    ↓
On-chain settlement: EVM verifies P3 in a single transaction
```

The Ethereum transaction proves, transitively, that `f(input) = x` — computation that originated on nox, composed through SP1, settled on Ethereum. No single VM runs the full computation. Each layer handles the verification it is best suited for. See [[cyber-stack-adoption]] for the broader strategy of bridging the nox/zheng stack into existing ecosystems.

## Key Tradeoffs

**Verifier implementation cost**: Implementing a correct proof verifier for each target VM is substantial engineering work. Each verifier is a complex `.tri` program that must itself be verified correct. The zheng verifier for foreign proof systems is the most critical component and requires the most validation effort.

**Proof size across layers**: Each composition layer adds proof overhead. A zheng proof embedded in a Miden execution is data that Miden must process and prove. Brakedown commitments are larger than FRI-based commitments but verify faster — the tradeoff favours the verifier side, which is what matters for embedding. Neural proof compression (separate proposal) directly reduces this cost.

**Field compatibility**: Different VMs operate over different prime fields. nox uses Goldilocks; Miden uses Goldilocks; SP1 uses BabyBear; EVM uses BN254. Cross-field verification requires either:
- A verifier that translates between fields (expensive)
- A common proof format that is field-agnostic (future work)
The proposal currently assumes field-compatible VMs for direct composition, and requires translation layers for field-incompatible ones.

**Soundness of recursive verification**: Recursive proof composition has soundness that depends on the soundness of the inner verifier implementation. A bug in the `.tri` verifier program would allow invalid inner proofs to appear valid in the outer proof. The verifier must be formally verified — ideally using Trident's own `#[ensures]` contracts and `trident audit` to verify the verifier.

**Planned release: 128K**: This is the most ambitious proposal in the roadmap. It requires the foreign-proof verifier implemented in `.tri` running on nox, the self-hosting compiler to compile it correctly, and stable interoperability agreements with other VM ecosystems. The 128K milestone reflects this depth.

## Implementation Sketch

```rust
// The `current_proof()` intrinsic — implemented in the runtime:
// runtime/intrinsics.rs
pub fn current_proof(state: &ExecutionState) -> FieldSequence {
    state.current_zheng_proof()  // Brakedown commitment + sumcheck transcript so far
}

// The cross-VM verification call:
// std/interop/verify.tri (Trident source)
extern fn verify_zheng_proof(
    proof: ZhengProof,
    public_inputs: [Field; N],
    public_outputs: [Field; M],
) -> bool;
// When running on Miden: calls the Miden-native zheng verifier
// When running on nox:  calls the .tri verifier program recursively

// std/interop/sp1.tri (foreign verifier — planned for 128K)
fn verify_sp1_proof(
    proof: ForeignStarkProof,
    outputs: [Field; N],
) -> bool {
    // SP1's STARK verification algorithm implemented in Trident, running on nox
    // ~10,000 lines of Trident code; compiled to ProgramBundle and deployed to Atlas
    sp1_fri_verify(proof) && sp1_constraint_check(proof, outputs)
}
```

The development sequence: (1) implement `current_proof()` intrinsic in the runtime, (2) extend `ProgramBundle` (`reference/ir.md`) to carry cross-VM proof fields, (3) implement the Miden verifier in `.tri` (simplest — same Goldilocks field), (4) extend to SP1 (BabyBear field, requires translation layer), (5) extend to EVM-compatible systems via Groth16 wrapping.
