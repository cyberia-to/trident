---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Multi-Objective Compiler Ensemble

## Motivation

A single TIR→TASM optimizer cannot be optimal for all programs. The STARK cost function — $2^{\lceil \log_2(\max_t H_t) \rceil}$ — depends only on the tallest table. Which table is tallest varies by program. A hash-dominated program needs a different optimization strategy than a processor-dominated one. A balanced program needs yet another strategy. No single optimizer can dominate across all program types.

The ensemble solution: 8–16 specialist optimizers, each tuned to minimize a specific table or objective. For each program, run all specialists in parallel (~800μs combined), use the trace predictor or cost surrogate to predict which specialist's output will prove fastest, and compile only that one. The meta-selector eliminates the need to run the full prover on all 16 variants.

## Design

### Specialist Objectives

Each specialist is a TIR→TASM optimizer with a different fitness function. The specialists cover the space of possible bottleneck scenarios:

```
specialist_0:  minimize max(all tables)           — general optimizer
specialist_1:  minimize H_processor               — arithmetic-heavy programs
specialist_2:  minimize H_hash                    — hash-dominated programs
specialist_3:  minimize H_ram                     — memory-intensive programs
specialist_4:  minimize H_u32                     — bit-manipulation programs
specialist_5:  minimize (H_processor + H_hash)    — joint arithmetic+hash bottleneck
specialist_6:  minimize max_H - second_max_H      — balance optimization
specialist_7:  minimize max(H) subject to H_hash < 512  — hash cliff avoidance
...
specialist_15: minimize total rows (ignoring cliff structure)
```

The specific objectives are tunable. The key principle: cover the space of possible bottleneck configurations so that the true optimal for any program is close to at least one specialist.

### Specialist Architecture

Each specialist is an evolutionary-trained neural compiler:
- Input: TIR graph features
- Output: TASM instruction selection and ordering decisions
- Trained on programs where its target table was the bottleneck
- Parameters: ~728KB (91,000 field elements)

Training: run the evolutionary method on programs curated for each specialist's bottleneck scenario. Specialist 2 (hash-minimizer) trains on programs where Hash is the tallest table after compilation with a naive optimizer.

### Meta-Selection

Running all 16 specialists for every program costs $16 \times 50\mu s \approx 800\mu s$ — negligible compared to proving (seconds). The meta-selector then picks the winner:

```
For each program P:
1. Run all 16 specialists on P's TIR         → 16 TASM variants
2. Run trace predictor on each TASM variant   → 16 predicted AET heights
3. Apply cost function to predicted heights   → 16 predicted costs
4. Select variant with minimum predicted cost → COMPILE THIS ONE
```

Step 2 uses the trace predictor (fast, no execution) rather than the cost surrogate (requires TASM encoding). The predictor runs on the TIR features of each specialist's output, providing table height predictions in microseconds.

Alternatively, for higher accuracy, use the cost surrogate:

```
Step 2': Encode each TASM variant as instruction sequence
         Run cost surrogate on each sequence  → 16 predicted costs
         (cost surrogate: ~50μs per variant × 16 = 800μs additional)
```

Total meta-selection overhead: 800μs + 800μs = 1.6ms. Still negligible.

### Memory Footprint

16 specialists × 728KB = 11.6MB — fits in L2 cache on modern CPUs. All specialists can be resident simultaneously with no cache pressure. The entire ensemble, including the meta-selector, operates within a 15MB working set.

### The Cliff Discontinuity Argument

Why does the ensemble dominate a single optimizer?

The STARK cost function has cliff discontinuities at every power-of-2 table height. The globally optimal optimizer must avoid crossing these cliffs for the specific bottleneck table of each program. But which table is the bottleneck depends on the program — and the bottleneck can shift mid-optimization (reducing Hash rows may make Processor the new bottleneck, requiring a different optimization strategy).

A single optimizer trained on diverse programs learns to balance all tables, which is suboptimal for programs with a dominant bottleneck. Specialist optimizers learn to aggressively minimize their target table, accepting imbalance in other tables. For programs where one table dominates, the specialist wins. For balanced programs, the balance specialist (specialist_6) wins. The meta-selector ensures the right specialist is applied to each program.

### Online Improvement

The ensemble improves online:

1. Each program compiled produces actual proving data: (TIR features, specialist chosen, actual proving time)
2. This data improves the meta-selector's accuracy (train on the ground truth: which specialist was actually cheapest?)
3. Specialists that consistently lose to others on specific program types are retrained on those types
4. New specialists can be added as new bottleneck patterns emerge

The ensemble is not a static artifact — it grows and improves with usage.

## Key Tradeoffs

**Specialist training data**: Each specialist needs training data where its target table is the bottleneck. For rare bottleneck configurations (e.g., programs dominated by U32 operations), collecting enough training data may require generating synthetic programs specifically designed to stress that table.

**Meta-selector accuracy**: The meta-selector's prediction of which specialist is best is only as good as the trace predictor's or cost surrogate's accuracy. When the predictor misranks specialists, the ensemble underperforms the best specialist. The meta-selector's accuracy should be measured and optimized independently.

**Specialist diversity**: Specialists trained with similar objectives may converge to similar strategies, reducing ensemble diversity. Explicit diversity regularization (penalize specialists that agree with existing ones on most inputs) helps maintain coverage.

**Inference cost**: Running 16 specialists in parallel requires 16× the single-optimizer latency. For programs where compilation latency is critical (interactive use, `trident watch` mode), 800μs may be noticeable. An early-exit strategy — stop when the trace predictor is confident about the winner — reduces expected latency.

## Implementation Sketch

```rust
// tir/ensemble.rs
pub struct CompilerEnsemble {
    specialists: Vec<Box<dyn TirToTasmOptimizer>>,
    meta_selector: TracePredictor,
}

impl CompilerEnsemble {
    pub fn compile(&self, tir: &TirFunction) -> TasmProgram {
        // Run all specialists in parallel
        let tasm_variants: Vec<TasmProgram> = self.specialists
            .par_iter()
            .map(|s| s.optimize(tir))
            .collect();

        // Predict costs for each variant
        let predicted_costs: Vec<f64> = tasm_variants.iter()
            .map(|tasm| {
                let features = extract_tir_features_from_tasm(tasm);
                let heights = self.meta_selector.predict_from_features(&features);
                stark_cost(heights)
            })
            .collect();

        // Select minimum predicted cost
        let best_idx = predicted_costs.iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        tasm_variants[best_idx].clone()
    }
}
```

The `par_iter()` uses Rayon for parallel execution. Each specialist runs in a separate thread. The 800μs total latency assumes all specialists complete within 50μs each — the budget for a specialist's forward pass over a typical TIR function.
