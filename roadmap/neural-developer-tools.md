---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Neural Developer Tools (Type Inference, Incremental Recompile, Program Synthesis)

## Motivation

Three developer-facing neural capabilities that make the Trident toolchain feel fast and intelligent. Each is independently useful; together they enable a workflow where the developer describes intent and the system fills in the mechanism.

## Design

### Neural Type Inference

Different type representations for the same semantic value produce different AET profiles. A boolean stored as a full field element occupies the same trace space as any other field element. A boolean constrained to {0, 1} adds an extra constraint (`x * (x - 1) = 0`) but enables branch elimination elsewhere. The "right" choice depends on how the value is used.

A Tree-LSTM operating on the Trident AST predicts type annotations that minimize expected TASM cost:

- **Input**: Trident AST node + surrounding context (how the value is used, which tables it touches downstream)
- **Output**: (predicted_type, expected_cost_delta_vs_current_type)
- **Architecture**: ~30K parameters, runs in microseconds on any AST

**IDE integration**: LSP-style type suggestions appear inline. If the programmer's chosen type is >2× more expensive than the model's recommendation, a warning appears with the suggested alternative and the cost difference.

**Training**: every compilation pair (type_annotation, actual_AET_heights) is a free training example. The model improves with the program corpus.

### Incremental Recompilation via Neural Diff

Full recompilation is expensive for large programs. `trident watch` should recompile only the parts affected by a source edit. The challenge: dependency analysis for field arithmetic programs is non-trivial — a change to one function can propagate through the TIR in non-obvious ways.

A GNN operating on the TIR dependency graph predicts which nodes are affected by a source edit:

- **Input**: (old TIR graph, edit location + new fragment) → graph with change delta marked
- **Output**: per-node probability of being affected by the change
- **Threshold**: conservative — any node with >5% probability of being affected is recompiled. Bias toward recall >99.9% over precision (never miss an affected node, but accept some over-recompilation).

**Target**: <100ms for single-line edits on programs up to 10K LOC. Full recompilation remains available; `trident watch` uses the neural diff by default.

**Correctness fallback**: if the neural diff causes a compilation that produces a different proof than full recompilation would, the discrepancy is caught at proof verification time. The system automatically falls back to full recompilation and logs the case as a training example.

### Fuzzing-Guided Program Synthesis

Specification-first development: write input/output examples as field element pairs, let the system synthesize a Trident program satisfying the spec, then get a STARK proof of correctness.

```trident
synthesize fn mystery(x: Field) -> Field from {
    (2, 8),      // mystery(2)  = 8
    (3, 27),     // mystery(3)  = 27
    (4, 64),     // mystery(4)  = 64
    (5, 125),    // mystery(5)  = 125
}
// synthesized: fn mystery(x: Field) -> Field { x * x * x }
```

**Architecture**: seq2seq model
- Encoder: processes the set of (input, output) examples → fixed-size latent representation (order-invariant, uses a permutation-invariant pooling)
- Decoder: autoregressive over TIR operations (vocab ~54 ops, max length 32)
- Beam search: K=16 candidates

**Verification**: compile each candidate, execute on all spec examples. Match → generate STARK proof. No match → generate more candidates.

**Feedback loop**: failed candidates are negative training examples. Successful syntheses are added to the corpus. The synthesizer improves as more programs are verified.

**Scope**: targets short, pure functions (≤32 TIR nodes). Works well for: arithmetic formulas, field encoding/decoding, hash pre-images (when the hash is simple), lookup table functions.

## Key Tradeoffs

**Type inference confidence**: the model's type suggestion is a prediction, not a proof. The programmer retains control — the IDE shows the suggestion with a cost estimate, not an error. For safety-critical code, the programmer should verify the cost model independently via `trident bench`.

**Incremental recompilation correctness**: the neural diff is not formally verified. It is a heuristic that works well empirically. The proof verification fallback is the safety net. This means incremental recompilation can be used freely — worst case is a slightly slower compile due to the fallback, not an incorrect result.

**Program synthesis search space**: the autoregressive decoder generates programs in O(vocab^length) space. For length 32, this is astronomically large. Beam search with K=16 covers a tiny fraction. The model must prioritize promising candidates via learned priors from the training corpus. For programs outside the training distribution, synthesis may fail — the user falls back to manual implementation.

## Implementation Path

1. **Type inference**: instrument the compiler to record (type_choice, downstream_AET_impact) pairs; train Tree-LSTM offline; integrate into LSP server via `trident lsp`
2. **Incremental recompile**: build TIR change delta representation; train GNN on (old_TIR, edit, new_TIR, affected_nodes) from compilation history; integrate into `trident watch`
3. **Program synthesis**: build (input, output) example encoder; train decoder on Trident standard library functions (free training data); expose via `synthesize` keyword in the language
