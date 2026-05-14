---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Field-Native Neural Network Library (nn.trd)

## Motivation

Every neural technique on this roadmap — the algebraic identity explorer's GFlowNet proposer, the trace predictor, the cost surrogate, the instruction scheduler — is a neural network. These networks must run inside Trident to be provable. Running inside Trident means operating over Goldilocks field arithmetic, with no floating point, no signed integers natively, and no smooth activation functions.

`nn.trd` is the foundation: a Trident library implementing neural network primitives entirely in field arithmetic. Building it first enables every subsequent neural technique to be compiled to TASM, proven on Triton VM, and progressively self-optimized by the algebraic identity explorer.

A neural network whose every inference produces a valid Triton VM trace is a world-first: provable AI.

## Design

### Library Structure

```
nn.trd
├── field_signed.trd      — signed integer convention
├── field_fixed.trd       — fixed-point arithmetic
├── linalg.trd            — matrix and vector operations
├── activations.trd       — field-native activation functions
├── layers.trd            — linear layers, normalization, residual
├── loss.trd              — loss functions
└── inference.trd         — forward pass orchestration
```

### Signed Field Arithmetic (`field_signed.trd`)

Goldilocks has no native signed integers. The convention: $x > p/2$ means $x$ represents the negative value $x - p$. All signed arithmetic follows from this convention.

```trident
// signed addition: works natively (field addition wraps correctly)
fn signed_add(a: Field, b: Field) -> Field { a + b }

// signed comparison:
fn signed_lt(a: Field, b: Field) -> bool {
    let norm_a = if a > P_HALF { a - P } else { a };
    let norm_b = if b > P_HALF { b - P } else { b };
    norm_a < norm_b
}
```

### Fixed-Point Arithmetic (`field_fixed.trd`)

Fractions are represented as $\text{value} \times 2^{SCALE}$ where SCALE is a compile-time constant (typically 16 or 24).

```trident
const SCALE: Field = 1 << 16;
const SCALE_INV: Field = invert(SCALE);  // computed at compile time

fn fixed_mul(a: Field, b: Field) -> Field {
    // a * b in fixed-point: result has 2× scale factor, need to reduce
    (a * b) * SCALE_INV  // one extra multiply for scale correction
}
```

For performance-critical paths, the compiler can fuse the scale correction into adjacent operations (e.g., `fixed_mul` followed by `fixed_add` can cancel one scale correction). The algebraic identity explorer will discover these fusions automatically.

### Activation Functions (`activations.trd`)

No floating-point means all activations are polynomial or rational approximations:

```trident
// GELU approximation: 0.5x(1 + tanh(√(2/π)(x + 0.044715x³)))
// Implemented as degree-5 polynomial approximation in fixed-point:
fn gelu(x: Field) -> Field {
    // Padé approximant coefficients (precomputed constants):
    let x2 = fixed_mul(x, x);
    let x3 = fixed_mul(x2, x);
    x * (C0 + C1 * x2 + C2 * x3)  // truncated approximation
}

// ReLU: conditional — implemented as (x > 0) ? x : 0
fn relu(x: Field) -> Field {
    if signed_lt(ZERO, x) { x } else { ZERO }
}

// tanh: Padé approximant P(x)/Q(x) for |x| ≤ 4, clamped outside
fn tanh(x: Field) -> Field {
    // NOTE: clamp to ±20 before calling tanh in MSL kernels (GPU NaN bug)
    let clamped = clamp(x, NEG_20, POS_20);
    let x2 = fixed_mul(clamped, clamped);
    (clamped * (P1 + P2 * x2)) * invert(Q1 + Q2 * x2 + x2 * x2)
}
```

### Linear Layers (`layers.trd`)

A linear layer is a matrix multiply plus bias addition:

```trident
fn linear<const IN: Field, const OUT: Field>(
    weights: Matrix<OUT, IN>,
    bias: Vector<OUT>,
    input: Vector<IN>,
) -> Vector<OUT>
  cost [processor: OUT * IN + OUT..OUT * IN * 2 + OUT]
{
    matmul(weights, input) + bias
}
```

The cost annotation is a dependent bound — scales with `OUT * IN` (the matrix multiply cost).

### Size and Performance

A 3-layer MLP with 64-wide hidden layers:
- ~500 lines of Trident source across all modules
- Compiles to ~2,000 TASM instructions
- Inference: one Triton VM execution
- Proof: one STARK over the inference trace (~3,000 Processor rows, no Hash table)

The inference is provable in a single STARK. The proof certifies that the neural network computed this specific output from this specific input using these specific weights.

### What Provable Inference Enables

- **Verifiable AI**: Any party can verify that a neural network produced a specific output without re-running the network. Relevant for AI-assisted decisions in high-stakes contexts.
- **ZK inference**: With `zk fn` wrapping, the input can be private. The proof certifies the output is consistent with some valid input to this network, without revealing the input.
- **Self-bootstrapping**: The algebraic identity explorer uses `nn.trd` for its GFlowNet proposer. The proposer is a provable neural network. Its inference is proven alongside the identities it discovers.

## Key Tradeoffs

**Approximation error**: Polynomial activation approximations introduce error. For inference-only applications, this error is typically negligible (< 0.1% relative). For training, accumulated approximation error may prevent convergence. Calibrating approximation degree against cost and accuracy is an empirical tuning problem.

**Scale factor choice**: The fixed-point scale factor determines the precision-cost tradeoff. Higher scale (more precision) requires larger multiplications (higher cost) and more careful overflow management. Scale 16 is a reasonable default; scale 24 gives better precision for deeper networks.

**Matrix multiply cost**: Matrix multiply is $O(m \times n \times k)$ field multiplications. For the 64-wide hidden layers in the target MLP (~2,000 TASM instructions), this is manageable. For larger networks (256-wide, 8 layers), the Processor table may exceed practical bounds. Large models require the NTT auto-vectorization pass (Pass 7) to convert matmul to NTT convolution.

**No backpropagation**: `nn.trd` is an inference library. Training uses the evolutionary method (separate proposal `evolutionary-training.md`). Gradient-based training in field arithmetic requires finite-difference approximation, which is noisy and expensive. Evolution is the preferred training method for field-native networks.

## Implementation Sketch

All modules in `nn.trd` are standard Trident source. The only non-obvious implementation detail is the Padé approximant constants:

```trident
// activations.trd — tanh Padé coefficients in fixed-point:
const TANH_P1: Field = fixed_const(1.0);           // 1.0 in fixed-point
const TANH_P2: Field = fixed_const(0.16667);       // 1/6 approximation
const TANH_Q1: Field = fixed_const(1.0);
const TANH_Q2: Field = fixed_const(0.5);
// Coefficients computed at compile time via field arithmetic:
// invert(6) computed by compiler, stored as field constant — zero runtime cost
```

`nn.trd` is the first deliverable in the AI roadmap — it unblocks every other neural technique. Target: implement and test a verifiable 3-layer MLP before 128K milestone.
