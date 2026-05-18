---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Differentiable Proof Cost Surrogate

## Motivation

The actual [[nox]]/[[zheng]] proving cost function is `cost = trace_length + sum(jet_costs)` where jet costs are fixed per-jet (the hash jet via [[hemera]] is the most expensive) but trace_length is program-dependent. This function has properties that make gradient-based optimization difficult:

1. **Non-differentiable at boundaries**: trace_length changes in discrete steps (each [[nox]] reduction pattern application adds 1). No smooth gradient.
2. **Non-locally-structured**: the cost contribution of a single nox operation depends on which reduction patterns are triggered across the full program — not just locally.
3. **Cliff discontinuities**: if a [[zheng]] configuration has cliff-based pricing (e.g., next power-of-2 padding), reducing trace_length by 1 may have zero effect or 2× effect depending on where you are. Note: [[zheng]] uses Brakedown PCS (not FRI), so there is no FRI folding factor; cliff structure, if present, comes from padding choices in the [[zheng]] prover configuration.

A learned smooth surrogate approximates this cost function in a way that gradient-based optimization can exploit. The surrogate doesn't need to predict absolute cost accurately — it needs to correctly rank nox reduction sequences by [[nox]] proof cost. Pairwise ranking accuracy above 95% is sufficient for useful gradient guidance.

Related proposals: [[trace-predictor]], [[instruction-scheduling-nn]], [[compiler-ensemble]], [[algebraic-identity-explorer]].

## Vision

The differentiable cost surrogate is the gradient signal that drives the neural compiler. In the cyber ecosystem, where [[bbg]] charges real focus for every computation, lowering proof cost has direct economic value. The cost surrogate translates this economic pressure into a gradient that flows through the compiler's optimization decisions. Programs get cheaper automatically as the surrogate improves — no developer action required. The cyber network's economic incentives and the neural compiler's optimization objective are aligned: cheaper proofs = lower focus costs = more usage = more training data = better surrogate.

Stack integration: The surrogate's training data comes from [[warrior-cyber]] proving runs recorded in the [[cybergraph]]. Every (nox reduction sequence, actual [[nox]] trace length) pair is a training example, stored as a cyberlink. The surrogate is deployed as an [[Atlas]] package updated continuously as new data arrives — each update version-stamped and [[hemera]]-addressed, so every compiler installation can pin to a specific surrogate version and reproduce results exactly.

## Design

### Architecture: 1D CNN over Nox Reduction Sequences

The cost surrogate takes a nox reduction sequence as input and predicts a scalar proof cost. A 1D convolutional architecture processes the sequence naturally:

```
Input: nox reduction sequence (padded to 128 ops; 23 operation kinds: 18 patterns + 5 jets)
  → Embedding layer: op_id → 16-dim vector
  → Conv1D(kernel=5, filters=32, stride=1)
  → ReLU
  → Conv1D(kernel=3, filters=64, stride=2)
  → ReLU
  → Global average pool
  → Dense(32) → ReLU
  → Dense(1)   → scalar cost prediction (proxy for nox trace_length + jet_costs)
```

Parameters: approximately 15,000 field elements. Inference: ~20,000 [[nox]] steps in [[nn-trd]]. Still fast enough for interactive use.

The [[trace-predictor]] output (predicted trace_length + jet invocation counts) can be appended as additional input features, improving accuracy for programs with unusual jet usage patterns.

### Why 1D CNN vs. Other Architectures

Nox reduction sequences are sequential — the operation at position $i$ contributes to nox proof cost based on the surrounding operations (e.g., three adjacent hash-calling operations dominate jet costs). A 1D CNN captures local context (kernel size 5 = 5 consecutive operations) efficiently. Compared to:

- **MLP over flat features**: ignores positional structure — misses patterns like "three hemera-calling operations in a row dominate hash jet cost"
- **RNN/LSTM**: captures global context but is harder to train with evolutionary methods
- **Transformer**: captures global attention but is larger (~50K params) and overkill for sequences of 128 nox operations

The 1D CNN matches the problem's local-sequential structure at minimal parameter count.

### Training Data

Training pairs: (nox reduction sequence, actual [[zheng]] proving time) from real proving runs via [[warrior-cyber]].

```rust
// Collect training data:
fn collect_surrogate_data(programs: &[NoxSequence]) -> Vec<(NoxOpSequence, f64)> {
    programs.iter().map(|seq| {
        let time = warrior_cyber_prove_and_measure(seq);  // [[nox]] execution + [[zheng]] proof time
        (seq.ops().take(128), time)
    }).collect()
}
```

For pairwise ranking, the training objective is:

```
For each pair (S_A, S_B) where time(S_A) < time(S_B):
  loss = max(0, surrogate(S_A) - surrogate(S_B) + margin)
  // Hinge loss: surrogate must rank S_A below S_B
```

This directly optimizes for ranking accuracy rather than absolute cost prediction.

### Enabling Gradient-Based Optimization

Once the surrogate is trained, it enables gradient flow through cost:

```
nox sequence → (differentiable ordering process) → nox reduction ordering → surrogate → cost prediction
         ↑                                                                                      |
         ←←←←←←←←←←←←←←←←←←←←←←←←←←←←←← gradient ←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←
```

The gradient of the surrogate with respect to nox operation embedding vectors guides which operations to change or reorder. This is not directly applicable to discrete nox optimization (operations are discrete, not continuous), but it guides two uses:

1. **Continuous relaxation**: Optimize over a soft probability distribution over nox operation choices, then round to discrete operations. Analogous to Gumbel-softmax in NLP.
2. **Gradient as ranking signal**: Use gradient direction to select which of several candidate nox reduction sequences to prefer, without requiring the gradient to exactly specify the optimal modification.

### Combination with Evolutionary Optimization

The surrogate's gradient is most useful as a complement to evolutionary search:

- **Gradients for smooth landscape**: The surrogate's gradient identifies the direction of local cost improvement — useful for fine-tuning near convergence.
- **Evolution for cliff-jumping**: Cliff discontinuities in the actual cost function are invisible to the surrogate (it smooths them). Evolution handles discrete jumps that gradients miss.

The hybrid strategy: run evolutionary optimization to reach a good region, then use the surrogate's gradient to fine-tune within that region before the next evolutionary step.

### Interaction with the Trace Predictor

The [[trace-predictor]] predicts nox trace_length + jet invocation counts from nox graph features. The cost surrogate predicts zheng proving time from nox reduction sequences. Both are cost predictors, at different levels:

- Trace predictor: nox graph feature level, fast, approximate, guides high-level optimization
- Cost surrogate: nox reduction sequence level, slower, more precise, guides finer optimization

The trace predictor's output (predicted trace_length + jet counts) can feed the cost surrogate as additional input features, improving accuracy by providing bottleneck component information that the operation sequence alone might not reveal.

## Key Tradeoffs

**Smoothness vs. accuracy**: The surrogate is smooth by construction (differentiable). The actual [[nox]] proof cost function (`trace_length + jet_costs`) is piecewise linear with discrete jumps per pattern application. The surrogate cannot predict step transitions accurately. For optimization near cost thresholds, the surrogate may suggest changes that look like 10% improvement but actually trigger a larger cost increase. Note that [[zheng]] uses Brakedown PCS, not FRI — there are no FRI folding factor cliffs. Any cliff-like structure comes from [[zheng]] prover padding choices, which are configuration-dependent. Gradient-guided optimization should be used cautiously near threshold regions.

**Distribution shift**: The surrogate is trained on a corpus of nox reduction sequences. If the optimizer generates nox sequences that are structurally different from the training corpus (e.g., very unusual operation sequences from aggressive optimization), the surrogate may be inaccurate. The training corpus should include outputs from the [[compiler-ensemble]] itself (online training) to prevent this.

**Ranking vs. absolute accuracy**: Training for pairwise ranking rather than absolute cost prediction means the surrogate may have poor calibration — "program A costs 1.2, program B costs 1.5" may not mean anything in absolute terms. The surrogate's output should only be used for comparison, never for absolute cost claims.

**Field arithmetic limitations**: The surrogate inference operates in field arithmetic via [[nn-trd]] (producing a provable [[nox]] trace). Field arithmetic approximations of floating-point costs may limit accuracy. A hybrid where the surrogate trains in floating-point (outside Trident) and only inference is re-implemented in field arithmetic is the practical approach — matching the pattern used by all [[nn-trd]] networks.

## Implementation Sketch

```trident
// cost_surrogate.trd  (a std.nn network — see ../reference/stdlib.md §std.nn)
fn predict_cost(nox_ops: [OpId; 128]) -> Field {
    // Embedding: 23 nox operation kinds (18 patterns + 5 jets)
    let embedded: Matrix<128, 16> = nox_ops.map(|id| EMBEDDINGS[id]);
    // Conv layer 1 (kernel 5, filters 32)
    let conv1: Matrix<124, 32> = conv1d(embedded, CONV1_WEIGHTS, 5);
    let relu1 = relu_matrix(conv1);
    // Conv layer 2 (kernel 3, filters 64, stride 2)
    let conv2: Matrix<61, 64> = conv1d_strided(relu1, CONV2_WEIGHTS, 3, 2);
    let relu2 = relu_matrix(conv2);
    // Global average pool
    let pooled: Vector<64> = global_avg_pool(relu2);
    // Dense layers
    let h = relu(linear(DENSE1_W, DENSE1_B, pooled));
    linear_scalar(DENSE2_W, DENSE2_B, h)
    // output: scalar proxy for nox proof cost (trace_length + jet_costs)
}
```

Training runs outside Trident (in Rust with actual [[zheng]] proving times from [[warrior-cyber]]), producing weight values that are then embedded as constants in `cost_surrogate.trd`. The inference is field-native and provable via [[nox]]/[[zheng]]. The training is floating-point and fast.

The [[instruction-scheduling-nn]] and [[compiler-ensemble]] both use the surrogate's ranking signal to select among candidate orderings and specialist outputs.
