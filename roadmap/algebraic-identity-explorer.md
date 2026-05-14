---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Algebraic Identity Explorer

## Motivation

The Goldilocks field $p = 2^{64} - 2^{32} + 1$ has algebraic structure that extends indefinitely: arithmetic identities, subgroup structure, roots of unity, extension field shortcuts, and interactions between [[nox]]'s reduction patterns and the field's geometry. Human algebraists find these one at a time, by studying theory and having insights. A neural system can search the space systematically, discovering identities no human would find, at a rate no human can match.

Every discovered identity is a new compiler optimization pass. The compiler improves forever. The program corpus gets cheaper to prove — retroactively, for every program that matches the pattern. There is no ceiling. Every branch of algebra offers new passes. The deeper the network explores, the more it finds.

This changes the nature of compilation from a static engineering artifact to an open-ended learning system.

Related proposals: [[field-arithmetic-passes]], [[polynomial-optimization-passes]], [[learned-peephole]], [[neural-theorem-prover]].

## Vision

The rule database is an [[Atlas]] package — `atlas.cyber/trident/algebra-rules`. It grows continuously. Every device running the explorer contributes new identities. The [[cybergraph]] records each discovery: the identity pair is a particle ([[hemera]]-addressed), and the proof of its validity is a cyberlink from the pair to the [[zheng]] proof. Retroactively, all compiled programs benefit — recompile against the latest rule database version and get cheaper proofs for free. The algebraic structure of the Goldilocks field is being mapped by a distributed neural network, permanently recorded in the knowledge graph, available to every programmer for eternity.

The explorer is itself a Trident program, compiled to [[nox]], proved by [[zheng]]. Its own compilation benefits from the rules it discovers. The fixed point — when the explorer can no longer optimize its own compilation — defines the algebraic Shannon limit of the Goldilocks field for [[nox]].

Stack integration: The explorer runs as a [[nox]] program. Its execution produces cyberlinks: `(sequence_A, sequence_B) → zheng_proof_of_equivalence`. The [[cybergraph]] is simultaneously the explorer's output store and its training corpus. Each new identity makes future identities easier to find (compositional flywheel). [[bbg]] charges focus for each exploration step; the [[cybergraph]] offsets this by providing free cached validation results for known pairs.

## Design

### Algebraic Layers

The field has identifiable layers of algebraic structure, each with its own optimization surface:

| Layer | Example | Savings per match |
|-------|---------|-------------------|
| 0 — Arithmetic | identity reduction: `add 0` → ∅ | 1–2 nox steps |
| 1 — Goldilocks constants | `mul 2^32` → shift trick | 3–10 nox steps |
| 2 — Subgroup inversions | Fermat with exponent $2^{32}-2$ | 10–50 nox steps |
| 3 — Roots of unity | NTT butterfly for ω-constants | 50–200 nox steps |
| 4 — Hash shortcuts | hemera (Poseidon2) internal structure | 100–500 nox steps |
| 5+ — Polynomial geometry | Evaluation at structured points | 200–1000+ nox steps |

Layer N is essentially unbounded: interactions between the field, the nox reduction patterns + jets, and accumulated composed identities from lower layers.

Proof cost in nox/zheng is `trace_length + sum(jet_costs)`. Every reduction in trace length or jet invocation count directly lowers proving cost. A match at Layer 3 that eliminates 100 nox steps saves 100 units of trace cost.

### Discovery Architecture

**Proposer (GFlowNet)**: Generates candidate [[nox]] pattern sequence pairs (sequence_A, sequence_B) — the claim being that both sequences compute the same function. Input: known identity database, [[nox]] reduction pattern vocabulary (16 patterns + 5 jets + 1 hint), frequency data from real programs. Output: candidate pairs of 2–12 pattern applications. Reward: identity_found × usefulness_score. Diversity maintained via GFlowNet sampling.

**Validator (4 stages)**:
1. Execute both nox sequences on 10,000 random inputs — any output disagreement rejects immediately
2. Execute on 10,000,000 inputs — false positive probability below $10^{-7}$
3. Symbolic execution → express both as polynomial maps → verify via Schwartz-Zippel
4. Optional: [[zheng]] proof of equivalence for high-value identities ([[warrior-cyber]] runs the equivalence program, producing a full [[nox]] trace + [[zheng]] proof)

**Usefulness scorer**: Scans the program corpus. For each validated identity: frequency (how often does sequence_A appear in [[nox]] traces?), savings (trace_cost(A) - trace_cost(B) in [[nox]] steps), jet_criticality (extra weight if savings reduce expensive jet invocations — hash jet via [[hemera]], poly_eval jet). Score = frequency × savings × jet_criticality. The [[trace-predictor]] provides jet_criticality weights dynamically.

**Rule database**: Each rule carries pattern, replacement, cost_savings (in [[nox]] trace steps), confidence (validation stage 1–4), frequency, layer, discovery date, composable_with list. Applied deterministically before the neural compiler runs. Sorted by (frequency × savings) descending. Longest-match wins for conflicts.

The rule database applies at TIR level (see `../reference/ir.md` for TIR op definitions) and is also consulted by [[learned-peephole]] for its deterministic pass.

### The Compounding Flywheel

Four compounding effects per discovered identity:

1. **Direct savings**: all programs with the pattern get cheaper to prove, retroactively
2. **Training enrichment**: each identity teaches the GFlowNet the shape of valid rewrites
3. **Compositional explosion**: identity A transforms X→Y, identity B transforms Y→Z, yielding X→Z — a deeper identity invisible at either layer
4. **Corpus shift**: applied rules change the instruction patterns in real programs, creating new patterns for the proposer to explore

The rule database only grows. Savings compound multiplicatively across layers. A program benefiting from Layer 1 + Layer 3 + Layer 4 identities could see 3–5× proving cost reduction.

### Self-Referential Closure

The explorer is itself a Trident program. Its compilation benefits from the identities it discovers. The explorer optimizes its own execution. The fixed point: when the explorer can no longer improve its own compilation cost, it has extracted the maximum algebraic efficiency reachable by its architecture — a lower bound on the extractable efficiency of the Goldilocks field for nox.

A larger explorer reaches a lower fixed point. The hierarchy of fixed points, indexed by explorer capacity, converges to the theoretical minimum proving cost — the algebraic Shannon limit of the field.

The explorer runs 24/7 as a background process. Discovered rules are snapshotted weekly into the rule database and trigger a monthly corpus recompilation to measure cumulative savings.

### Estimated Cumulative Impact

Savings compound multiplicatively across layers. Programs reaching multiple layers simultaneously achieve 3–5× total proving cost reduction. The cumulative impact across the full corpus, over months of continuous operation, is estimated at 5–70%+ depending on program type. Programs heavy in polynomial arithmetic and hash-adjacent operations benefit most.

## Key Tradeoffs

**Validation cost vs. confidence**: Stage 1 validation (10K inputs) is fast but accepts false positives at probability $10^{-4}$. Stage 2 (10M inputs) drops this to $10^{-7}$. Stage 3 (symbolic) gives near-certainty. Stage 4 (STARK) gives mathematical proof. The system runs stages 1–2 continuously and triggers stage 3 for high-usefulness candidates.

**Compositional depth limit**: Automated composition searches all pairs (A, B) where A's output overlaps B's input. With $N$ rules, this is $O(N^2)$ pairs — manageable for hundreds of rules, expensive for thousands. The composition search caps at depth 3 and runs asynchronously.

**Reward shaping**: The GFlowNet's reward signal (identity_found × usefulness) may encourage re-discovering known identities at Layer 0 (trivial but frequent). Penalties for redundant discoveries (zero reward for known identities) and exploration bonuses for novel instruction combinations maintain diversity.

## Implementation Sketch

**Phase A** (1 week): Brute-force random pair generator over nox pattern sequences of length 2–4. Builds initial rule database, establishes validation pipeline. No NN.

**Phase B** (2 weeks): Small MLP (~10K params) filters random proposals. 10× speedup over brute force. Uses [[nn-trd]] for field-native inference on [[nox]].

**Phase C** (3 weeks): GFlowNet proposer. Reward-driven, diversity-guaranteed exploration across all algebraic layers. The proposer is implemented in [[nn-trd]] and is itself a provable neural network — every inference call produces a [[zheng]] proof.

**Phase D** (2 weeks): Automated composition search. Depth-limited to 3. Runs asynchronously alongside Phase C.

**Phase E** (ongoing): 24/7 continuous operation. Weekly rule database snapshots. Monthly corpus recompilation to measure cumulative savings.

The explorer is scheduled for 128K — it requires the full compiler stack to be stable before meaningful corpus data accumulates, and its value compounds with the size and diversity of the program corpus. See also [[neural-theorem-prover]] for formal equivalence proofs of high-value identities.
