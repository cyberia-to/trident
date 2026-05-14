---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Transfer Learning Across Proof Backends

**Related:** [[warrior-architecture]] · [[compiler-ensemble]] · [[cost-surrogate]] · [[cybergraph]] · [[bbg]] · [[Atlas]] · [[soft3]] · [[warrior-cyber]] · [reference/vm.md](../reference/vm.md)

## Vision

Transfer learning means every new chain, every new VM, every new proof system gets a Trident compiler immediately — not after months of training, but with 10% of the data. When a new L2 deploys with a custom proving backend, Trident supports it in days. The compiler ecosystem expands horizontally with the [[cybergraph]]'s reach. Each new warrior is a new node in the proving network; each new proving network node increases [[bbg]]'s capacity for parallel computation. The planetary intelligence network grows not just in knowledge but in proving bandwidth.

New warriors register in [[Atlas]] as packages (`atlas.cyber/warriors/miden`, etc.). The shared TIR encoder — frozen from [[nox]] training — lives as an [[Atlas]] package version. The new backend decoder is added as a companion. [[soft3]]'s `submit()` can route computations to the cheapest available warrior for any target. The [[warrior-cyber]] cpu/webgpu/metal backend split demonstrates the pattern at the hardware level; transfer learning scales the same pattern to entire new proving ecosystems.

## Motivation

Trident already targets 20 VMs (see [reference/vm.md](../reference/vm.md)). Each new warrior — Miden, SP1, OpenVM, or a future system — requires the neural compiler to learn its specific cost model. Training from scratch requires thousands of (program, proving time) pairs and weeks of compute. Transfer learning reuses the knowledge already encoded in the nox-targeting neural compiler, reducing new-warrior training data requirements to ~10% of the original.

The key insight: TIR-level optimization patterns generalize across warriors. The IR is the same — only the lowering to the specific instruction set changes. The shared encoder learns TIR-level patterns (control flow, data dependencies, field arithmetic idioms); each backend decoder learns its specific lowering and cost model.

## Design

### Split Architecture

The neural compiler is structured as two separable components:

```
TIR graph → [SHARED ENCODER] → latent representation → [BACKEND DECODER] → nox patterns / MASM / ...
```

The **shared encoder** learns the general structure of Trident programs at the TIR level: control flow patterns, data dependencies, loop structures, field arithmetic idioms. This knowledge is backend-agnostic — it operates purely on the 54 TIR ops across 4 tiers.

The **backend decoder** learns the specific instruction set and cost model of a particular warrior. For warrior-cyber targeting nox, cost = trace length. For a Miden warrior, cost = Miden cycle count. For an SP1 warrior, cost = SP1 constraint count. The decoder is the only part that must be retrained for a new warrior.

### Transfer Protocol

1. **Train on nox** (full training, ~100K programs): shared encoder + nox decoder jointly optimized. Encoder learns rich TIR representations; decoder learns nox-specific lowering.

2. **Freeze encoder** for new warrior: encoder weights are fixed. Only the decoder is trainable.

3. **Train decoder for new warrior** (~10K programs): much faster — decoder is a shallow network, and the pre-trained encoder provides high-quality TIR features immediately. Cold start is solved.

4. **Optional fine-tuning**: after decoder converges, unfreeze encoder and fine-tune jointly with a small learning rate. Adjusts encoder features toward the new warrior's cost landscape.

### Algebraic Identity Transfer

TIR-level algebraic identities (pure arithmetic: `Const(0); Add` → ∅, constant folding, identity elimination, commutativity) transfer directly — they hold in any field-based instruction set with the same Goldilocks arithmetic, regardless of which warrior executes them.

Architecture-dependent identities (those that reduce nox trace length specifically) may not transfer. The algebraic identity explorer's validation pipeline re-validates each identity against the new warrior's execution semantics. Identities that survive validation are added to the new warrior's rule database immediately — no re-discovery needed.

### Cost Model Transfer

Proof cost = nox trace length is nox-specific. Miden uses a different cycle model; SP1 uses a different constraint system entirely. The cost surrogate must be retrained per warrior.

However, the relative ordering of TIR patterns by "costliness" is partially preserved across warriors — hash-heavy programs are expensive everywhere; pure arithmetic is cheap everywhere. The encoder's latent space encodes this relative ordering, giving the new cost surrogate a strong initialization.

## Key Tradeoffs

**Encoder coupling**: Freezing the encoder during decoder training assumes the encoder's TIR-level representation is sufficiently general. If the new warrior has fundamentally different optimization opportunities (e.g., a warrior that prefers depth-first vs. breadth-first computation), the encoder features may be poorly aligned. The optional fine-tuning step handles this, but requires care to avoid catastrophic forgetting of nox performance.

**Validation cost for identity transfer**: Re-validating thousands of TIR-level algebraic identities against a new warrior's execution semantics can take hours. The identity explorer's Stage 1 validation (10K random inputs) is cheap; Stage 3 (symbolic proof) may require warrior-specific symbolic execution support.

**Data collection for new warrior**: Even 10% of the nox training data means 10K programs proved on the new warrior. If the warrior is slow (long proving times), this collection phase dominates. The cost surrogate from the shared encoder can be used to prioritize which programs are most informative to prove, minimizing data collection cost.

## Implementation Path

1. Refactor neural compiler into encoder/decoder architecture with a clean interface boundary
2. Serialize and load encoder weights independently from decoder weights
3. Implement transfer training loop: freeze encoder, train decoder, optional joint fine-tuning
4. Integrate identity explorer validation pipeline as a warrior-parameterized tool
5. Add new warrior as a target in `trident build --target miden` — warrior decoder loads from weights file; see [reference/vm.md](../reference/vm.md) for the full list of supported VMs
