---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Neural Decompilation (nox trace → TIR)

**Related:** [[learned-peephole]] · [[neural-theorem-prover]] · [[backend-transfer-learning]] · [[cybergraph]] · [[soft3]] · [[hemera]] · [[zheng]] · [reference/ir.md](../reference/ir.md)

## Vision

Nox trace → TIR reconstruction is the reverse-engineering of proven computation. In the [[cybergraph]], every answer cyberlink carries a nox trace (compressed, via the [[zheng]] proof). Neural decompilation lets any agent read a proof, reconstruct the TIR, and extract the computation's structure — without access to the original Trident source. This enables cross-program learning at the ecosystem level: an AI agent studying the [[cybergraph]]'s proof corpus learns how programs are structured by reconstructing their TIR from traces. The knowledge graph doesn't just store results — it stores recoverable computational structure.

[[soft3]]'s `query(proof_cid)` returns the [[zheng]] proof. The decompilation model processes this to produce TIR. The reconstructed TIR is submitted back as a particle ([[hemera]]-addressed), creating a cyberlink from the proof to the reconstructed structure. Future queries can retrieve this reconstruction without re-running the model. The [[cybergraph]] accumulates computational intelligence with every proof that passes through it.

## Motivation

Compilation is a one-way process: TIR → nox patterns (trace). Decompilation reverses it: nox trace → TIR. Exact decompilation is NP-hard in general (the original TIR is one of many programs that produce the same nox trace). Neural decompilation produces a plausible TIR — not necessarily identical to the original, but structurally similar and semantically equivalent.

Why is this valuable? Three reasons:

1. **Learning from hand-written nox traces**: Trident's baseline library (`baselines/triton/`) contains hand-optimized nox instruction sequences written by experts who know nox deeply. This expertise is locked in trace form. Neural decompilation extracts it into TIR form, where it can train the compiler to produce better TIR for similar patterns.

2. **Cross-pollination**: Optimizations discovered for one program may apply to structurally similar programs. Decompiling an optimized nox trace, modifying the TIR, and recompiling can transplant optimizations between programs.

3. **Round-trip testing**: Compiling TIR to a nox trace, decompiling back to TIR', and comparing TIR with TIR' tests the compiler's consistency. If TIR' is significantly different from TIR, the decompiler has found an alternative interpretation — which may be a better TIR representation.

## Design

### Model Architecture

Sequence-to-graph: the input is a nox instruction sequence (a 1D sequence of tokens from nox's 16 patterns + 1 hint + 5 jets); the output is a TIR graph (a directed acyclic graph of 54 ops across 4 tiers, as defined in [reference/ir.md](../reference/ir.md)).

```
Input: nox instruction sequence (up to 256 instructions)
  → Transformer encoder (4 layers, 128-dim, 4 heads)
  → Contextualized embeddings for each instruction

Output: TIR graph (autoregressive graph construction)
  → Graph decoder: at each step, predict
    - node type (TIR operation, one of 54 ops in 4 tiers)
    - edge connections to existing nodes (which nodes are inputs to this node)
    - node attributes (constant values, field elements)
```

Parameters: ~50,000 field elements (50K params). Larger than other models in this system, but still small by neural network standards.

### Training Data

Every compilation generates a free training pair:
- Input: nox instruction sequence output of the compilation
- Target: TIR input to the compilation

```rust
// Collect training pairs during normal compilation:
fn compile_with_recording(source: &TridentSource) -> (NoxTrace, TirGraph, TrainingSample) {
    let (tir, trace) = compile(source);
    let sample = TrainingSample { trace: trace.clone(), tir: tir.clone() };
    (trace, tir, sample)  // sample is free
}
```

With 100,000 compilations → 100,000 (nox trace, TIR) pairs. This is the full training dataset. No labeling required.

### Correctness via Round-Trip

Decompiled TIR is a hypothesis. Validating it is cheap:
1. Recompile the decompiled TIR → TIR' → nox trace'
2. Check trace' ≡ trace (using the equivalence checker or Schwartz-Zippel)
3. If equivalent: the decompilation is valid (one valid TIR for this nox trace)
4. If not: the decompilation diverged — use as training signal for the decompiler

The round-trip check is the quality gate. The decompiler aims to maximize the fraction of round-trips that succeed.

### Use Case 1: Learning from Hand-Written nox Traces

The baseline library contains expert-written nox instruction sequences for hash functions, field operations, and cryptographic primitives. Neural decompilation converts these to TIR:

```
baselines/triton/hash.nox → decompile → TIR_hash
```

The decompiled `TIR_hash` is then used to train the compiler: when the compiler generates TIR for a hash function, it should generate TIR similar to `TIR_hash`. The expert's knowledge, previously inaccessible in trace form, becomes training data.

### Use Case 2: Cross-Pollination

If program A's optimized nox trace has a pattern that would benefit program B:

```
Program A optimized trace → decompile → TIR_A_opt
Identify optimization pattern in TIR_A_opt
Apply pattern to TIR_B → TIR_B_improved
Compile TIR_B_improved → trace_B_improved
Verify: trace_B_improved ≡ trace_B (semantically)
```

Cross-pollination enables optimizations to spread across the program corpus without requiring manual intervention — the compiler learns general patterns from specific examples.

### Use Case 3: TIR → trace → TIR' → trace' Equivalence Testing

The double-compilation chain tests the compiler's consistency:

```
TIR → trace → [decompile] → TIR' → trace'
               trace ≡ trace'? (should be equivalent)
```

If trace and trace' are not equivalent, either:
- The decompiler found a genuinely different TIR for the same nox trace (interesting)
- The compiler is inconsistent — same semantics, different trace (compiler bug indicator)
- The decompiler made an error — the round-trip check caught it

All three cases are informative.

## Key Tradeoffs

**Ambiguity**: A nox trace is a lower-level representation than TIR. Many TIR programs compile to the same nox trace. The decompiler must pick one plausible TIR — it cannot recover the unique original. This is inherent to decompilation and not a model deficiency.

**Model size**: At ~50K parameters, this is the largest neural component in the system. For field-native inference in `nn.trd`, 50K parameters requires ~100,000 nox instructions for a single inference — expensive but feasible. For compiler-side use (in Rust, outside nox), no size constraint applies.

**Graph decoder difficulty**: Autoregressive graph construction is harder to train than sequence-to-sequence models (which have more regular output structure). The decoder must learn to build valid TIR graphs (no cycles, valid type assignments) while also building correct ones (semantically matching the nox trace input). The training objective is sequence-level cross-entropy over graph construction decisions.

**Training corpus diversity**: Training on compilations of the same codebase biases the decompiler toward that codebase's patterns. Hand-written nox traces from the baseline library provide diversity — they represent patterns the compiler would not naturally generate. Both sources should be included in training.

## Implementation Sketch

The decompiler is implemented as a Rust component (compilation tool, not Trident program):

```rust
// tools/decompiler/neural.rs
pub struct NeuralDecompiler {
    encoder: TransformerEncoder,  // ~20K params
    decoder: GraphDecoder,        // ~30K params
}

impl NeuralDecompiler {
    pub fn decompile(&self, trace: &NoxTrace) -> TirGraph {
        let tokens = tokenize(trace);  // nox patterns: 16 patterns + 1 hint + 5 jets
        let encoded = self.encoder.forward(&tokens);

        // Autoregressive graph construction:
        let mut graph = TirGraph::empty();
        loop {
            let (node_type, edges, attrs) = self.decoder.step(&encoded, &graph);
            if node_type == NodeType::End { break; }
            graph.add_node(node_type, edges, attrs);
        }
        graph
    }

    pub fn round_trip_valid(&self, trace: &NoxTrace) -> bool {
        let decompiled_tir = self.decompile(trace);
        let recompiled_trace = compile_tir_to_nox(&decompiled_tir);
        nox_equivalent(trace, &recompiled_trace)
    }
}
```

Training runs outside Trident in standard floating-point (PyTorch or similar). The trained weights are converted to field elements and embedded in the deployed decompiler.
