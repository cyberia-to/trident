---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Neural Decompilation (TASM → TIR)

## Motivation

Compilation is a one-way process: TIR → TASM. Decompilation reverses it: TASM → TIR. Exact decompilation is NP-hard in general (the original TIR is one of many programs that compile to the same TASM). Neural decompilation produces a plausible TIR — not necessarily identical to the original, but structurally similar and semantically equivalent.

Why is this valuable? Three reasons:

1. **Learning from hand-written TASM**: Trident's baseline library (`baselines/triton/`) contains hand-optimized TASM written by experts who know Triton VM deeply. This expertise is locked in TASM form. Neural decompilation extracts it into TIR form, where it can train the compiler to produce better TIR for similar patterns.

2. **Cross-pollination**: Optimizations discovered for one program may apply to structurally similar programs. Decompiling an optimized TASM, modifying the TIR, and recompiling can transplant optimizations between programs.

3. **Round-trip testing**: Compiling TIR to TASM, decompiling back to TIR', and comparing TIR with TIR' tests the compiler's consistency. If TIR' is significantly different from TIR, the decompiler has found an alternative interpretation — which may be a better TIR representation.

## Design

### Model Architecture

Sequence-to-graph: the input is a TASM instruction sequence (a 1D sequence of tokens); the output is a TIR graph (a directed acyclic graph of operations).

```
Input: TASM sequence (up to 256 instructions)
  → Transformer encoder (4 layers, 128-dim, 4 heads)
  → Contextualized embeddings for each instruction

Output: TIR graph (autoregressive graph construction)
  → Graph decoder: at each step, predict
    - node type (TIR operation)
    - edge connections to existing nodes (which nodes are inputs to this node)
    - node attributes (constant values, field elements)
```

Parameters: ~50,000 field elements (50K params). Larger than other models in this system, but still small by neural network standards.

### Training Data

Every compilation generates a free training pair:
- Input: TASM output of the compilation
- Target: TIR input to the compilation

```rust
// Collect training pairs during normal compilation:
fn compile_with_recording(source: &TridentSource) -> (TasmProgram, TirGraph, TrainingSample) {
    let (tir, tasm) = compile(source);
    let sample = TrainingSample { tasm: tasm.clone(), tir: tir.clone() };
    (tasm, tir, sample)  // sample is free
}
```

With 100,000 compilations → 100,000 (TASM, TIR) pairs. This is the full training dataset. No labeling required.

### Correctness via Round-Trip

Decompiled TIR is a hypothesis. Validating it is cheap:
1. Recompile the decompiled TIR → TIR' → TASM'
2. Check TASM' ≡ TASM (using the STARK equivalence checker or Schwartz-Zippel)
3. If equivalent: the decompilation is valid (one valid TIR for this TASM)
4. If not: the decompilation diverged — use as training signal for the decompiler

The round-trip check is the quality gate. The decompiler aims to maximize the fraction of round-trips that succeed.

### Use Case 1: Learning from Hand-Written TASM

The baseline library contains expert-written TASM for hash functions, field operations, and cryptographic primitives. Neural decompilation converts these to TIR:

```
baselines/triton/hash.tasm → decompile → TIR_hash
```

The decompiled `TIR_hash` is then used to train the compiler: when the compiler generates TIR for a hash function, it should generate TIR similar to `TIR_hash`. The expert's knowledge, previously inaccessible in TASM form, becomes training data.

### Use Case 2: Cross-Pollination

If program A's optimized TASM has a pattern that would benefit program B:

```
Program A optimized TASM → decompile → TIR_A_opt
Identify optimization pattern in TIR_A_opt
Apply pattern to TIR_B → TIR_B_improved
Compile TIR_B_improved → TASM_B_improved
Verify: TASM_B_improved ≡ TASM_B (semantically)
```

Cross-pollination enables optimizations to spread across the program corpus without requiring manual intervention — the compiler learns general patterns from specific examples.

### Use Case 3: TIR → TASM → TIR' → TASM' Equivalence Testing

The double-compilation chain tests the compiler's consistency:

```
TIR → TASM → [decompile] → TIR' → TASM'
              TASM ≡ TASM'? (should be equivalent)
```

If TASM and TASM' are not equivalent, either:
- The decompiler found a genuinely different TIR for the same TASM (interesting)
- The compiler is inconsistent — same semantics, different TASM (compiler bug indicator)
- The decompiler made an error — the round-trip check caught it

All three cases are informative.

## Key Tradeoffs

**Ambiguity**: TASM is a lower-level representation than TIR. Many TIR programs compile to the same TASM. The decompiler must pick one plausible TIR — it cannot recover the unique original. This is inherent to decompilation and not a model deficiency.

**Model size**: At ~50K parameters, this is the largest neural component in the system. For field-native inference in `nn.trd`, 50K parameters requires ~100,000 TASM instructions for a single inference — expensive but feasible. For compiler-side use (outside Triton VM, in Rust), no size constraint applies.

**Graph decoder difficulty**: Autoregressive graph construction is harder to train than sequence-to-sequence models (which have more regular output structure). The decoder must learn to build valid TIR graphs (no cycles, valid type assignments) while also building correct ones (semantically matching the TASM input). The training objective is sequence-level cross-entropy over graph construction decisions.

**Training corpus diversity**: Training on compilations of the same codebase biases the decompiler toward that codebase's patterns. Hand-written TASM from the baseline library provides diversity — it represents patterns that the compiler would not naturally generate. Both sources should be included in training.

## Implementation Sketch

The decompiler is implemented as a Rust component (compilation tool, not Trident program):

```rust
// tools/decompiler/neural.rs
pub struct NeuralDecompiler {
    encoder: TransformerEncoder,  // ~20K params
    decoder: GraphDecoder,        // ~30K params
}

impl NeuralDecompiler {
    pub fn decompile(&self, tasm: &TasmProgram) -> TirGraph {
        let tasm_tokens = tokenize(tasm);
        let encoded = self.encoder.forward(&tasm_tokens);

        // Autoregressive graph construction:
        let mut graph = TirGraph::empty();
        loop {
            let (node_type, edges, attrs) = self.decoder.step(&encoded, &graph);
            if node_type == NodeType::End { break; }
            graph.add_node(node_type, edges, attrs);
        }
        graph
    }

    pub fn round_trip_valid(&self, tasm: &TasmProgram) -> bool {
        let decompiled_tir = self.decompile(tasm);
        let recompiled_tasm = compile_tir_to_tasm(&decompiled_tir);
        tasm_equivalent(tasm, &recompiled_tasm)
    }
}
```

Training runs outside Trident in standard floating-point (PyTorch or similar). The trained weights are converted to field elements and embedded in the deployed decompiler.
