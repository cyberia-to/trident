---
status: draft
author: mastercyb
area: verification
planned: 32K
---

# Termination Proofs as Compilation Artifacts

## Motivation

Conventional termination analysis asks: "Does this program halt?" The answer is a boolean property, verified separately from execution, useful for proving the absence of infinite loops. In proof systems, the question is richer: not whether the program terminates, but exactly when, and as a proven property embedded in the proof of execution.

Trident requires bounded loops — all loop counts must be statically known at compile time. This makes termination decidable by construction. The compiler goes further: it generates a proof of termination with a specific step count, embedded in the STARK proof itself. The Processor table has exactly $N$ rows, and the proof commits to this count. Anyone who verifies the STARK proof learns both that the program ran correctly and that it terminated in exactly $N$ steps for this specific input.

Deterministic step count is a proven property, not an observation.

## Design

### Bounded Loops as Termination Certificates

Every loop in Trident has a statically-known bound:

```trident
for i in 0..N { ... }    // exactly N iterations, N is a compile-time constant
for item in array { ... } // exactly array.len() iterations, known at compile time
```

The compiler computes the exact total step count for the program: the sum of all loop bounds, times all nesting depths, plus the straight-line instructions at each level. This count is a field element embedded in the STARK proof.

### The STARK Commitment to Step Count

The Processor table in Triton VM's AET has exactly one row per executed instruction. If a program executes $N$ instructions total, the Processor table has exactly $N$ rows. The STARK proof commits to the height of the Processor table as part of the proof structure (through the Brakedown commitment to the AET).

This means: verifying the STARK proof implicitly verifies that the program executed exactly $N$ instructions. The verifier, upon accepting the proof, learns the step count as a mathematical consequence of proof validity.

```trident
// Compile-time analysis:
fn compute_something(input: Field) -> Field {
    let mut x = input;
    for _ in 0..100 { x = x * x + 1; }  // 100 iterations × 2 ops = 200 steps
    for _ in 0..50 { x = hash(x); }      // 50 iterations × ~300 ops = 15000 steps
    x
}
// Compiler computes: total steps = 200 + 15000 + bookkeeping ≈ 15210
// STARK proof commits to Processor table height 15210 (rounded to next power of 2)
```

### Exact Step Count as a Feature

Knowing the exact step count enables critical application patterns:

- **Timestamping**: The step count is a precise measure of computation. If the input uniquely determines the step count, the proof is a timestamp with cryptographic precision.
- **Fair resource charging**: In a distributed system, parties can charge for computation based on proven step counts. No estimation, no profiling, no trust — the proof is the bill.
- **Scheduling guarantees**: A real-time system can guarantee that a computation completes within a deadline if the step count is proven bounded. The STARK proof is the deadline guarantee.
- **Determinism verification**: Two parties executing the same program on the same input produce identical STARK proofs (same step count, same trace). Disagreement on step count reveals non-determinism or implementation divergence.

### Termination Proof Generation

The compiler generates the termination proof as part of normal compilation:

```
Source → TIR → Step Count Analysis → TASM with step count annotation → STARK proof
                      ↓
                Step count: N (field element)
                Embedded in: Processor table height commitment in STARK proof
```

No additional proof artifact is needed. The termination proof is the STARK proof. Presenting the STARK proof to a verifier is presenting the termination proof.

### Interaction with Supercompilation

Supercompilation can change the step count dramatically (collapsing a 1000-iteration loop to a 10-step exponentiation). After supercompilation, the compiler recomputes the step count from the optimized TASM. The optimized STARK proof commits to the smaller step count — proving that the optimized program terminates faster.

Two STARK proofs for the same function (before and after optimization) can be compared: both commit to the same input/output relationship, but different step counts. The smaller count is the tighter termination proof.

## Key Tradeoffs

**Static bound requirement**: The termination proof system requires all loop bounds to be statically known. Programs with data-dependent loop bounds (`for i in 0..x where x is runtime`) cannot generate precise termination proofs. The compiler must either reject these or fall back to a worst-case upper bound, which produces a weaker termination claim ("terminates in at most N steps" rather than "terminates in exactly N steps").

**Proof size vs. precision**: Committing to the exact step count makes the proof marginally larger (one additional field element in the commitment). This is negligible. However, step count is implicitly committed through the Processor table height, which is already part of the STARK proof structure — no additional data is needed.

**Optimization and step count drift**: Aggressive optimization (especially supercompilation) dramatically changes the step count. If external systems depend on a specific step count (e.g., for scheduling), optimization must be disabled or the step count contract must be explicitly maintained. The compiler should warn when optimization changes the step count by more than a configurable threshold.

**Recursive programs**: Trident's bounded loop requirement limits recursion depth too (via unrolling). Deep recursive programs must be restructured as iterative programs with explicit stacks. This is a design constraint that stems from the termination proof requirement, not from any technical limitation of STARK proofs.

## Implementation Sketch

Step count analysis is a static pass over the TASM output:

```rust
// cost/termination.rs
fn compute_step_count(program: &TasmProgram) -> FieldElement {
    let mut count = FieldElement::zero();
    for instruction in program.instructions() {
        count += instruction.processor_rows();  // usually 1, varies by instruction type
    }
    count
}

fn embed_step_count(proof: &mut StarkProof, count: FieldElement) {
    // The step count is already implicit in the Processor table height
    // This function validates the claimed height matches the actual table
    assert_eq!(proof.processor_table_height(), count.ceiling_power_of_2());
}
```

The termination proof module is primarily documentation and verification infrastructure — the underlying mechanism (Processor table height commitment) is inherent to the STARK system and requires no additional implementation.
