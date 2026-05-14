---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Differentiable STARK Cost Surrogate

## Motivation

The actual STARK proving cost function is $\text{cost}(S) = 2^{\lceil \log_2(\max_t H_t(S)) \rceil}$ — a power-of-2 ceiling of the maximum table height. This function has two properties that make gradient-based optimization impossible:

1. **Non-differentiable**: the ceiling function has zero gradient everywhere it is defined, and undefined gradient at cliff transitions
2. **Bottleneck-driven**: only the tallest table matters; improving any other table has zero effect on cost

A learned smooth surrogate model approximates this function in a way that gradient-based optimization can exploit. The surrogate doesn't need to predict absolute cost accurately — it needs to correctly rank TASM sequences by cost. Pairwise ranking accuracy above 95% is sufficient for useful gradient guidance.

## Design

### Architecture: 1D CNN over TASM Sequences

The cost surrogate takes a TASM instruction sequence as input and predicts a scalar cost. A 1D convolutional architecture processes the sequence naturally:

```
Input: TASM sequence (padded to 128 instructions)
  → Embedding layer: instruction_id → 16-dim vector
  → Conv1D(kernel=5, filters=32, stride=1)
  → ReLU
  → Conv1D(kernel=3, filters=64, stride=2)
  → ReLU
  → Global average pool
  → Dense(32) → ReLU
  → Dense(1)   → scalar cost prediction
```

Parameters: approximately 15,000 field elements. Inference: ~20,000 TASM instructions in nn.trd. Still fast enough for interactive use.

### Why 1D CNN vs. Other Architectures

TASM is sequential — instruction at position $i$ influences tables based on the sequence of instructions before and after it. A 1D CNN captures local context (kernel size 5 = 5 consecutive instructions) efficiently. Compared to:

- **MLP over flat features**: ignores positional structure — misses patterns like "three hash instructions in a row dominate the Hash table"
- **RNN/LSTM**: captures global context but is harder to train with evolutionary methods
- **Transformer**: captures global attention but is larger (~50K params) and overkill for sequences of 128 instructions

The 1D CNN matches the problem's local-sequential structure at minimal parameter count.

### Training Data

Training pairs: (TASM sequence, actual proving time) from real proving runs.

```rust
// Collect training data:
fn collect_surrogate_data(programs: &[TasmProgram]) -> Vec<(TasmSequence, f64)> {
    programs.iter().map(|prog| {
        let time = trisha_prove_and_measure(prog);  // actual proving time
        (prog.instructions().take(128), time)
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
TIR → (differentiable compilation process) → TASM → surrogate → cost prediction
         ↑                                                           |
         ←←←←←←←←←←←←←← gradient ←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←←
```

The gradient of the surrogate with respect to TASM embedding vectors guides which instructions to change. This is not directly applicable to discrete TASM optimization (instructions are discrete, not continuous), but it guides two uses:

1. **Continuous relaxation**: Optimize over a soft probability distribution over instruction choices, then round to discrete instructions. Analogous to Gumbel-softmax in NLP.
2. **Gradient as ranking signal**: Use gradient direction to select which of several candidate TASM sequences to prefer, without requiring the gradient to exactly specify the optimal modification.

### Combination with Evolutionary Optimization

The surrogate's gradient is most useful as a complement to evolutionary search:

- **Gradients for smooth landscape**: The surrogate's gradient identifies the direction of local cost improvement — useful for fine-tuning near convergence.
- **Evolution for cliff-jumping**: Cliff discontinuities in the actual cost function are invisible to the surrogate (it smooths them). Evolution handles discrete jumps that gradients miss.

The hybrid strategy: run evolutionary optimization to reach a good region, then use the surrogate's gradient to fine-tune within that region before the next evolutionary step.

### Interaction with the Trace Predictor

The trace predictor (`trace-predictor.md`) predicts AET table heights from TIR features. The cost surrogate predicts STARK proving time from TASM sequences. Both are cost predictors, but at different levels:

- Trace predictor: TIR level, fast, approximate, guides TIR optimization
- Cost surrogate: TASM level, slower, more precise, guides TASM optimization

The trace predictor's output (predicted AET heights) can feed the cost surrogate as additional features, improving accuracy by providing the bottleneck table information that the TASM sequence alone might not reveal.

## Key Tradeoffs

**Smoothness vs. accuracy**: The surrogate is smooth by construction (differentiable). The actual cost function has cliff discontinuities. The surrogate cannot predict cliff crossings accurately — it will output a smooth prediction where the actual function has a step. For optimization near cliffs, the surrogate may suggest changes that look like 10% improvement but actually trigger a 2× cost increase. Gradient-guided optimization must be used cautiously near cliff regions.

**Distribution shift**: The surrogate is trained on a corpus of TASM programs. If the optimizer generates TASM sequences that are structurally different from the training corpus (e.g., very unusual instruction sequences from aggressive optimization), the surrogate may be inaccurate. The training corpus should include outputs from the optimizer itself (online training) to prevent this.

**Ranking vs. absolute accuracy**: Training for pairwise ranking rather than absolute cost prediction means the surrogate may have poor calibration — "program A costs 1.2, program B costs 1.5" may not mean anything in absolute terms. The surrogate's output should only be used for comparison, never for absolute cost claims.

**Field arithmetic limitations**: The surrogate operates in field arithmetic (to be compiled to TASM). Field arithmetic approximations of floating-point costs may limit accuracy. A hybrid where the surrogate runs in floating-point (outside Trident) and only the inference is re-implemented in field arithmetic may be more practical.

## Implementation Sketch

```trident
// cost_surrogate.trd
fn predict_cost(tasm_seq: [InstructionId; 128]) -> Field {
    // Embedding
    let embedded: Matrix<128, 16> = tasm_seq.map(|id| EMBEDDINGS[id]);
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
}
```

Training runs outside Trident (in Rust with actual proving times), producing weight values that are then embedded as constants in `cost_surrogate.trd`. The inference is field-native and provable. The training is floating-point and fast.
