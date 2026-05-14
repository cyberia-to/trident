---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Algebraic Identity Explorer

## Motivation

The Goldilocks field $p = 2^{64} - 2^{32} + 1$ has algebraic structure that extends indefinitely: arithmetic identities, subgroup structure, roots of unity, extension field shortcuts, and interactions between Triton VM's instruction set and the field's geometry. Human algebraists find these one at a time, by studying theory and having insights. A neural system can search the space systematically, discovering identities no human would find, at a rate no human can match.

Every discovered identity is a new compiler optimization pass. The compiler improves forever. The program corpus gets cheaper to prove — retroactively, for every program that matches the pattern. There is no ceiling. Every branch of algebra offers new passes. The deeper the network explores, the more it finds.

This changes the nature of compilation from a static engineering artifact to an open-ended learning system.

## Design

### Algebraic Layers

The field has identifiable layers of algebraic structure, each with its own optimization surface:

| Layer | Example | Savings per match |
|-------|---------|-------------------|
| 0 — Arithmetic | `push 0; add` → ∅ | 1–2 rows |
| 1 — Goldilocks constants | `push 2^32; mul` → shift trick | 3–10 rows |
| 2 — Subgroup inversions | Fermat with exponent $2^{32}-2$ | 10–50 rows |
| 3 — Roots of unity | NTT butterfly for ω-constants | 50–200 rows |
| 4 — Hash shortcuts | Tip5 internal structure | 100–500 rows |
| 5+ — Polynomial geometry | Evaluation at structured points | 200–1000+ rows |

Layer N is essentially unbounded: interactions between the field, the instruction set, and accumulated composed identities from lower layers.

### Discovery Architecture

**Proposer (GFlowNet)**: Generates candidate TASM sequence pairs (sequence_A, sequence_B) — the claim being that both sequences compute the same function. Input: known identity database, instruction vocabulary (~44 TASM ops), frequency data from real programs. Output: candidate pairs of 2–12 instructions. Reward: identity_found × usefulness_score. Diversity maintained via GFlowNet sampling.

**Validator (4 stages)**:
1. Execute both sequences on 10,000 random inputs — any output disagreement rejects immediately
2. Execute on 10,000,000 inputs — false positive probability below $10^{-7}$
3. Symbolic execution → express both as polynomial maps → verify via Schwartz-Zippel
4. Optional: STARK proof of equivalence for high-value identities

**Usefulness scorer**: Scans the program corpus. For each validated identity: frequency (how often does sequence_A appear?), savings (cost(A) - cost(B)), table_criticality (extra weight if savings hit the current bottleneck table). Score = frequency × savings × table_criticality.

**Rule database**: Each rule carries pattern, replacement, cost_savings, confidence (validation stage 1–4), frequency, layer, discovery date, composable_with list. Applied deterministically before the neural compiler runs. Sorted by (frequency × savings) descending. Longest-match wins for conflicts.

### The Compounding Flywheel

Four compounding effects per discovered identity:

1. **Direct savings**: all programs with the pattern get cheaper to prove, retroactively
2. **Training enrichment**: each identity teaches the GFlowNet the shape of valid rewrites
3. **Compositional explosion**: identity A transforms X→Y, identity B transforms Y→Z, yielding X→Z — a deeper identity invisible at either layer
4. **Corpus shift**: applied rules change the instruction patterns in real programs, creating new patterns for the proposer to explore

The rule database only grows. Savings compound multiplicatively across layers. A program benefiting from Layer 1 + Layer 3 + Layer 4 identities could see 3–5× proving cost reduction.

### Self-Referential Closure

The explorer is itself a Trident program. Its compilation benefits from the identities it discovers. The explorer optimizes its own execution. The fixed point: when the explorer can no longer improve its own compilation cost, it has extracted the maximum algebraic efficiency reachable by its architecture — a lower bound on the extractable efficiency of the Goldilocks field for Triton VM.

A larger explorer reaches a lower fixed point. The hierarchy of fixed points, indexed by explorer capacity, converges to the theoretical minimum proving cost — the algebraic Shannon limit of the field.

### Estimated Cumulative Impact

Savings compound multiplicatively across layers. Programs reaching multiple layers simultaneously achieve 3–5× total proving cost reduction. The cumulative impact across the full corpus, over months of continuous operation, is estimated at 5–70%+ depending on program type. Programs heavy in polynomial arithmetic and hash-adjacent operations benefit most.

## Key Tradeoffs

**Validation cost vs. confidence**: Stage 1 validation (10K inputs) is fast but accepts false positives at probability $10^{-4}$. Stage 2 (10M inputs) drops this to $10^{-7}$. Stage 3 (symbolic) gives near-certainty. Stage 4 (STARK) gives mathematical proof. The system runs stages 1–2 continuously and triggers stage 3 for high-usefulness candidates.

**Compositional depth limit**: Automated composition searches all pairs (A, B) where A's output overlaps B's input. With $N$ rules, this is $O(N^2)$ pairs — manageable for hundreds of rules, expensive for thousands. The composition search caps at depth 3 and runs asynchronously.

**Reward shaping**: The GFlowNet's reward signal (identity_found × usefulness) may encourage re-discovering known identities at Layer 0 (trivial but frequent). Penalties for redundant discoveries (zero reward for known identities) and exploration bonuses for novel instruction combinations maintain diversity.

## Implementation Sketch

**Phase A** (1 week): Brute-force random pair generator over sequences of length 2–4. Builds initial rule database, establishes validation pipeline. No NN.

**Phase B** (2 weeks): Small MLP (~10K params) filters random proposals. 10× speedup over brute force.

**Phase C** (3 weeks): GFlowNet proposer. Reward-driven, diversity-guaranteed exploration across all algebraic layers.

**Phase D** (2 weeks): Automated composition search. Depth-limited to 3. Runs asynchronously alongside Phase C.

**Phase E** (ongoing): 24/7 continuous operation. Weekly rule database snapshots. Monthly corpus recompilation to measure cumulative savings.

The explorer is scheduled for 128K — it requires the full compiler stack to be stable before meaningful corpus data accumulates, and its value compounds with the size and diversity of the program corpus.
