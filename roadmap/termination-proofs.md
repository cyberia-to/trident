---
status: draft
author: mastercyb
area: verification
planned: 32K
---

# Termination Proofs as Compilation Artifacts

**Related:** [[contracts]] · [[loop-invariants]]

## Motivation

Conventional termination analysis asks: "Does this program halt?" The answer is a boolean property, verified separately from execution, useful for proving the absence of infinite loops. In proof systems, the question is richer: not whether the program terminates, but exactly when, and as a proven property embedded in the proof of execution.

Trident requires bounded loops — all loop counts must be statically known at compile time. This makes termination decidable by construction. The compiler goes further: it generates a proof of termination with a specific nox trace length, embedded in the zheng proof itself. The nox trace has exactly $N$ reduction steps, and the proof commits to this count via Brakedown PCS. Anyone who verifies the proof learns both that the program ran correctly and that it terminated in exactly $N$ reduction steps for this specific input.

Deterministic step count is a proven property, not an observation.

### Connection to nox's Focus Budget

nox has a built-in focus budget (the `τ` parameter in `ask`) that bounds total execution depth. The termination proof in nox is not separate from this mechanism — the focus budget commitment in the trace IS the termination proof. When the compiler statically computes the step count, it is computing an upper bound for `τ`. If the trace fits within `τ`, the nox execution terminates and the zheng proof is valid. Exceeding the budget is not a runtime error; it is a constraint violation that makes the proof impossible to generate.

## Design

### Bounded Loops as Termination Certificates

Every loop in Trident has a statically-known bound:

```trident
for i in 0..N { ... }    // exactly N iterations, N is a compile-time constant
for item in array { ... } // exactly array.len() iterations, known at compile time
```

The compiler computes the exact total step count for the program: the sum of all loop bounds, times all nesting depths, plus the straight-line instructions at each level. This count is a field element embedded in the zheng proof via the nox trace length commitment.

### The nox Trace Commitment to Step Count

The nox trace records every reduction step. If a program executes $N$ reduction steps total, the trace has exactly $N$ entries. The zheng proof commits to the nox trace length as part of the proof structure (through the Brakedown PCS commitment).

This means: verifying the zheng proof implicitly verifies that the program executed exactly $N$ reduction steps. The verifier, upon accepting the proof, learns the step count as a mathematical consequence of proof validity.

```trident
// Compile-time analysis:
fn compute_something(input: Field) -> Field {
    let mut x = input;
    for _ in 0..100 { x = x * x + 1; }  // 100 iterations × reduction steps each
    for _ in 0..50 { x = hash(x); }      // 50 iterations × hash jet cost each
    x
}
// Compiler computes: total reduction steps = sum of per-iteration nox costs
// zheng proof commits to nox trace length via Brakedown PCS
```

Note: the per-instruction cost is the nox reduction step count plus jet costs (hash jet, merkle_step jet, etc.) — not "AET table heights." See [[loop-invariants]] for how per-iteration invariant constraints add to this count.

### Exact Step Count as a Feature

Knowing the exact step count enables critical application patterns:

- **Timestamping**: The step count is a precise measure of computation. If the input uniquely determines the step count, the proof is a timestamp with cryptographic precision.
- **Fair resource charging**: In a distributed system, parties can charge for computation based on proven step counts. No estimation, no profiling, no trust — the proof is the bill.
- **Scheduling guarantees**: A real-time system can guarantee that a computation completes within a deadline if the step count is proven bounded. The zheng proof of the nox trace is the deadline guarantee.
- **Determinism verification**: Two parties executing the same program on the same input produce identical zheng proofs (same nox trace length, same trace). Disagreement on step count reveals non-determinism or implementation divergence.

### Termination Proof Generation

The compiler generates the termination proof as part of normal compilation:

```
Source → TIR → Step Count Analysis → nox patterns → zheng proof
                      ↓
                Step count: N (field element, ≤ τ focus budget)
                Embedded in: nox trace length commitment in zheng proof (Brakedown PCS)
```

No additional proof artifact is needed. The termination proof is the zheng proof of the nox trace. Presenting the zheng proof to a verifier is presenting the termination proof. The `#[requires]`/`#[ensures]` contracts from [[contracts]] are nox constraints within the same trace — same proof covers both execution and specification compliance.

### Interaction with Supercompilation

Supercompilation can change the step count dramatically (collapsing a 1000-iteration loop to a 10-step exponentiation). After supercompilation, the compiler recomputes the step count from the optimized nox patterns. The optimized zheng proof commits to the smaller nox trace length — proving that the optimized program terminates faster.

Two zheng proofs for the same function (before and after optimization) can be compared: both commit to the same input/output relationship, but different nox trace lengths. The smaller count is the tighter termination proof.

## Vision

In the cyber network, every [[nox]] computation is bounded by a focus budget (τ). The termination proof IS the focus budget commitment — the prover commits to exactly τ steps via the Brakedown PCS commitment to the [[nox]] trace length. [[bbg]] can verify this commitment before executing anything, enabling pre-authorization of complex computations.

A governance proposal that says "this upgrade requires no more than 50,000 [[nox]] steps" is verifiable without running it. The compiler produces the step count as a static artifact. Any node can check the claim by reviewing the TIR step-count analysis output — no execution, no proof generation, just a deterministic analysis pass. The proposal's computational cost is public knowledge before the vote, not a surprise revealed at execution time.

This transforms how the network reasons about focus economics. Today, focus pricing is reactive — you pay after you compute. With termination proofs embedded at compile time, focus pricing becomes predictive. The network can implement futures markets for focus: "I'll prove this computation for τ steps at price P, and here's the compiler's termination certificate that τ is sufficient." The certificate is the market instrument.

## Stack Integration

[[soft3]]'s `query(cid, dimension)` returns a termination proof alongside the result. The caller knows not just the answer but exactly how expensive the computation was — the [[zheng]] proof commits to it via the [[nox]] trace length. This is the foundation for honest focus pricing: every result comes with a certified computation receipt.

[[bbg]] uses termination proofs for focus accounting. When a [[warrior-cyber]] instance submits a completed computation, it submits the [[zheng]] proof (which commits to the trace length) alongside the result. BBG reads the committed step count from the proof, deducts the corresponding focus from the participant's budget, and updates the state. No self-reporting, no estimation — the proof is the bill.

The interaction with [[cybergraph]] is particularly powerful. [[cybergraph]]'s global memoization means that as the graph grows, fewer computations execute over time. Termination proofs make this efficiency gain visible: a cache hit returns not just the cached result but the original computation's termination proof. A caller can see that the computation was previously proven to terminate in N steps — and know they paid zero steps for the cache hit. The difference between "computed" and "looked up" is visible in the proof structure, and the focus savings are mathematically certified.

## Key Tradeoffs

**Static bound requirement**: The termination proof system requires all loop bounds to be statically known. Programs with data-dependent loop bounds (`for i in 0..x where x is runtime`) cannot generate precise termination proofs. The compiler must either reject these or fall back to a worst-case upper bound, which produces a weaker termination claim ("terminates in at most N steps" rather than "terminates in exactly N steps").

**Proof size vs. precision**: The step count is implicitly committed through the nox trace length, which is already part of the zheng proof structure (Brakedown PCS commitment) — no additional data is needed. No separate field element required.

**Optimization and step count drift**: Aggressive optimization (especially supercompilation) dramatically changes the step count. If external systems depend on a specific step count (e.g., for scheduling), optimization must be disabled or the step count contract must be explicitly maintained. The compiler should warn when optimization changes the step count by more than a configurable threshold.

**Recursive programs**: Trident's bounded loop requirement limits recursion depth too (via unrolling). Deep recursive programs must be restructured as iterative programs with explicit stacks. This is a design constraint that stems from the termination proof requirement — the nox focus budget `τ` must be a finite constant — not from any technical limitation of zheng proofs.

## Implementation Sketch

Step count analysis is a static pass over the nox pattern output:

```rust
// cost/termination.rs
fn compute_step_count(program: &NoxProgram) -> FieldElement {
    let mut count = FieldElement::zero();
    for step in program.reduction_steps() {
        count += step.cost();  // 1 for base patterns, jet-specific for hash/merkle/ntt jets
    }
    count
}

fn validate_focus_budget(count: FieldElement, tau: FieldElement) {
    // nox's focus budget (τ) must cover the full trace
    // If count > τ, the execution cannot be proven
    assert!(count <= tau, "trace exceeds focus budget τ");
}
```

The termination proof module is primarily documentation and validation infrastructure — the underlying mechanism (nox trace length commitment via Brakedown PCS) is inherent to the zheng proof system and requires no additional implementation beyond computing the step count and validating it fits within `τ`.
