---
status: draft
author: mastercyb
area: runtime
planned: 64K
---

# Incremental Proof Updates

## Motivation

Reproving from scratch after every program change is a development workflow problem. A developer iterating on a function runs full prove cycles on every save. For large programs, a full prove cycle takes seconds to minutes. The developer's loop slows to the speed of proof generation.

Incremental proving solves this by reusing stable parts of the previous proof. When a program changes slightly — one branch modified, one constant updated — only the affected AET rows need to be recomputed. The unaffected rows, and the Brakedown commitment layers over them, are reused from the previous proof. The developer pays a fraction of the full proof cost for small changes.

## Design

### The `prove_delta` Interface

```trident
// Full proof for the initial version:
let proof_v1 = prove(program_v1, input);

// Program changes slightly — one function modified:
let proof_v2 = prove_delta(proof_v1, diff(program_v1, program_v2), input);
// Only re-proves affected AET rows
// Unaffected Brakedown layers are reused
```

`prove_delta` takes the previous proof, a program diff, and the input, and produces an updated proof that covers the new program. The semantic guarantee is identical to a full proof of `program_v2` — the result is a valid STARK proof for the new program, not a weaker partial proof.

### What Changes in the AET

A program diff affects a subset of the execution trace. The specific subset depends on the structure of the change:

- **Constant update** (e.g., a round constant changes): Affects only the Processor rows that load or use that constant. Other rows are unchanged.
- **Branch modification** (e.g., an `if` body changes): Affects only the Processor rows in that branch. Rows outside the branch are unchanged.
- **Function call change** (e.g., a callee is modified): Affects the caller's rows only at the call site and below. Rows before the call are unchanged.
- **Hash function change**: Affects Hash table rows for that hash call. Processor rows unrelated to the hash are unchanged.

The `diff` function computes which AET rows are affected by the program change. This is a static analysis over the program structure — no execution needed.

### Brakedown Layer Reuse

Brakedown is the commitment scheme used to commit to the AET. It is a hierarchical polynomial commitment: the AET is encoded as a polynomial, then committed level by level. When part of the AET changes, only the Brakedown tree layers covering the changed rows need to be recomputed. Layers covering unchanged rows are reused from the previous commitment.

For a change affecting 5% of the AET rows, approximately 5% of the Brakedown tree needs to be recomputed. The rest is copied from the previous proof. The variable cost scales with change size; the fixed cost remains one proof's worth of FRI queries and grinding.

### Proof Validity Guarantee

Incremental proofs are semantically equivalent to full proofs. A verifier who checks an incremental proof learns exactly the same thing as a verifier who checks a full proof of the same program: the program executed correctly on the given input. The verifier does not need to know whether the proof was generated incrementally.

This is possible because the STARK proof structure is compositional. The proof commits to the entire AET, including both changed and unchanged rows. The FRI protocol verifies the polynomial encoding of the full AET — the fact that some rows were carried over from a previous computation is invisible to the verifier.

### Development Workflow Integration

`prove_delta` enables `trident watch` mode:

```
$ trident watch my_program.tri
Watching for changes...

[12:34:01] Change detected: my_program.tri line 42
           Computing diff... 3 AET rows affected (0.2% of trace)
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

**Diff granularity**: The `diff` function operates at the TASM instruction level. For high-level source changes that generate complex TASM diffs (e.g., changing a loop bound), many rows may be affected. The incremental speedup is only significant when the TASM-level diff is small.

**Cache invalidation**: If the diff analysis is incorrect — it misses affected rows — the incremental proof may be invalid without the verifier detecting this. The diff analysis must be conservative: it is better to over-report affected rows (losing some speedup) than to under-report them (producing an incorrect proof). The implementation should err heavily on the side of over-reporting.

**First prove cost**: The first proof of a program is always a full proof. Incremental proving only helps from the second proof onward. For programs that change frequently, the amortized speedup is large. For programs proven once and deployed, there is no benefit.

**Memory for previous proof**: `prove_delta` must hold the previous proof in memory (or on disk) to reuse its Brakedown layers. For large programs with large proofs, this is significant memory. The runtime must manage the proof cache and evict old entries when memory is constrained.

## Implementation Sketch

`prove_delta` is implemented in the prover component (trisha):

```rust
// prover/incremental.rs
pub fn prove_delta(
    prev_proof: &StarkProof,
    program_diff: &TasmDiff,
    input: &ProgramInput,
) -> StarkProof {
    // Identify affected rows in each AET table
    let affected = compute_affected_rows(program_diff);

    // Reuse previous Brakedown layers for unaffected rows
    let mut aet = prev_proof.aet_snapshot();  // start from previous state
    
    // Re-execute only the affected portions of the trace
    execute_affected(program_diff, input, &mut aet, &affected);
    
    // Recompute only affected Brakedown tree nodes
    let commitment = update_brakedown_tree(prev_proof.commitment(), &aet, &affected);
    
    // Full FRI and grinding over the updated commitment (cannot be skipped)
    generate_fri_proof(commitment, &aet)
}
```

The `compute_affected_rows` function is the critical piece: it maps high-level program diff to low-level AET row ranges. This requires detailed knowledge of how each TASM instruction contributes to each AET table row. The implementation must be verified against the AET specification to ensure it never under-reports affected rows.
