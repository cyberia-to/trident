---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Predictive Trace Cost Analysis

## Motivation

Every optimization decision in the compiler depends on understanding the cost landscape: what the [[nox]] trace length will be, how many jet invocations each candidate transformation produces, and which cost component is the bottleneck. In [[nox]]/[[zheng]], `proof_cost = trace_length + sum(jet_costs)` — jet costs are fixed per-jet (hash jet via [[hemera]] is the most expensive), but trace length is program-dependent and only known after full lowering.

Currently, the compiler must fully lower a TIR function to a [[nox]] trace and count reduction steps + jet calls to measure actual proof cost. This is expensive for interactive tools and for optimization passes that need to evaluate multiple candidate transformations.

A small neural network that predicts [[nox]] trace length and jet invocation counts from TIR features — before lowering and execution — changes this. Cost estimation becomes a millisecond operation. The compiler can evaluate hundreds of candidate transformations, pick the predicted cheapest one, and then lower only that one. Interactive tools (REPL, IDE) get cost estimates without any lowering at all.

Related proposals: [[nn-trd]], [[cost-surrogate]], [[compiler-ensemble]]. Reference: `../reference/neural.md` (canonical neural compiler spec).

## Vision

The trace cost predictor is the economic oracle of the cyber network. Before [[bbg]] executes any program, it can query the predictor: "how much focus will this cost?" The predictor is a small [[nn-trd]] network, running on [[nox]], whose output is a [[zheng]]-proved estimate. Focus pricing becomes predictable. Users see cost estimates before submitting. The trace predictor turns the focus market from an opaque auction into a transparent quoted price.

The predictor's training data is the [[cybergraph]] itself — every computation ever run has its actual cost recorded as a cyberlink. The predictor learns from all of history. As more programs run, the predictor improves. The [[cybergraph]]'s accumulation of execution data directly improves the quality of the economic oracle.

Stack integration: [[soft3]]'s `query()` call can use the trace predictor to estimate focus cost before `submit()`. [[bbg]] can use the predictor for focus budget pre-allocation. The predictor is deployed as an [[Atlas]] package with version-stamped training checkpoints — each checkpoint a particle in [[cybergraph]], provably derived from all execution data up to that point.

## Design

### Input Features

The predictor takes TIR graph features as input — approximately 32 field elements:

| Feature | Description |
|---------|-------------|
| node_count | Total TIR nodes in the function |
| op_histogram[16] | Count of each major operation type (mul, add, hash, invert, ...) |
| max_nesting_depth | Maximum loop/branch nesting depth |
| branch_count | Total number of conditional branches |
| loop_bound_sum | Sum of all loop iteration counts |
| memory_access_count | Number of RAM read/write operations |
| has_hash | Binary: does the function call any hash? |
| has_invert | Binary: does the function call invert? |
| max_array_size | Largest static array size |

These features are extractable from the TIR in O(n) time without any execution.

### Output

The nox/zheng cost model does not have 9 fixed AET tables. Proof cost has two components:

- **trace_length**: total number of nox reduction pattern applications
- **jet_costs**: per-jet invocation counts × fixed jet cost

The predictor outputs these as separate values:

```
[trace_length, jet_hash_count, jet_poly_eval_count, jet_invert_count, jet_ntt_count, jet_other_count]
```

Six field elements in log-scale (predicting log2 of the value) to compress the range. Total predicted cost = predicted_trace_length + sum(predicted_jet_count[i] × jet_cost[i]).

The `hash` jet is the most expensive: it invokes [[hemera]] (Poseidon2), which dominates proof cost for hash-heavy programs.

### Model Architecture

Single hidden layer, 64 units, field-native implementation in [[nn-trd]]:

```trident
// trace_predictor.trd
fn predict_trace_costs(features: Vector<32>) -> Vector<6> {
    // Layer 1: 32 → 64 (with ReLU)
    let h1 = relu(linear(W1, b1, features));   // W1: Matrix<64, 32>
    // Layer 2: 64 → 6 (trace_length + 5 jet counts)
    let out = linear(W2, b2, h1);              // W2: Matrix<6, 64>
    out  // log-scale: [trace_length, jet_hash, jet_poly_eval, jet_invert, jet_ntt, jet_other]
}
```

Parameters: $64 \times 32 + 64 + 6 \times 64 + 6 = 2048 + 64 + 384 + 6 = 2502$ field elements ≈ ~20 KB. Inference: ~2,700 nox steps. Trivial.

Trained by [[evolutionary-training]]. The model itself is a [[nn-trd]] network, making its inference a provable [[nox]] trace proved by [[zheng]].

### Training Data Collection

Training data is collected during compilation: for each compiled function, record (TIR features, actual nox trace costs). Start with ~1,000 programs; scale to ~100,000 as the program corpus grows.

```rust
// compiler: collect training pairs during compilation
fn compile_and_record(source: &TridentSource, dataset: &mut Dataset) {
    let tir = lower_to_tir(source);
    let features = extract_features(&tir);
    let trace = lower_to_nox_trace(&tir);  // warrior-cyber lowers TIR → nox
    let costs = measure_trace_costs(&trace);  // trace_length + per-jet counts
    dataset.push(TrainingSample { features, costs });
}
```

Training runs on the collected dataset using [[evolutionary-training]] — a self-referential loop where the predictor itself is trained using the same evolutionary method it will later assist.

### Accuracy Targets

- **Bottleneck component identification** (trace_length vs. hash jets): correct >90% of the time
- **Exact cost**: within 20% of actual nox proof cost
- **Ranking**: correctly rank two implementations by proof cost >95% of the time

The ranking target is most important for compiler use — the predictor is used to choose between optimization candidates, not to predict absolute costs.

### Uses

1. **Compilation cost estimation**: estimate proof cost of any TIR function without nox lowering
2. **Optimization candidate selection**: rank 8–16 optimization variants, lower only the predicted winner
3. **CI/CD fast path**: estimate cost gate compliance without full proving by zheng
4. **REPL cost hints**: show cost estimates without any compilation
5. **[[algebraic-identity-explorer]] usefulness scorer**: provides jet_criticality weights dynamically
6. **[[cost-surrogate]] input**: predicted trace costs feed the surrogate as additional features
7. **[[compiler-ensemble]] meta-selector**: ranks specialist outputs in microseconds before committing to one

## Key Tradeoffs

**Feature completeness**: The 32 input features capture the most important cost drivers but miss subtle interactions. Programs with unusual instruction mixes (e.g., heavy U32 operations from bit manipulation, or deep Lookup table chains) may be poorly predicted. The feature set should grow as the corpus reveals new cost drivers.

**Generalization**: The predictor trained on 1,000 programs may overfit to the programs' stylistic patterns. A diverse corpus (cryptographic code, neural inference, sorting algorithms, arithmetic circuits) is essential for generalization. Adversarial programs (generated by the adversarial compiler — separate proposal) stress-test generalization.

**Update frequency**: As the compiler's optimization passes improve, the relationship between TIR features and nox trace costs changes. The predictor must be retrained periodically (at least after each major pass improvement). An online learning approach (update continuously as new programs are compiled) would maintain accuracy without periodic retraining.

**Cold start**: The first 1,000 programs must be compiled without the predictor. The compiler falls back to the cost model (approximation, always available) during the cold start period. Once the predictor is trained, it replaces the cost model for most purposes.

## Implementation Sketch

```rust
// cost/trace_predictor.rs
pub struct TracePredictor {
    weights: NnWeights,  // trained by evolutionary method (evolutionary-training.md)
}

impl TracePredictor {
    pub fn predict(&self, tir: &TirFunction) -> NoxTraceCosts {
        let features = extract_features(tir);
        let log_costs = self.weights.infer(&features);
        NoxTraceCosts::from_log_scale(log_costs)
        // returns: trace_length + per-jet invocation counts
    }

    pub fn train(dataset: &[(Features, NoxTraceCosts)]) -> Self {
        let evolved = evolutionary_train(dataset, N_GENERATIONS);
        TracePredictor { weights: evolved }
    }
}

fn extract_features(tir: &TirFunction) -> Vector<32> {
    let mut f = Vector::zero();
    f[0] = tir.node_count() as Field;
    for node in tir.nodes() {
        f[1 + node.op_id()] += 1;  // op histogram
    }
    f[17] = tir.max_nesting_depth() as Field;
    f[18] = tir.branch_count() as Field;
    f[19] = tir.loop_bound_sum() as Field;
    // ... remaining features
    f
}
```

The predictor is a small [[nn-trd]] network trained by [[evolutionary-training]], predicting [[nox]] trace costs (trace_length + jet invocation counts) that guide every other optimization decision in the system. It is foundational — schedule its implementation early in the 128K milestone. See `../reference/neural.md` for the canonical neural compiler architecture this integrates with.
