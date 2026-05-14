---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Learned Peephole Optimization Patterns

## Motivation

The algebraic identity explorer discovers identities from field theory — it knows that a pattern is equivalent to another pattern because both represent the same mathematical function. But not all valuable compiler patterns are algebraic identities. Some are compiler-specific heuristics: instruction sequences that, in practice, consistently appear in suboptimal TASM and can be replaced with cheaper alternatives — not because of field-theoretic equivalence, but because of how the compiler generates code and how Triton VM executes it.

Learned peephole patterns extract these compiler-specific heuristics from observing the difference between naive compiler output and evolved (optimized) compiler output. Where the algebraic explorer asks "what is mathematically equivalent?", the peephole learner asks "what did the evolutionary compiler change, and can we replicate those changes as fast deterministic rules?"

## Design

### What Peephole Optimization Is

A peephole optimizer scans TASM instruction sequences with a sliding window (size 3–8 instructions). At each window position, it checks whether the windowed sequence matches a known pattern and, if so, replaces it with a cheaper equivalent:

```
Window [push X; dup; mul] → detected as x^2 pattern → replace with [dup; mul]
Window [push 0; add]      → detected as add-zero pattern → replace with []
Window [push 1; mul]      → detected as mul-one pattern → replace with []
```

Classical peephole patterns are hand-coded by engineers who study compiler output. Learned peephole patterns are discovered automatically from data.

### The Training Pipeline

**Step 1**: Collect paired programs. For each of 10,000 training programs, generate:
- Naive TASM: from the standard TIR→TASM lowering, no optimization
- Evolved TASM: from the evolutionary compiler (many generations of optimization)

**Step 2**: Align and diff. Find the minimal set of local changes that transforms naive TASM into evolved TASM. This produces per-window changes: `{position: 47, before: [A, B, C], after: [D, E]}`.

**Step 3**: Train the peephole CNN. The model sees a window (5–8 instructions) and predicts whether it should be rewritten and, if so, which replacement from a learned vocabulary:

```
Architecture:
  Input: window of 8 instruction encodings (each: instruction_id + operands)
  Conv1D(kernel=5, filters=32) → ReLU
  Conv1D(kernel=3, filters=32) → ReLU
  GlobalPool
  Dense(64) → ReLU
  Dense(N_REPLACEMENTS + 1)  → softmax
  // N_REPLACEMENTS: number of learned replacement patterns (starts at 50, grows)
```

Parameters: ~15,000 field elements. Inference: fast (microseconds per window).

**Step 4**: Extract high-confidence patterns as deterministic rules. When the CNN predicts a specific replacement with confidence > 95% over a diverse set of programs, extract the (before, after) pair as a deterministic rule. Add it to the rule database alongside algebraic identity explorer rules.

### Relationship to the Algebraic Identity Explorer

The two systems discover patterns at different levels and through different mechanisms:

| | Algebraic Identity Explorer | Peephole Learner |
|--|--|--|
| **Source** | Field theory, symbolic reasoning | Evolutionary compiler output |
| **Claim** | "A ≡ B for all inputs" (mathematical) | "A is usually replaced by B" (empirical) |
| **Validation** | 4-stage (brute force + symbolic + STARK) | Confidence threshold + correctness check |
| **Rule type** | Universal equivalences | Compiler-specific heuristics |
| **Layer** | Algebraic layers 0–5+ | Compiler architecture layer |

They feed the same rule database. Before any neural compiler runs, both algebraic identities and peephole patterns are applied as deterministic passes in order of (frequency × savings). The deterministic rules handle the majority of common patterns; the neural compiler focuses on unusual cases.

### Growing Rule Vocabulary

As the evolutionary compiler improves (through better specialists, better GNN scheduling, more training data), the peephole learner retrains on the new output. New patterns emerge in the evolved TASM that weren't there before. The rule vocabulary grows:

- Month 1: 50 patterns (obvious replacements)
- Month 3: 200 patterns (learned sequences, multi-step rewrites)
- Month 6: 500 patterns (deep patterns from mature evolutionary compiler)
- Month 12: 1000+ patterns (compositional, context-dependent)

Each growth phase reduces the compiler's workload: more patterns handled deterministically → neural compiler sees harder, more unusual cases → neural compiler improves on the hard cases → evolutionary compiler produces new patterns → repeat.

### Correctness Validation

Unlike algebraic identities (which are mathematically proven), peephole patterns extracted from evolutionary compiler output must be validated before deployment as deterministic rules. The validation pipeline:

1. Extract candidate rule: `before → after`
2. Run both sequences on 1,000,000 random inputs — must agree on all
3. Cross-validate on 100 programs not in the training set — must improve or equal cost
4. If passes: add to rule database with confidence level "peephole_validated"

The validation is less stringent than for algebraic identities (Stage 2 rather than Stage 3/4) because peephole rules are empirically discovered rather than theoretically derived. Borderline cases fail safe — they are not added to the deterministic database and remain as CNN predictions only.

## Key Tradeoffs

**Generalization gap**: Patterns extracted from 10,000 training programs may not generalize to all programs. A rule that looks universal on the training set may have exceptions in unusual programs. The validation step catches most exceptions, but rare edge cases may slip through. The rule database should track how often each rule is applied versus how often it improves cost, to detect low-quality rules in production.

**Interaction with algebraic identities**: Some peephole patterns may be rediscoveries of algebraic identities (the evolutionary compiler applied an algebraic identity, and the peephole learner picks it up empirically). These are redundant but harmless — both the algebraic and peephole versions will be in the database, and the longest match rule prevents double-application.

**Rule ordering**: When a TASM window matches both an algebraic identity rule and a peephole rule, which takes precedence? The rule database uses (frequency × savings) ordering, so the more impactful rule fires first. For conflicts where both rules apply to the same window, the rule with higher savings wins.

**Training data quality**: The evolutionary compiler's output quality determines what patterns are available to learn. Early in the system's development, when the evolutionary compiler is immature, peephole rules add little value. The peephole learner benefits most from a mature evolutionary compiler — schedule it for later in the 128K development arc.

## Implementation Sketch

```rust
// tasm/peephole/learned.rs
pub struct LearnedPeepholeOptimizer {
    cnn: PeepholeCNN,
    rule_db: RuleDatabase,  // shared with algebraic identity explorer
}

impl LearnedPeepholeOptimizer {
    pub fn optimize(&self, tasm: &mut TasmProgram) {
        // First pass: apply all deterministic rules (algebraic + confirmed peephole)
        apply_rule_database(tasm, &self.rule_db);

        // Second pass: CNN-guided optimization for remaining patterns
        let mut i = 0;
        while i < tasm.len().saturating_sub(8) {
            let window = &tasm[i..i+8];
            if let Some((replacement, confidence)) = self.cnn.predict(window) {
                if confidence > 0.80 {  // threshold for CNN-guided replacement
                    tasm.replace_window(i, 8, replacement);
                    // Don't advance i — re-check the position with new instructions
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }
}
```

The deterministic rule pass handles high-confidence patterns cheaply. The CNN pass handles residual patterns with a higher computational cost but lower frequency. Together, they cover the full peephole optimization surface.
