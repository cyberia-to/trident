---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Learned Peephole Optimization Patterns

## Motivation

The [[algebraic-identity-explorer]] discovers identities from field theory — it knows that a pattern is equivalent to another pattern because both represent the same mathematical function over Goldilocks. But not all valuable compiler patterns are algebraic identities. Some are compiler-specific heuristics: nox reduction sequences that, in practice, consistently appear in suboptimal programs and can be replaced with cheaper alternatives — not because of field-theoretic equivalence, but because of how the Trident compiler generates nox sequences and how [[warrior-cyber]] proves them via [[zheng]].

Peephole optimization operates on nox reduction sequences (23-op vocabulary: 18 patterns + 5 jets). The goal is cheaper [[nox]] execution: fewer trace steps and fewer jet invocations, which determines both trace_length and jet invocation counts.

Learned peephole patterns extract these compiler-specific heuristics from observing the difference between naive compiler output and evolved (optimized) compiler output. Where the algebraic explorer asks "what is mathematically equivalent?", the peephole learner asks "what did the evolutionary compiler change at the nox level, and can we replicate those changes as fast deterministic rules?"

Related proposals: [[algebraic-identity-explorer]], [[instruction-scheduling-nn]], [[neural-theorem-prover]], [[compiler-ensemble]].

## Vision

Peephole rules are compiler knowledge made explicit. Once extracted by the [[learned-peephole]] system and validated by [[neural-theorem-prover]], they are added to the deterministic rule database — an [[Atlas]] package. Every future compilation automatically applies them. The neural discovery process is one-time; the economic benefit is forever. In the [[cybergraph]], the rule database version is a particle; each rule is a cyberlink from the pattern particle ([[hemera]]-addressed) to the replacement particle.

Stack integration: Peephole rules operate on nox reduction sequences. The most impactful rules — those that reduce [[hemera]] jet invocations — have outsized impact because every [[hemera]] call touches the [[cybergraph]]'s identity layer. Rules are deployed as an [[Atlas]] package; compilers pin to a version and can reproduce compilation results exactly. As the [[bbg]] network accumulates more execution data, the rule database's economic value compounds — more usage means more pattern data means better rules means cheaper proofs for everyone.

## Design

### What Peephole Optimization Is

A peephole optimizer scans nox reduction sequences with a sliding window (size 3–8 operations). At each window position, it checks whether the windowed sequence matches a known pattern and, if so, replaces it with a cheaper equivalent:

```
Window [Mul(x, x)]           → detected as square pattern → replace with [Square(x)]
Window [Add(x, Const(0))]    → detected as add-zero pattern → eliminate
Window [Mul(x, Const(1))]    → detected as mul-one pattern → eliminate
```

These reductions lower the nox trace length (fewer reduction pattern applications) or eliminate jet invocations. Classical peephole patterns are hand-coded by engineers who study compiler output. Learned peephole patterns are discovered automatically from data.

The patterns from the [[algebraic-identity-explorer]] (validated field-theoretic equivalences, some with full [[zheng]] proofs) feed directly into this system's rule database — they are the highest-confidence peephole rules.

### The Training Pipeline

**Step 1**: Collect paired programs. For each of 10,000 training programs, generate:
- Naive nox sequence: from the standard Trident frontend, no optimization
- Evolved nox sequence: from the [[compiler-ensemble]] / [[evolutionary-training]] output (many optimization generations)

**Step 2**: Align and diff at nox operation level. Find the minimal set of local changes that transforms the naive nox sequence into the evolved nox sequence. This produces per-window changes: `{position: 47, before: [OpA, OpB, OpC], after: [OpD, OpE]}`.

**Step 3**: Train the peephole CNN. The model sees a window (5–8 nox operations, each one of 23 kinds: 18 patterns + 5 jets) and predicts whether it should be rewritten and, if so, which replacement from a learned vocabulary:

```
Architecture:
  Input: window of 8 nox operation encodings (each: op_kind [23 kinds] + operand types)
  Conv1D(kernel=5, filters=32) → ReLU
  Conv1D(kernel=3, filters=32) → ReLU
  GlobalPool
  Dense(64) → ReLU
  Dense(N_REPLACEMENTS + 1)  → softmax
  // N_REPLACEMENTS: number of learned replacement patterns (starts at 50, grows)
```

Parameters: ~15,000 field elements. Inference: fast (microseconds per window). Implemented as a [[nn-trd]] network — inference is itself a provable [[nox]] trace proved by [[zheng]].

**Step 4**: Extract high-confidence patterns as deterministic rules. When the CNN predicts a specific nox reduction replacement with confidence > 95% over a diverse set of programs, extract the (before, after) pair as a deterministic rule. Add it to the rule database alongside [[algebraic-identity-explorer]] rules.

### Relationship to the Algebraic Identity Explorer

The two systems discover nox rewrite patterns through different mechanisms:

| | Algebraic Identity Explorer | Peephole Learner |
|--|--|--|
| **Source** | Field theory, symbolic reasoning over nox patterns | Evolutionary compiler output — nox reduction sequences |
| **Claim** | "A ≡ B for all inputs" (mathematical, field-theoretic) | "A is usually replaced by B" (empirical) |
| **Validation** | 4-stage (brute force + symbolic + zheng proof) | Confidence threshold + correctness check |
| **Rule type** | Universal equivalences over Goldilocks | Compiler-specific nox heuristics |
| **Layer** | Algebraic layers 0–5+ | nox reduction sequence level |

They feed the same rule database. Before any neural compiler runs, both algebraic identities and peephole patterns are applied as deterministic nox rewrite passes in order of (frequency × [[nox]]_cost_savings). The deterministic rules handle the majority of common patterns; the neural compiler (see `../reference/neural.md`) focuses on unusual cases.

The [[instruction-scheduling-nn]] runs after peephole — peephole reduces the nox operation count first, then scheduling reorders the reduced sequence for minimal nox trace cost.

### Growing Rule Vocabulary

As the [[compiler-ensemble]] improves (through better specialists, better [[instruction-scheduling-nn]], more training data), the peephole learner retrains on the new evolved nox output. New patterns emerge in the evolved nox sequences that weren't there before. The rule vocabulary grows:

- Month 1: 50 patterns (obvious replacements)
- Month 3: 200 patterns (learned sequences, multi-step rewrites)
- Month 6: 500 patterns (deep patterns from mature evolutionary compiler)
- Month 12: 1000+ patterns (compositional, context-dependent)

Each growth phase reduces the compiler's workload: more patterns handled deterministically → neural compiler sees harder, more unusual cases → neural compiler improves on the hard cases → evolutionary compiler produces new nox patterns → repeat.

### Correctness Validation

Unlike algebraic identities (which are mathematically proven via field theory and optionally by zheng proof), peephole patterns extracted from evolutionary compiler output must be validated before deployment as deterministic rules. The validation pipeline:

1. Extract candidate nox rule: `before → after`
2. Execute both nox sequences on 1,000,000 random inputs — output must agree on all
3. Cross-validate on 100 programs not in the training set — must improve or equal [[nox]] proof cost
4. If passes: add to rule database with confidence level "peephole_validated"

The validation is less stringent than for algebraic identities (Stage 2 rather than Stage 3/4) because peephole rules are empirically discovered rather than theoretically derived. Borderline cases fail safe — they are not added to the deterministic database and remain as CNN predictions only.

For high-value patterns (savings > 500 nox steps, frequency > 1000 matches), consider escalating to [[neural-theorem-prover]] for formal equivalence proof.

## Key Tradeoffs

**Generalization gap**: Patterns extracted from 10,000 training programs may not generalize to all programs. A rule that looks universal on the training set may have exceptions in unusual programs. The validation step catches most exceptions, but rare edge cases may slip through. The rule database should track how often each rule is applied versus how often it improves cost, to detect low-quality rules in production.

**Interaction with algebraic identities**: Some peephole patterns may be rediscoveries of algebraic identities (the evolutionary compiler applied an algebraic identity, and the peephole learner picks it up empirically). These are redundant but harmless — both the algebraic and peephole versions will be in the database, and the longest match rule prevents double-application.

**Rule ordering**: When a nox reduction window matches both an algebraic identity rule and a peephole rule, which takes precedence? The rule database uses (frequency × nox_cost_savings) ordering, so the more impactful rule fires first. For conflicts where both rules apply to the same window, the rule with higher nox cost savings wins.

**Training data quality**: The evolutionary compiler's output quality determines what patterns are available to learn. Early in the system's development, when the evolutionary compiler is immature, peephole rules add little value. The peephole learner benefits most from a mature evolutionary compiler — schedule it for later in the 128K development arc.

## Implementation Sketch

```rust
// nox/peephole/learned.rs
pub struct LearnedPeepholeOptimizer {
    cnn: PeepholeCNN,
    rule_db: RuleDatabase,  // shared with algebraic-identity-explorer
}

impl LearnedPeepholeOptimizer {
    pub fn optimize(&self, nox: &mut NoxSequence) {
        // First pass: apply all deterministic nox rewrite rules
        // (algebraic identities + confirmed peephole patterns)
        apply_rule_database(nox, &self.rule_db);

        // Second pass: CNN-guided optimization for remaining nox reduction windows
        let mut i = 0;
        while i < nox.ops().len().saturating_sub(8) {
            let window = &nox.ops()[i..i+8];  // 8 nox operations (22 kinds)
            if let Some((replacement, confidence)) = self.cnn.predict(window) {
                if confidence > 0.80 {  // threshold for CNN-guided replacement
                    nox.replace_window(i, 8, replacement);
                    // Don't advance i — re-check the position with new ops
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        // After peephole: nox sequence is passed to [[instruction-scheduling-nn]],
        // then proved via [[warrior-cyber]] / [[zheng]]
    }
}
```

The deterministic rule pass handles high-confidence nox reduction patterns cheaply. The CNN pass handles residual patterns with a higher computational cost but lower frequency. Together, they cover the full peephole optimization surface at the nox level.
