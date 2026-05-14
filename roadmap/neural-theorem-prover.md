---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Neural Theorem Prover for TASM Equivalence

## Motivation

Showing that two TASM sequences compute the same function is the core problem of compiler correctness. The algebraic identity explorer validates identities empirically (random testing + Schwartz-Zippel). This is probabilistic: failure probability $< 10^{-7}$ is excellent, but not mathematical certainty. For high-assurance contexts — the identities that form the foundation of the compiler's optimization passes — mathematical proof is required.

A neural theorem prover for TASM equivalence searches for a chain of valid rewrite steps that transforms one sequence into another. Each step is a proven-correct rewrite rule. The chain is a constructive proof of equivalence — not a probabilistic argument, but a step-by-step demonstration that the transformation is valid. Every successful chain is a new peephole pattern. Every new peephole pattern enriches the rewrite vocabulary. The system is self-amplifying.

## Design

### Rewrite Rule Vocabulary

The NTP operates over a vocabulary of atomic rewrite rules. Each rule is correct by construction or proven correct by the algebraic identity explorer:

| Rule | Before | After | Condition |
|------|--------|-------|-----------|
| Dead push | `push X; pop` | ∅ | X unused by subsequent instructions |
| Constant fold | `push X; push Y; add` | `push (X+Y)` | X, Y are constants |
| Identity elim | `push 0; add` | ∅ | Always valid |
| Swap cancel | `swap K; swap K` | ∅ | Always valid |
| Commutativity | `push X; push Y; add` | `push Y; push X; add` | Always valid |
| Reorder | `A; B` | `B; A` | No data dependency between A, B |
| **Explorer rules** | *from identity database* | *from identity database* | *proven valid* |

The rule vocabulary starts with ~20 atomic rules and grows as the identity explorer validates new ones. Every rule added to the identity explorer's database is automatically available to the NTP.

### GNN Architecture for Sequence Embedding

The NTP embeds TASM sequences as vectors using a GNN on the dependency DAG:

```
TASM sequence → Dependency DAG → GNN embedding → 128-dim vector
```

The GNN uses the same architecture as the instruction scheduling GNN, but trained for a different task: producing embeddings that capture equivalence structure (two equivalent sequences should have similar embeddings).

```
Node features: instruction_type, operand_values (if constant), stack_depth, table_touched
Edge features: dependency_type (data/control), edge_distance
Message passing: 3 layers
Output: 128-dim graph-level embedding (mean pooling over node embeddings)
```

### The Proof Search Algorithm

Given source sequence $S$ and target sequence $T$:

1. Embed both: $e_S = \text{GNN}(S)$, $e_T = \text{GNN}(T)$
2. If $\|e_S - e_T\| < \epsilon$: likely equivalent, try random rule application to confirm
3. Predict the most useful rewrite rule: $r = \text{RulePredMLP}(e_S, e_T)$
4. Apply rule $r$ to $S$ at the highest-impact position → $S'$
5. If $S' = T$: proof found — return rule sequence
6. If $\|e_{S'} - e_T\| < \|e_S - e_T\|$: progress, recurse with $S'$
7. Otherwise: backtrack, try next rule

The search is beam search with beam width $K = 8$: maintain the 8 most promising current sequences (by embedding distance to target). This avoids getting stuck in local minima.

### Proof Depth Limit

The search is bounded at depth 50 rewrite steps. Longer proofs are unlikely to be found by heuristic search and likely indicate that the two sequences are not equivalent (or require fundamentally different algebraic reasoning).

When the depth limit is reached without finding a proof: fall back to the probabilistic validation (algebraic identity explorer stages 1–3). The NTP is a best-effort proof finder, not a complete proof system.

### Interaction with the Identity Explorer

The NTP and algebraic identity explorer form a mutual amplification loop:

1. **Explorer → NTP**: Every identity discovered by the explorer (stage 3+ validated) becomes a new NTP rewrite rule. The NTP can then prove equivalences using these rules as atomic steps.

2. **NTP → Explorer**: Every successful NTP proof is a chain of rewrite steps. This chain itself is a new peephole pattern — "sequence A can be transformed to sequence B in these steps" — which the explorer can validate and add to the database.

3. **NTP → Peephole**: Every successful NTP proof is also a new peephole pattern (the full chain, not just individual steps). Adding NTP-proven chains to the peephole database enables longer-range pattern matching that pure windowed peephole cannot achieve.

The three systems together — explorer, NTP, peephole — form a flywheel that grows more powerful as each adds to the shared rule database.

### Proof Certificates

Successful NTP proofs produce a certificate: the sequence of (rule, position) pairs that transforms the source into the target. This certificate is:
- Checkable independently (without the NTP) by applying the rules mechanically
- Storable in the rule database as evidence for the corresponding peephole pattern
- Publishable as a formal proof of the identity for auditing

```rust
struct ProofCertificate {
    source: TasmSequence,
    target: TasmSequence,
    steps: Vec<(RewriteRule, Position)>,
}

fn verify_certificate(cert: &ProofCertificate) -> bool {
    let mut current = cert.source.clone();
    for (rule, pos) in &cert.steps {
        current = rule.apply_at(&current, *pos);
    }
    current == cert.target
}
```

Certificate verification is $O(\text{steps} \times \text{sequence\_length})$ — fast and independent of the NTP.

## Key Tradeoffs

**Completeness**: The NTP is not a complete proof system. It can fail to find a proof even when two sequences are equivalent, if the proof requires more than 50 steps or if the heuristic search takes wrong turns. For the most important identities, the NTP should be augmented with breadth-first search (exhaustive, slow) or domain-specific proof strategies.

**GNN embedding quality**: The proof search's efficiency depends on the GNN embedding capturing equivalence structure well. If two equivalent sequences have distant embeddings, the search starts far from the target and may not converge. Training the GNN specifically for equivalence-sensitive embeddings is critical.

**Rule explosion**: As the vocabulary grows (from explorer discoveries), the number of possible rule applications at each step grows. The rule prediction MLP must accurately rank rules to keep beam search tractable. Poor rule prediction makes search exponentially slower.

**False negatives**: The NTP may report "proof not found" for equivalent sequences. This is safe (it does not falsely claim inequivalence), but it leaves valid identities unproven. The probabilistic validation (explorer stages 1–3) serves as the fallback.

## Implementation Sketch

```rust
// tools/ntp/search.rs
pub struct NeuralTheoremProver {
    embedder: TasmGNN,
    rule_predictor: RulePredMLP,
    rule_set: Vec<RewriteRule>,
}

impl NeuralTheoremProver {
    pub fn find_proof(
        &self,
        source: &TasmSequence,
        target: &TasmSequence,
        max_depth: usize,
    ) -> Option<ProofCertificate> {
        let e_target = self.embedder.embed(target);
        let mut beam = vec![(source.clone(), vec![])];

        for _ in 0..max_depth {
            let mut next_beam = Vec::new();
            for (current, proof_so_far) in &beam {
                if current == target { return Some(ProofCertificate::from(proof_so_far)); }
                let e_current = self.embedder.embed(current);
                let rule_scores = self.rule_predictor.rank(&e_current, &e_target);
                for rule in rule_scores.top_k(8) {
                    for pos in rule.applicable_positions(current) {
                        let next = rule.apply_at(current, pos);
                        let e_next = self.embedder.embed(&next);
                        let dist = (e_next - e_target).norm();
                        let mut new_proof = proof_so_far.clone();
                        new_proof.push((rule, pos));
                        next_beam.push((next, new_proof, dist));
                    }
                }
            }
            next_beam.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
            beam = next_beam.into_iter().take(8).map(|(s, p, _)| (s, p)).collect();
        }
        None
    }
}
```
