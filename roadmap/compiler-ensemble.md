---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Multi-Objective Compiler Ensemble

## Motivation

A single TIR optimizer cannot be optimal for all programs. In [[nox]]/[[zheng]], proof cost = `trace_length + sum(jet_costs)`. Which component dominates varies by program: a hash-heavy program is dominated by [[hemera]] jet invocations; a polynomial-heavy program is dominated by poly_eval jet calls; a pure-arithmetic program is dominated by trace length. No single optimizer can dominate across all program types.

The ensemble solution: 8–16 specialist optimizers, each tuned to minimize a specific [[nox]] cost component. For each program, run all specialists in parallel (~800μs combined), use the [[trace-predictor]] or [[cost-surrogate]] to predict which specialist's output will prove cheapest via [[zheng]], and lower only that one. The meta-selector eliminates the need to run [[warrior-cyber]] on all 16 variants.

Related proposals: [[cost-surrogate]], [[instruction-scheduling-nn]], [[trace-predictor]], [[learned-peephole]].

## Vision

The 16 specialist optimizers are like competing warriors — each one bids on proving the program the cheapest way, biased by its specialty. The meta-selector picks the winner based on the [[trace-predictor]]'s estimate. This mirrors [[bbg]]'s focus market: multiple provers compete, the cheapest valid proof wins. The compiler ensemble is a simulation of the proving market at compile time, optimizing for the same objective the network will optimize at runtime.

Stack integration: Each specialist's TIR output is lowered by [[warrior-cyber]] to a [[nox]] trace. The trace lengths are compared (or estimated via [[cost-surrogate]]). The cheapest trace wins. In the limit, the compiler ensemble and the [[bbg]] proving market converge on the same program representation. Competition results and actual proving costs are cyberlinked in [[cybergraph]] — the ensemble's meta-selector improves continuously as more competition data accumulates.

## Design

### Specialist Objectives

Each specialist is a TIR optimizer with a different nox cost fitness function. The specialists cover the space of possible bottleneck scenarios:

```
specialist_0:  minimize total proof cost (trace_length + all jet_costs)  — general optimizer
specialist_1:  minimize trace_length only                                — pure arithmetic programs
specialist_2:  minimize hash jet invocations ([[hemera]] calls)            — hash-dominated programs
specialist_3:  minimize poly_eval jet invocations                        — polynomial-heavy programs
specialist_4:  minimize invert jet invocations                           — inversion-heavy programs
specialist_5:  minimize (trace_length + hash_jet_cost)                   — joint trace+hash bottleneck
specialist_6:  minimize imbalance between cost components                — balance optimization
specialist_7:  minimize hash jets subject to trace_length < threshold    — hash-first with trace budget
...
specialist_15: minimize trace_length ignoring jet costs                  — jet-unaware trace optimizer
```

The specific objectives are tunable. The key principle: cover the space of possible nox cost bottleneck configurations so that the true optimal for any program is close to at least one specialist.

### Specialist Architecture

Each specialist is an evolutionary-trained neural compiler (see `../reference/neural.md` for the canonical architecture):
- Input: TIR graph features (TirGraph: 54 op kinds, 3 edge types, 59-dim node features)
- Output: TIR op ordering decisions (lowered to nox trace by warrior-cyber)
- Trained on programs where its target nox cost component was the bottleneck
- Parameters: ~728KB (91,000 field elements)

Training: run [[evolutionary-training]] on programs curated for each specialist's bottleneck scenario. Specialist 2 (hash jet minimizer) trains on programs where [[hemera]] jet invocations dominate proof cost after compilation with a naive optimizer.

### Meta-Selection

Running all 16 specialists for every program costs $16 \times 50\mu s \approx 800\mu s$ — negligible compared to proving (seconds via zheng). The meta-selector then picks the winner:

```
For each program P:
1. Run all 16 specialists on P's TIR              → 16 TIR op orderings
2. Run trace predictor on each ordering's features → 16 predicted nox costs
                                                     (trace_length + jet_costs)
3. Select ordering with minimum predicted cost     → LOWER THIS ONE via warrior-cyber
```

Step 2 uses the [[trace-predictor]] (fast, no lowering required) rather than the [[cost-surrogate]] (requires TIR op sequence encoding). The predictor runs on TIR graph features of each specialist's output, providing cost predictions in microseconds.

Alternatively, for higher accuracy, use the cost surrogate:

```
Step 2': Encode each TIR ordering as op sequence
         Run cost surrogate on each sequence  → 16 predicted nox costs
         (cost surrogate: ~50μs per variant × 16 = 800μs additional)
```

Total meta-selection overhead: 800μs + 800μs = 1.6ms. Still negligible compared to zheng proving time.

### Memory Footprint

16 specialists × 728KB = 11.6MB — fits in L2 cache on modern CPUs. All specialists can be resident simultaneously with no cache pressure. The entire ensemble, including the meta-selector, operates within a 15MB working set.

### The Cost Decomposition Argument

Why does the ensemble dominate a single optimizer?

In [[nox]]/[[zheng]], proof cost = `trace_length + sum(jet_costs)`. Which term dominates depends on the program, and the bottleneck can shift mid-optimization (reducing hash jet invocations may reveal that trace_length is now the dominant cost, requiring a different strategy). Note: [[zheng]] uses Brakedown PCS, not FRI — there are no FRI folding factor cliffs. Cost structure is linear in trace length and jet counts, not power-of-2 stepped (unless the [[zheng]] prover configuration adds explicit padding — check the [[zheng]] prover configuration for cliff behavior).

A single optimizer trained on diverse programs learns to balance all cost components, which is suboptimal for programs with a dominant bottleneck. Specialist optimizers learn to aggressively minimize their target cost component. For programs dominated by hash jet calls, the hash specialist wins. For programs dominated by trace length, the trace specialist wins. For balanced programs, the balance specialist (specialist_6) wins. The meta-selector ensures the right specialist is applied to each program.

### Online Improvement

The ensemble improves online:

1. Each program compiled produces actual proving data: (TIR features, specialist chosen, actual proving time)
2. This data improves the meta-selector's accuracy (train on the ground truth: which specialist was actually cheapest?)
3. Specialists that consistently lose to others on specific program types are retrained on those types
4. New specialists can be added as new bottleneck patterns emerge

The ensemble is not a static artifact — it grows and improves with usage.

## Key Tradeoffs

**Specialist training data**: Each specialist needs training data where its target nox cost component is the bottleneck. For rare bottleneck configurations (e.g., programs dominated by invert jet calls), collecting enough training data may require generating synthetic programs specifically designed to stress that jet.

**Meta-selector accuracy**: The meta-selector's prediction of which specialist is best is only as good as the [[trace-predictor]]'s or [[cost-surrogate]]'s accuracy. When the predictor misranks specialists, the ensemble underperforms the best specialist. The meta-selector's accuracy should be measured and optimized independently.

**Specialist diversity**: Specialists trained with similar objectives may converge to similar strategies, reducing ensemble diversity. Explicit diversity regularization (penalize specialists that agree with existing ones on most inputs) helps maintain coverage.

**Inference cost**: Running 16 specialists in parallel requires 16× the single-optimizer latency. For programs where compilation latency is critical (interactive use, `trident watch` mode), 800μs may be noticeable. An early-exit strategy — stop when the trace predictor is confident about the winner — reduces expected latency.

## Implementation Sketch

```rust
// tir/ensemble.rs
pub struct CompilerEnsemble {
    specialists: Vec<Box<dyn TirOptimizer>>,   // each produces a TIR op ordering
    meta_selector: TracePredictor,              // predicts nox proof cost per ordering
}

impl CompilerEnsemble {
    pub fn compile(&self, tir: &TirFunction) -> NoxTrace {
        // Run all specialists in parallel — each produces a TIR op ordering
        let tir_variants: Vec<OrderedTir> = self.specialists
            .par_iter()
            .map(|s| s.optimize(tir))
            .collect();

        // Predict nox proof costs for each TIR ordering (no lowering needed)
        let predicted_costs: Vec<f64> = tir_variants.iter()
            .map(|ordered_tir| {
                let features = extract_tir_features(ordered_tir);
                let costs = self.meta_selector.predict(&features);
                costs.total_proof_cost()  // trace_length + sum(jet_costs)
            })
            .collect();

        // Select minimum predicted cost and lower to nox via warrior-cyber
        let best_idx = predicted_costs.iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        warrior_cyber_lower(&tir_variants[best_idx])  // [[nox]] trace + [[zheng]] proof
    }
}
```

The `par_iter()` uses Rayon for parallel execution. Each specialist runs in a separate thread. The 800μs total latency assumes all specialists complete within 50μs each — the budget for a specialist's forward pass over a typical TIR function. [[learned-peephole]] patterns are applied to the winning TIR ordering before warrior-cyber lowering.
