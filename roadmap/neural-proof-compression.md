---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Neural Proof Compression

**Related:** [[nn-prover-config]] · [[proof-carrying-code]]

## Motivation

zheng proofs are large — typically hundreds of kilobytes. For proof distribution (proof-carrying code), proof storage (verifiable computation archives), and on-chain verification (where calldata costs gas), proof size is a practical constraint. A zheng proof contains three main components: the Brakedown commitment (commitment matrix + hemera hashes), the sumcheck transcript (round-by-round challenge/response), and the opening queries. These components have different entropy profiles and different compressibility. Further reduction beyond zheng's existing optimizations requires a learned approach.

Neural proof compression uses a learned predictor to anticipate redundant elements in the proof. The verifier runs an identical predictor — elements the predictor got right are transmitted at 1 bit ("predicted correctly"). Elements the predictor got wrong are transmitted at full field-element size. If the predictor achieves 80% accuracy, the effective proof size is approximately 5× smaller. The full proof is still verified — compression is transport-layer only.

## Design

### The Compressor/Decompressor Architecture

```
PROVER SIDE:
  Generate full proof: [e1, e2, ..., eN]
  Run predictor on (program, public_inputs, proof_prefix): predict next element
  For each element e_i:
    pred = predictor(program, inputs, [e1..e_{i-1}])
    if pred == e_i:
      transmit bit: 0 (correct prediction)
    else:
      transmit bit: 1 (wrong prediction) + transmit full e_i

VERIFIER SIDE:
  Receive compressed stream
  Run identical predictor in sync with prover
  For each bit in stream:
    pred = predictor(program, inputs, [reconstructed_e1..e_{i-1}])
    if bit == 0: e_i = pred
    if bit == 1: e_i = next_full_element_from_stream
  Reconstruct full proof: [e1, e2, ..., eN]
  Verify full proof normally
```

The verifier reconstructs the complete proof before verification. The full STARK verification runs on the reconstructed proof. No changes to the STARK verifier.

### Predictor Architecture

A small autoregressive model that conditions on:
- The program being proved (encoded as TIR features or nox trace embedding)
- The public inputs
- All previously seen proof elements (the context window)

The predictor is aware of which proof component it is currently processing: Brakedown commitment elements are low-entropy (structured by the commitment matrix geometry), sumcheck transcript elements are moderately predictable (polynomial evaluations at challenge points), and opening query responses are the most entropic.

```
Input: [program_features (32), public_inputs (32), proof_context (last 16 elements), component_id (3)]
  → Dense(64) → ReLU
  → Dense(64) → ReLU
  → Dense(|Field|) → softmax over field elements
  // predicts the distribution over the next proof element
```

The `component_id` encodes which part of the proof structure is being predicted (Brakedown commitment / sumcheck round / opening query). The prediction is the mode of the softmax distribution — the most likely next element. If correct, it is transmitted as 1 bit.

For practical implementation, the field has $p \approx 2^{64}$ elements — a softmax over all of them is intractable. Instead, the predictor outputs a smaller vocabulary of predicted values (top-K candidates), and the transmission encodes whether the actual value is in the top-K and, if so, which one:

- If actual value in top-8: transmit 4 bits (3-bit index into top-8 + 1 flag bit)
- If not: transmit 1 flag bit + full 64-bit field element

Effective compression: if top-8 covers 80% of elements and each of those elements is compressed to 4 bits (vs. 64 bits), and remaining 20% are transmitted at 64 bits: average bits per element = $0.8 \times 4 + 0.2 \times 65 = 16.2$ bits vs. 64 bits. Compression ratio ≈ 4×.

### What the Predictor Learns

zheng proofs have structure by component:

- **Brakedown commitment**: The commitment matrix encodes the witness as a linear code. Matrix entries follow a structured distribution determined by the commitment dimensions. The predictor learns these matrix-level patterns and can anticipate many entries, especially in sparse witness regions.
- **Sumcheck transcript**: Each sumcheck round produces a low-degree polynomial evaluated at the verifier's challenge. If the predictor has seen enough rounds, it can interpolate the polynomial and predict subsequent evaluations — analogous to polynomial interpolation over a few known points.
- **Opening queries**: hemera hash outputs (used for Fiat-Shamir challenges) are high-entropy and not predictable. The predictor allocates more bits here. Opening responses at revealed positions are somewhat predictable given the commitment structure.

This component-aware structure lets the predictor allocate its bit budget effectively — spending few bits on structured commitment entries and full bits on hash-derived challenges.

### Determinism Requirement

Both the prover and verifier must run exactly the same predictor with exactly the same weights. Any divergence produces a reconstruction failure (the verifier reconstructs a different proof element, which causes zheng verification to fail). The predictor must be:

1. **Deterministic**: same inputs → same outputs, always
2. **Version-matched**: both sides must use the same model version
3. **Bit-exact**: no floating-point that may differ across hardware

Meeting requirement 3 means the predictor is implemented in field arithmetic (nn.trd) and compiled to nox patterns. The same compiled binary is used on both sides. This is where shipping the predictor as a Trident program becomes critical: the nox trace is bit-exact across all warrior-cyber backends (cpu/webgpu/metal).

### Compatibility

The compression protocol is entirely at the transport layer. warrior-cyber generates a complete zheng proof exactly as before. The compressor post-processes it. The verifier decompresses before verification. No changes to the zheng protocol, the sumcheck relation, or the Brakedown commitment scheme.

This means the compression layer can be added to any existing proof system — not just Trident. A Miden proof, an SP1 proof, or a Groth16 proof can be compressed with the same architecture, using a predictor trained on proofs from that system.

## Key Tradeoffs

**Predictor accuracy ceiling**: Even a perfect predictor for sumcheck polynomial evaluations (via interpolation) cannot predict hemera hash outputs used in Fiat-Shamir. The entropy of hash-derived challenges is a hard lower bound on compressed proof size. For programs with short nox traces where the sumcheck transcript dominates, high compression is achievable. For proofs where Brakedown opening queries dominate (many revealed positions), compression is limited.

**Predictor training data**: The predictor must be trained on a corpus of STARK proofs. Collecting millions of proofs is expensive. Transfer learning from proofs of simpler programs (which are faster to generate) to complex programs may reduce data requirements.

**Decompression overhead at verifier**: The verifier must run the predictor for each proof element to reconstruct the proof. This adds computation to the verifier — trading transmission cost for verification cost. For use cases where verification is cheap (smart contracts paying per byte received but computation is free), this trade is excellent. For use cases where verification computation is expensive (IoT devices), the trade may not be favorable.

**Model version management**: If the predictor is updated, all prover-verifier pairs must update simultaneously. A mismatch causes verification failures that are indistinguishable from invalid proofs. Strict versioning (predictor version embedded in the proof format) and a compatibility layer for old proof formats is essential.

## Implementation Sketch

```trident
// neural_proof_compression.trd
fn compress_proof(
    proof_elements: [Field; N],
    program_features: Vector<32>,
    public_inputs: Vector<32>,
) -> CompressedProof {
    let mut compressed = BitStream::new();
    let mut context = Vector::zero();

    for i in 0..N {
        let predicted_top8 = predict_top8(program_features, public_inputs, context);
        let actual = proof_elements[i];

        if let Some(idx) = predicted_top8.find(actual) {
            compressed.push_bit(0);  // predicted
            compressed.push_bits(idx, 3);  // 3-bit index
        } else {
            compressed.push_bit(1);  // not predicted
            compressed.push_field(actual);
        }

        context = update_context(context, actual);
    }

    compressed.finalize()
}
```

The compressor and decompressor are symmetric: the same predictor running on the same context produces the same top-8 candidates. The protocol is self-synchronizing as long as the predictor is deterministic and both sides start from the same state.
