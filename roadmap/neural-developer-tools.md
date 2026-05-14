---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Neural Developer Tools (Type Inference, Incremental Recompile, Program Synthesis)

**Related:** [[proof-cost-ide]] · [[proof-explorer]] · [[trident-repl]] · [[cost-surrogate]] · [[cybergraph]] · [[Atlas]] · [[soft3]] · [[warrior-cyber]] · [[nox]] · [[zheng]] · [reference/ir.md](../reference/ir.md)

## Vision

Neural developer tools make every programmer a world-class Trident developer. Type inference prevents the most expensive type choices. Incremental recompile makes the edit-compile-test loop milliseconds. Program synthesis generates correct-by-construction code from examples. Together, they change who can build for the cyber network: not just experts who know the intricacies of [[nox]] proof costs, but any developer with a clear specification.

In the far future, the REPL + program synthesis loop becomes the primary way to contribute to the [[cybergraph]]: a developer describes what they want (input/output examples), the synthesizer generates a Trident function, [[warrior-cyber]] proves it correct, and [[soft3]] submits it as a cyberlink. The barrier between "have an idea" and "publish a proved computation" collapses to a few seconds. Type inference and incremental recompile improve the local development experience before [[Atlas]] deployment. Program synthesis generates functions that compile to nox and can be immediately deployed — the synthesized code goes through TIR → [[nox]] → [[zheng]] automatically, and if the proof succeeds, the function is [[Atlas]]-deployable. The [[cybergraph]]'s existing proved functions become the synthesizer's training corpus, improving with every deployment.

## Motivation

Three developer-facing neural capabilities that make the Trident toolchain feel fast and intelligent. Each is independently useful; together they enable a workflow where the developer describes intent and the system fills in the mechanism.

## Design

### Neural Type Inference

Neural type inference operates on the AST — before TIR is generated. Different type representations for the same semantic value produce different nox trace lengths. A boolean stored as a full field element occupies the same trace space as any other field element. A boolean constrained to {0, 1} adds an extra constraint (`x * (x - 1) = 0`) but enables branch elimination elsewhere. The "right" choice depends on how the value is used downstream.

A Tree-LSTM operating on the Trident AST predicts type annotations that minimize expected nox trace length (proof cost):

- **Input**: Trident AST node + surrounding context (how the value is used, which TIR ops it reaches downstream)
- **Output**: (predicted_type, expected_cost_delta_vs_current_type) — cost expressed as estimated nox trace length delta
- **Architecture**: ~30K parameters, runs in microseconds on any AST

**IDE integration**: LSP-style type suggestions appear inline. If the programmer's chosen type is >2× more expensive than the model's recommendation, a warning appears with the suggested alternative and the trace-length cost difference. See [[proof-cost-ide]] for the IDE surface.

**Training**: every compilation pair (type_annotation, actual nox trace length) is a free training example. The model improves with the program corpus.

### Incremental Recompilation via Neural Diff

Incremental recompile works at the TIR level — the neural diff identifies which TIR nodes are invalidated by a source change, avoiding full TIR reconstruction and re-lowering to nox. The goal is to avoid re-lowering clean TIR regions to nox unnecessarily, since nox lowering (and the subsequent STARK witness generation) is the expensive step. Full recompilation is expensive for large programs. `trident watch` should recompile only the parts affected by a source edit. The challenge: dependency analysis for field arithmetic programs is non-trivial — a change to one function can propagate through the TIR in non-obvious ways.

A GNN operating on the TIR dependency graph (the same `TirGraph` with DataDep/ControlFlow/MemOrder edges, see [reference/ir.md](../reference/ir.md)) predicts which nodes are affected by a source edit:

- **Input**: (old TIR graph, edit location + new fragment) → graph with change delta marked
- **Output**: per-node probability of being affected by the change
- **Threshold**: conservative — any node with >5% probability of being affected is recompiled. Bias toward recall >99.9% over precision (never miss an affected node, but accept some over-recompilation).

**Target**: <100ms for single-line edits on programs up to 10K LOC. Full recompilation remains available; `trident watch` uses the neural diff by default.

**Correctness fallback**: if the neural diff causes a compilation that produces a different proof than full recompilation would, the discrepancy is caught at proof verification time. The system automatically falls back to full recompilation and logs the case as a training example.

### Fuzzing-Guided Program Synthesis

Program synthesis targets nox — the decoder outputs a TIR graph, which warrior-cyber immediately lowers to nox. TIR is the intermediate representation the decoder works with; nox is the actual compilation target and the zheng witness. Specification-first development: write input/output examples as field element pairs, let the system synthesize a Trident program satisfying the spec, then get a zheng proof of correctness.

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
- Encoder: processes the set of (input, output) examples → fixed-size latent representation (order-invariant, uses permutation-invariant pooling)
- Decoder: autoregressive over TIR operations (vocab: 54 ops across 4 tiers, see [reference/ir.md](../reference/ir.md)), max length 32; the TIR graph it produces is immediately lowered to nox by warrior-cyber
- Beam search: K=16 candidates

**Verification**: compile each TIR candidate to nox, execute on all spec examples via warrior-cyber. Match → generate zheng proof. No match → generate more candidates.

**Feedback loop**: failed candidates are negative training examples. Successful syntheses are added to the corpus. The synthesizer improves as more programs are verified.

**Scope**: targets short, pure functions (≤32 TIR nodes). Works well for: arithmetic formulas, field encoding/decoding, hash pre-images (when the hash is simple), lookup table functions. Explore synthesized programs interactively via [[trident-repl]] and [[proof-explorer]].

## Key Tradeoffs

**Type inference confidence**: the model's type suggestion is a prediction, not a proof. The programmer retains control — the IDE shows the suggestion with a nox trace cost estimate, not an error. For safety-critical code, the programmer should verify the cost model independently via `trident bench` (the scoreboard defined in [reference/cli.md](../reference/cli.md)) or the [[cost-surrogate]].

**Incremental recompilation correctness**: the neural TIR diff is not formally verified. It is a heuristic that works well empirically. The proof verification fallback is the safety net. This means incremental recompilation can be used freely — worst case is a slightly slower compile due to the fallback, not an incorrect result.

**Program synthesis search space**: the autoregressive TIR decoder generates programs in O(vocab^length) space. With 54 ops and length 32, the space is enormous. Beam search with K=16 covers a tiny fraction. The model must prioritize promising TIR candidates via learned priors from the training corpus. For programs outside the training distribution, synthesis may fail — the user falls back to manual implementation.

## Implementation Path

1. **Type inference**: instrument the compiler to record (type_choice, downstream nox trace length impact) pairs at AST→TIR boundary; train Tree-LSTM offline; integrate into LSP server via `trident lsp`
2. **Incremental recompile**: build TIR change delta representation on the `TirGraph`; train GNN on (old_TIR, edit, new_TIR, affected_nodes) from compilation history; integrate into `trident watch`
3. **Program synthesis**: build (input, output) example encoder; train TIR decoder on Trident standard library functions (free training data); expose via `synthesize` keyword in the language
