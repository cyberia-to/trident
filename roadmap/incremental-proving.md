---
status: draft
author: mastercyb
area: runtime
planned: 64K
---

# Incremental Proof Updates

**Related:** [[lazy-proving]] · [[neural-developer-tools]] · [[proof-cost-ide]] · [[CORE]] · [[zheng]] · [[cybergraph]] · [[bbg]] · [[warrior-cyber]]

## Vision

The [[CORE]] spec — 16 reduction patterns, [[bbg]] state machine, focus dynamics — is written in Trident and proved by [[zheng]] on every change. When a single reduction pattern's implementation changes, incremental proving re-proves only that pattern's constraint, not the entire [[CORE]] spec. The spec re-proves itself in seconds instead of hours.

This enables continuous formal verification: every commit to the CORE repo triggers an incremental proof update via `trident watch`, and the spec's proof is a living artifact in the [[cybergraph]], always current. The proof lives as a cyberlink from the source CID to the verified CORE spec particle — anyone querying `ask(verify, core_spec_cid)` gets the current proof back instantly, cached in the graph.

[[bbg]] checkpoints track which segments of long computations have been proved. Incremental proving lets [[warrior-cyber]] resume from checkpoints, reusing Brakedown layers that haven't changed. The checkpoint particles are stored in the [[cybergraph]] — each is a [[hemera]]-addressed node representing a proven segment of the computation. On a change, [[warrior-cyber]] loads the relevant checkpoint particle, restores its Brakedown state, and reproves only the affected delta. The development loop closes: edit → 47ms incremental proof → updated [[cybergraph]] artifact.

## Motivation

Reproving from scratch after every program change is a development workflow problem. A developer iterating on a function runs full prove cycles on every save. For large programs, a full prove cycle takes seconds to minutes. The developer's loop slows to the speed of proof generation.

Incremental proving solves this by reusing stable parts of the previous proof. When a program changes slightly — one branch modified, one constant updated — only the affected nox trace segments need to be recomputed. The unaffected segments, and the Brakedown PCS commitment layers over them, are reused from the previous proof. The developer pays a fraction of the full proof cost for small changes.

## Design

### The `prove_delta` Interface

```trident
// Full proof for the initial version:
let proof_v1 = prove(program_v1, input);

// Program changes slightly — one function modified:
let proof_v2 = prove_delta(proof_v1, diff(program_v1, program_v2), input);
// Only re-proves affected nox trace segments
// Unaffected Brakedown layers are reused
```

`prove_delta` takes the previous proof, a program diff, and the input, and produces an updated proof that covers the new program. The semantic guarantee is identical to a full proof of `program_v2` — the result is a valid zheng proof of the new nox trace, not a weaker partial proof.

### What Changes in the nox Trace

A program diff affects a subset of the nox execution trace. The specific subset depends on the structure of the change:

- **Constant update** (e.g., a round constant changes): Affects only the nox reduction steps that load or use that constant. Other steps are unchanged.
- **Branch modification** (e.g., an `if` body changes): Affects only the reduction steps in that branch. Steps outside the branch are unchanged.
- **Function call change** (e.g., a callee is modified): Affects the nox trace only at the call site and below. Steps before the call are unchanged.
- **Jet call change** (e.g., a hash or merkle_step call path changes): Affects only the nox trace steps for that jet invocation.

The `diff` function computes which nox trace segments are affected by the program change. This is a static analysis over the nox pattern structure — no execution needed.

### Brakedown Layer Reuse

Brakedown PCS is the commitment scheme used to commit to the nox trace (zheng uses Brakedown PCS, not FRI). It is a hierarchical linear code commitment: the trace polynomial is encoded, then committed level by level. When part of the nox trace changes, only the Brakedown tree layers covering the changed segments need to be recomputed. Layers covering unchanged segments are reused from the previous commitment.

For a change affecting 5% of the nox trace, approximately 5% of the Brakedown tree needs to be recomputed. The rest is copied from the previous proof. The variable cost scales with change size; the fixed cost remains one proof's worth of sumcheck and grinding — no FRI queries (zheng uses Brakedown PCS, not FRI). See [[lazy-proving]] for how `defer_proof` blocks naturally scope incremental reuse.

### Proof Validity Guarantee

Incremental proofs are semantically equivalent to full proofs. A verifier who checks an incremental proof learns exactly the same thing as a verifier who checks a full proof of the same program: the program executed correctly on the given input. The verifier does not need to know whether the proof was generated incrementally.

This is possible because the zheng proof structure is compositional. The proof commits to the entire nox trace via Brakedown PCS, including both changed and unchanged segments. The sumcheck verifies the polynomial encoding of the full trace — the fact that some segments were carried over from a previous computation is invisible to the verifier.

### Development Workflow Integration

`prove_delta` enables `trident watch` mode:

```
$ trident watch my_program.tri
Watching for changes...

[12:34:01] Change detected: my_program.tri line 42
           Computing diff... 3 nox trace segments affected (0.2% of trace)
           Re-proving changed rows...
           Proof updated in 47ms (full prove: 8.3s, 175× speedup)
           All constraints satisfied.
```

For programs where the developer iterates on a small part (common in cryptographic circuit development), the watch mode's speedup approaches the ratio of full trace size to changed trace size.

### Incremental Proving for Version Control

`prove_delta` composes with version control. Each commit to a program in a provable codebase produces a new proof. `prove_delta` carries the proof forward through the commit history:

```
commit A → proof_A (full)
commit B → proof_B = prove_delta(proof_A, diff(A, B), input)
commit C → proof_C = prove_delta(proof_B, diff(B, C), input)
```

The proof chain has the property that each proof is valid independently. The chain also provides a record of how the proof evolved — useful for auditing changes to provable code.

## Key Tradeoffs

**Diff granularity**: The `diff` function operates at the nox pattern level. For high-level source changes that generate large nox trace diffs (e.g., changing a loop bound), many trace segments may be affected. The incremental speedup is only significant when the nox-level diff is small.

**Cache invalidation**: If the diff analysis is incorrect — it misses affected rows — the incremental proof may be invalid without the verifier detecting this. The diff analysis must be conservative: it is better to over-report affected rows (losing some speedup) than to under-report them (producing an incorrect proof). The implementation should err heavily on the side of over-reporting.

**First prove cost**: The first proof of a program is always a full proof. Incremental proving only helps from the second proof onward. For programs that change frequently, the amortized speedup is large. For programs proven once and deployed, there is no benefit.

**Memory for previous proof**: `prove_delta` must hold the previous proof in memory (or on disk) to reuse its Brakedown layers. For large programs with large proofs, this is significant memory. The runtime must manage the proof cache and evict old entries when memory is constrained.

## Implementation Sketch

`prove_delta` is implemented in the prover component (trisha):

```rust
// prover/incremental.rs
pub fn prove_delta(
    prev_proof: &ZhengProof,
    program_diff: &NoxDiff,
    input: &ProgramInput,
) -> ZhengProof {
    // Identify affected segments in the nox trace
    let affected = compute_affected_segments(program_diff);

    // Reuse previous Brakedown PCS layers for unaffected trace segments
    let mut trace = prev_proof.trace_snapshot();  // start from previous nox trace
    
    // Re-execute only the affected portions of the nox trace
    execute_affected_nox(program_diff, input, &mut trace, &affected);
    
    // Recompute only affected Brakedown tree nodes (no FRI — Brakedown PCS only)
    let commitment = update_brakedown_tree(prev_proof.commitment(), &trace, &affected);
    
    // Full sumcheck and grinding over the updated commitment
    generate_zheng_proof(commitment, &trace)
}
```

The `compute_affected_segments` function is the critical piece: it maps high-level program diff to low-level nox trace segment ranges. This requires detailed knowledge of how each nox pattern (and jet invocation) contributes to the trace. The implementation must err on the side of over-reporting affected segments — a missed segment produces an incorrect proof that may pass verification.
