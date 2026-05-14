---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# NN-Guided STARK Prover Configuration

## Motivation

The STARK prover has multiple tunable parameters. Current practice: use a single fixed configuration for all programs. This leaves performance on the table — different programs benefit from different configurations. A hash-dominated program with a tall Hash table may benefit from a different FRI folding factor than a processor-dominated program. A program running on a machine with many CPU cores benefits from a different parallelism strategy than one running on a single-core VM.

A learned configuration selector — an MLP that takes program features and hardware specs as input and predicts the optimal prover configuration — can reduce proving time by 10–30% without any changes to the STARK protocol itself. The proof remains valid regardless of which configuration is chosen; the agent affects only speed.

## Design

### Configurable Parameters

| Parameter | Current | Range | Impact |
|-----------|---------|-------|--------|
| FRI folding factor | Fixed (8) | 2, 4, 8, 16 | Proof size vs. verification speed |
| Grinding bits | Fixed (20) | 16–28 | Security level vs. proving time |
| Blowup factor | Fixed (4) | 2, 4, 8 | Soundness vs. time/size |
| Evaluation domain offset | Default coset | Multiple choices | Numerical stability |
| Memory layout | Sequential | Sequential, cache-optimized, NUMA-aware | Cache performance |
| Parallelism strategy | Default threading | Per-table, per-column, per-row | Hardware utilization |
| Hash function selection | Tip5 (fixed) | Tip5, Blake3 (for non-ZK commitments) | Commitment speed |

Each parameter affects proving time differently for different programs. The optimal configuration depends on the AET profile (which tables are tall), the hardware (core count, cache sizes), and the security requirements.

### Model Architecture

A simple MLP — adequate for the input dimensionality:

```
Input features (~50 values):
  - AET table heights (9 values)
  - TIR operation histogram (16 values)
  - Hardware specs: core_count, L2_cache_mb, memory_bandwidth_gbps (3 values)
  - Security requirements: grinding_bits_min, blowup_factor_min (2 values)
  - Historical performance on similar programs: avg_prove_time_ms (1 value)
  - Program fingerprint: hash of dominant instruction pattern (hashed to 16 values)

→ Dense(64) → ReLU
→ Dense(32) → ReLU
→ Dense(N_CONFIGS)  → softmax over discrete config choices
```

Parameters: ~10,000 field elements. Inference: negligible. Runs in microseconds before proving begins.

### Configuration Space

Rather than predicting raw parameter values (which would require continuous optimization), the agent selects from a discrete set of pre-validated configurations:

```rust
const CONFIGS: &[ProverConfig] = &[
    ProverConfig { fri_fold: 8, grind: 20, blowup: 4, layout: Sequential, ... },
    ProverConfig { fri_fold: 16, grind: 20, blowup: 4, layout: Sequential, ... },
    ProverConfig { fri_fold: 8, grind: 24, blowup: 8, layout: CacheOptimized, ... },
    // ... up to 64 pre-validated configurations
];
```

All configurations in the set are guaranteed valid (produce correct proofs). The agent selects the best one. This avoids the risk of the agent selecting an invalid configuration.

### Training Protocol

Reinforcement learning:
- State: (program features, hardware specs)
- Action: select a configuration from the discrete set
- Reward: $-\text{proving\_time}$ (negative because we minimize)

Data collection: for each training program, run proving with all (or a random subset of) configurations, record (program, hardware, config, proving_time) tuples. Train the MLP to predict minimum-time configuration.

Alternatively, supervised learning from the collected data: label each (program, hardware) pair with the empirically best configuration, train MLP as a classifier.

The supervised approach is simpler to implement and may generalize adequately for the program distributions encountered in practice.

### Safety Guarantee

The agent cannot compromise soundness. All configurations in the discrete set are validated by the Trident/trisha developers. The agent selects from this set only. If the agent's selected configuration causes the prover to fail (due to edge cases not covered during validation), the prover falls back to the default configuration and logs the anomaly.

```rust
// prover/config.rs
pub fn select_and_prove(program: &TasmProgram, agent: &ConfigAgent) -> StarkProof {
    let features = extract_features(program);
    let config = agent.select(features);

    match prove_with_config(program, &config) {
        Ok(proof) => proof,
        Err(ProverError) => {
            log::warn!("Agent-selected config failed, falling back to default");
            prove_with_config(program, &DEFAULT_CONFIG).expect("default must not fail")
        }
    }
}
```

### Expected Impact

Based on manual tuning experiments with STARK provers:
- FRI folding factor optimization: 5–15% proving time reduction for proof-size-insensitive programs
- Parallelism strategy: 10–20% for programs with parallelizable table fills
- Memory layout: 5–10% for memory-bandwidth-limited programs

Combined: 10–30% total proving time reduction on the programs where the agent is most beneficial. Programs where the default configuration is near-optimal see minimal improvement.

### Online Adaptation

As programs and hardware evolve, the agent should continue to learn. Online RL: each proved program provides an immediate reward signal (actual proving time). The agent updates its policy continuously. New configurations can be added to the discrete set as the prover evolves.

## Key Tradeoffs

**Configuration validation cost**: All configurations in the discrete set must be validated (proven correct for a large test suite). Adding new configurations requires new validation — a one-time cost per configuration.

**Generalization to new hardware**: An agent trained on M4 Pro may not generalize well to a server with 128 cores or a VM with limited memory. Hardware specifications in the input features help, but the agent should be fine-tuned on each deployment target.

**Proving time measurement**: Training requires measuring actual proving times. For programs that take seconds to prove, collecting thousands of (config, time) measurements requires significant compute. A faster proxy — using the cost surrogate to predict relative proving times across configurations — reduces this cost at the expense of accuracy.

**Interaction with other optimizations**: The optimal prover configuration depends on the TASM being proved. If the compiler optimizes the TASM (reducing Hash rows, reordering instructions), the optimal configuration may change. The agent must be applied after compilation is complete.

## Implementation Sketch

```rust
// prover/config_agent.rs
pub struct ConfigAgent {
    mlp: MlpWeights,  // trained, ~10K params
    config_set: Vec<ProverConfig>,
}

impl ConfigAgent {
    pub fn select(&self, features: &ProgramFeatures) -> &ProverConfig {
        let input = features.to_field_vector();
        let logits = self.mlp.forward(&input);
        let best_idx = logits.argmax();
        &self.config_set[best_idx]
    }
}

pub struct ProgramFeatures {
    aet_heights: [u64; 9],
    op_histogram: [u32; 16],
    hardware: HardwareSpec,
    security_req: SecurityRequirements,
}

impl ProgramFeatures {
    fn to_field_vector(&self) -> Vec<FieldElement> {
        // Encode all features as field elements for MLP input
        let mut v = Vec::with_capacity(50);
        v.extend(self.aet_heights.iter().map(|h| FieldElement::from(h.ilog2())));
        // ... other features
        v
    }
}
```

The `ConfigAgent` is a lightweight wrapper around a small MLP. It is instantiated once per compilation session and reused for all programs in the session. The MLP weights are loaded from a pre-trained file (updated periodically as new training data accumulates).
