---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Transfer Learning Across Proof Backends

## Motivation

When Trident adds a new proving backend — Miden VM, SP1, OpenVM, or a future system — the neural compiler must learn to optimize for it. Training from scratch requires thousands of (program, proving time) pairs and weeks of compute. Transfer learning reuses the knowledge already encoded in the Triton VM neural compiler, reducing new-backend training data requirements to ~10% of the original.

The key insight: TIR-level optimization patterns generalize across backends. The IR is the same — only the lowering to the specific instruction set changes.

## Design

### Split Architecture

The neural compiler is structured as two separable components:

```
TIR graph → [SHARED ENCODER] → latent representation → [BACKEND DECODER] → TASM/MASM/...
```

The **shared encoder** learns the general structure of Trident programs at the IR level: control flow patterns, data dependencies, loop structures, field arithmetic idioms. This knowledge is backend-agnostic.

The **backend decoder** learns the specific instruction set, table structure, and cost model of a particular proving system. This is the only part that must be retrained for a new backend.

### Transfer Protocol

1. **Train on Triton VM** (full training, ~100K programs): shared encoder + Triton decoder jointly optimized. Encoder learns rich TIR representations.

2. **Freeze encoder** for new backend: encoder weights are fixed. Only the decoder is trainable.

3. **Train decoder for new backend** (~10K programs): much faster — decoder is a shallow network, and the pre-trained encoder provides high-quality features immediately. Cold start is solved.

4. **Optional fine-tuning**: after decoder converges, unfreeze encoder and fine-tune jointly with a small learning rate. Adjusts encoder features toward new backend's cost landscape.

### Algebraic Identity Transfer

Layer 0-1 algebraic identities (pure arithmetic: `push 0; add` → ∅, `push 2^32; mul` → shift) transfer directly — they hold in any field-based instruction set with the same Goldilocks arithmetic.

Layer 2+ identities are field-specific but may be architecture-dependent. The algebraic identity explorer's validation pipeline (§0 in neural compiler) re-validates each identity against the new backend's execution semantics. Identities that survive validation are added to the new backend's rule database immediately — no re-discovery needed.

### Cost Model Transfer

AET table heights are Triton-specific. Miden uses a different table structure; SP1 uses a different constraint system entirely. The cost surrogate must be retrained per backend.

However, the relative ordering of TIR patterns by "costliness" is partially preserved across backends — hash-heavy programs are expensive everywhere; pure arithmetic is cheap everywhere. The encoder's latent space encodes this relative ordering, giving the new cost surrogate a strong initialization.

## Key Tradeoffs

**Encoder coupling**: Freezing the encoder during decoder training assumes the encoder's representation is sufficiently general. If the new backend has fundamentally different optimization opportunities (e.g., a backend that prefers depth-first vs. breadth-first computation), the encoder features may be poorly aligned. The optional fine-tuning step handles this, but requires care to avoid catastrophic forgetting of Triton VM performance.

**Validation cost for identity transfer**: Re-validating thousands of algebraic identities against a new backend's execution semantics can take hours. The identity explorer's Stage 1 validation (10K random inputs) is cheap; Stage 3 (symbolic proof) may require backend-specific symbolic execution support.

**Data collection for new backend**: Even 10% of Triton VM's training data means 10K programs proved on the new backend. If the backend is slow (long proving times), this collection phase dominates. The cost surrogate from the shared encoder can be used to prioritize which programs are most informative to prove, minimizing data collection cost.

## Implementation Path

1. Refactor neural compiler into encoder/decoder architecture with a clean interface boundary
2. Serialize and load encoder weights independently from decoder weights
3. Implement transfer training loop: freeze encoder, train decoder, optional joint fine-tuning
4. Integrate identity explorer validation pipeline as a backend-parameterized tool
5. Add new backend as a target in `trident build --target miden` — backend decoder loads from weights file
