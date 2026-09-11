# Neural optimizer

The optional `neural` feature exposes a shared TIR graph encoder,
Transformer decoder, search and training library. A warrior implements
`trident::neural::Target` to supply its vocabulary, abstract execution
state, equivalence check, instruction costs and checkpoint directory.

Trident owns the model and training harness. Trisha owns the Triton
vocabulary, stack grammar, TASM verifier, AET costs and instruction
rewrites. The default compiler and warrior builds omit the model
backend dependencies; enable them with `--features neural`.

## Contract

`Target` provides:

- `vocabulary`: instruction encoding/decoding and the number of tokens;
- `grammar`: an abstract state that advances on tokens and emits validity,
  depth and type features;
- `verify_block`: checks a proposed instruction sequence against a baseline;
- `cost`: scores instructions in the target's own unit;
- `checkpoint_dir`: selects storage for this target's model weights;
- `augment`: proposes target-specific rewrites, rechecked by the core.

Token zero is the sequence boundary. Unrepresentable source instructions
and unknown output tokens invalidate the entire sequence. A training
pair cannot silently lose instructions during encoding.

The current model embeds depth in 65 buckets and consumes a
24-element target-defined type feature vector. A warrior must supply
these dimensions consistently. Model configuration takes an explicit
vocabulary size; there is no built-in ISA vocabulary in the core.

```rust,ignore
let target = /* warrior adapter */;
let result = trident::neural::compile(&target, &tir_ops, &baseline)?;
```

The result contains `assembly`, target `cost`, `valid_count`,
`total_count` and `neural`. Trisha retains its public compile wrappers,
whose result calls the assembly field `tasm_lines`.

## Model and inference

The default encoder is a four-layer GATv2 with model dimension 256 and
edge dimension 32. It consumes a graph with 59 node features and typed
edges for data dependencies, control flow and memory ordering.

The default decoder has six Transformer layers, eight attention heads,
inner dimension 1024 and maximum sequence length 256. It attends to
encoded graph nodes and receives abstract depth/type features from the
target. Parameter count depends on vocabulary size and model settings.

Beam search defaults to 32 candidates and 256 steps, with length
normalization and a repetition penalty. Target grammar states supply
conditioning features; the current search does not mask logits for
invalid instructions. Every returned candidate is checked by the
warrior's equivalence oracle and priced in its own unit.

The compiler chooses a verified candidate only when its cost is lower
than the baseline. Otherwise it returns the baseline and preserves the
observed candidate counters. With no checkpoint it returns the baseline
without initializing a numerical model. This fallback is reported as
`neural = false`; it is not a model improvement.

The Triton oracle executes supported straight-line blocks on concrete
field values. It is a bounded equivalence check, not a proof of arbitrary
program equivalence. No performance or correctness guarantee for a
trained model follows from model initialization or successful tests.

## Training

Supervised training uses teacher forcing and cross-entropy on TIR graph
and target-token pairs. Graph-to-tensor conversion and optimizer steps
are shared. Target grammar states supply depth and type features.

GFlowNet training samples candidate sequences, obtains validity and cost
from the target, computes a reward and returns trajectory-balance loss.
Selected-token log probabilities stay on the autodiff graph so this
loss reaches model parameters; reconstructing them from CPU scalars
would sever that gradient path. The caller applies optimizer updates.

Online-learning support includes replay persistence, validity and cost
metrics, finetuning triggers and a regression guard. Target-specific
instruction rewrites are owned by the warrior; the core verifies their
outputs before admitting augmented training pairs.

These are library APIs. There is no `trident train` command in the
current CLI. Trisha's checkpoint wrappers use `model/triton/v2`; old
`model/general/v2` weights are not automatically loaded into a different
vocabulary. Generic checkpoint functions take an explicit directory.

## Source and verification

| Responsibility | Source |
|---|---|
| Shared contract and compilation | `src/neural/target.rs`, `src/neural/mod.rs` |
| TIR graph, pairs, replay | `src/neural/data/` |
| Encoder, decoder, abstract-state sequence features | `src/neural/model/` |
| Beam search and verified ranking | `src/neural/inference/` |
| Supervised, GFlowNet, online, augmentation | `src/neural/training/` |
| Target adapter | Trisha `rs/neural/target.rs` |
| Triton vocabulary and grammar | Trisha `rs/neural/model/{vocab,grammar,grammar_tables}.rs` |
| Triton instruction rewrites | Trisha `rs/neural/augment_target.rs` |
| Triton block oracle and costs | Trisha `rs/cost/` |

Core tests exercise two different vocabularies without a Triton
implementation, including supervised training, search, target cost and
oracle selection, and a GFlowNet model-gradient regression. Trisha tests
exercise the same harness through its real target adapter.

```
cargo test --features neural --lib neural
cargo test -p trisha-rs --features neural --lib neural
```
