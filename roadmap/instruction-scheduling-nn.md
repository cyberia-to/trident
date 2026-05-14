---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Learned Instruction Scheduling

## Motivation

The order of TASM instructions within a dependency-respecting permutation affects AET table heights. Two orderings that are both correct (both respect data dependencies) can produce different table profiles. Interleaving hash instructions with arithmetic instructions forces the Hash table to fragment across the trace, potentially inflating its padded height. Clustering hash instructions reduces fragmentation. But the optimal clustering depends on the specific program — what works for one program may worsen another.

Learned instruction scheduling treats ordering as a machine learning problem. A graph neural network on the TASM dependency DAG predicts a priority score for each instruction. The scheduler executes a greedy topological sort using these priorities. The key property: the scheduler only outputs dependency-respecting permutations — guaranteed by algorithm construction, not by model correctness. Correctness is free. Performance is learned.

## Design

### Problem Formulation

Input: TASM dependency DAG — a directed acyclic graph where each node is a TASM instruction and each edge is a data dependency.

Output: a topological ordering of the DAG (a valid TASM sequence with all dependencies respected).

The scheduler chooses the ordering by priority scores: at each step, the instruction with the highest priority score among all dependency-free candidates is scheduled next. The priority scores are predicted by the GNN.

```
DEPENDENCY DAG:         SCHEDULE (priority-ordered):
  A → C → E            Step 1: {A, B} available → pick highest priority
  B → D → E            Step 2: schedule remaining...
  B → F                ...
```

### GNN Architecture

A Graph Neural Network is the natural architecture for this problem: the input (dependency DAG) is a graph, and the output (per-node priority) is a per-node regression.

```
For each node:
  node_features = [instruction_type, operand_count, stack_depth_at_node,
                   tables_touched, distance_to_root, distance_to_leaf]

Message passing (2 layers):
  h_v^(1) = aggregate(h_u^(0) for u in neighbors(v))
  h_v^(2) = aggregate(h_u^(1) for u in neighbors(v))

Priority prediction:
  priority(v) = MLP(h_v^(2))
```

Parameters: approximately 20,000 field elements. Inference: fast (milliseconds, even on large programs). The GNN runs on the CPU-side compilation infrastructure, not on Triton VM — it is a compilation tool, not a Trident program.

### What the GNN Learns

Training reveals three robust scheduling heuristics that the GNN discovers without being explicitly taught:

**Hash clustering**: Hash instructions (which write to the Hash table) should be grouped together. Interleaved hash + arithmetic forces the Hash table to stay active throughout a long trace segment, inflating its height. Clustered hash keeps the Hash table active only in a contiguous segment.

**Front-loading RAM**: RAM read instructions should be scheduled early to fill the memory pipeline. Delaying RAM reads until they are needed creates sequential dependencies that serialize what could be parallel execution.

**Delaying U32 operations**: U32 table operations are often not on the critical path. Scheduling them late, after main arithmetic is complete, avoids inflating U32 table height during the dense arithmetic phase.

The GNN learns these heuristics from data — thousands of (scheduling → table heights) pairs — without any of these rules being explicitly programmed.

### Training Protocol

For each training program:
1. Generate 1,000 random valid schedules (random topological sorts of the DAG)
2. Execute each schedule and measure actual AET table heights
3. The target: the schedule that minimizes max table height
4. Train the GNN to reproduce this schedule's priority order

Loss function: pairwise ranking loss over instruction priorities (instruction A should have higher priority than B if scheduling A before B reduces table height).

```
Training data: {(dag, best_schedule, random_schedules): 10,000 programs × 1,000 schedules}
Total proving runs: 10M  (each takes ~10ms → ~28 hours total compute)
```

This training compute is a one-time cost. The trained GNN is then applied to every new program at compile time.

### Correctness Guarantee

The scheduler is correct by construction: the greedy topological sort with any priority function always produces a valid dependency-respecting ordering. The GNN affects performance, not correctness. Even a randomly initialized GNN (before training) produces correct TASM — just not optimally ordered.

This makes the scheduler safe to deploy incrementally: start with any set of weights (even random), deploy, collect performance data, retrain, improve. No risk of correctness regression.

### Integration with Compiler Ensemble

The scheduler integrates with the multi-objective compiler ensemble (`compiler-ensemble.md`). Each specialist optimizer in the ensemble can use the GNN scheduler with different learned priorities:

- Specialist 1 (minimize Processor): learns priorities that front-load Processor-heavy instructions to cluster them
- Specialist 2 (minimize Hash): learns priorities that cluster Hash instructions
- ...

One GNN architecture, multiple learned priority functions. The ensemble uses the cost surrogate to select which specialist's schedule minimizes total proof cost.

## Key Tradeoffs

**Greedy suboptimality**: Greedy topological sort with learned priorities is not globally optimal. The optimal schedule requires lookahead — scheduling instruction A first might free instruction B which has the highest local priority later. Without lookahead, the greedy scheduler may miss globally optimal orderings.

**Training data scale**: 10 million proving runs for training data generation is computationally expensive. A faster proxy — using the cost model or trace predictor rather than actual proving — reduces this cost at the expense of training accuracy.

**Dynamic priorities**: The GNN assigns static priorities before scheduling begins. But the optimal priority of an instruction may depend on what other instructions have already been scheduled (dynamic scheduling). The GNN approximates this with context from the DAG structure, but pure static priority is an approximation.

**Program-specific vs. general**: A single GNN trained on diverse programs learns general scheduling principles. For specific program families (e.g., always hash-dominated programs), a specialized GNN might perform better. The compiler ensemble handles this: each specialist GNN is trained on programs where that table is the bottleneck.

## Implementation Sketch

The GNN scheduler is implemented as a Rust library (not a Trident program — it is a compiler tool, not a runtime computation):

```rust
// tir/scheduling/gnn.rs
pub struct SchedulingGNN {
    node_embed: Linear,       // instruction features → 32-dim
    message_1: Linear,       // message passing layer 1
    message_2: Linear,       // message passing layer 2
    priority_head: Linear,   // 32-dim → scalar priority
}

impl SchedulingGNN {
    pub fn schedule(&self, dag: &TasmDag) -> Vec<InstructionId> {
        let mut features = dag.nodes().map(|n| self.extract_features(n)).collect();
        let embeddings = self.message_pass(&features, dag, 2);
        let priorities: Vec<f32> = embeddings.iter().map(|e| self.priority_head.forward(e)).collect();
        greedy_topological_sort(dag, &priorities)
    }
}

fn greedy_topological_sort(dag: &TasmDag, priorities: &[f32]) -> Vec<InstructionId> {
    let mut result = Vec::new();
    let mut available = dag.roots().collect::<BinaryHeap<_>>();
    while let Some(next) = available.pop_max_by(|id| priorities[*id]) {
        result.push(next);
        for successor in dag.successors(next) {
            if dag.all_predecessors_scheduled(successor, &result) {
                available.push(successor);
            }
        }
    }
    result
}
```

The GNN weights are loaded from a file trained offline. The scheduler runs in microseconds per program — negligible compile time overhead.
