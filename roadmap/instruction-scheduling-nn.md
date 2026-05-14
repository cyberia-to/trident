---
status: draft
author: mastercyb
area: AI
planned: 128K
---

# Learned TIR Op Scheduling

## Motivation

The order of TIR ops within a dependency-respecting permutation affects [[nox]] proof cost. Two orderings that are both correct (both respect data dependencies) can produce different trace profiles. Interleaving hash-calling TIR ops with arithmetic ops forces the hash jet ([[hemera]]) to be invoked in scattered positions across the [[nox]] trace, potentially inflating its contribution to proof cost. Clustering hash-calling ops reduces jet invocation fragmentation. But the optimal clustering depends on the specific program — what works for one program may worsen another.

The scheduler works on the TIR dependency DAG to produce the [[nox]] trace with the lowest cost. TIR provides the dependency structure — the 54-op DAG where correctness constraints are visible and provable; [[nox]] trace length is the objective. Reordering TIR ops within the dependency constraints produces different [[nox]] traces when lowered by [[warrior-cyber]].

Learned TIR op scheduling treats ordering as a machine learning problem. A graph neural network on the TIR dependency DAG predicts a priority score for each TIR op. The scheduler executes a greedy topological sort using these priorities. The key property: the scheduler only outputs dependency-respecting permutations — guaranteed by algorithm construction, not by model correctness. Correctness is free. Performance is learned.

Related proposals: [[cost-surrogate]], [[compiler-ensemble]], [[learned-peephole]].

## Vision

TIR op scheduling is invisible to the developer but visible in focus costs. A program with optimal scheduling costs 30% less focus than one with naive ordering — same output, same correctness, different economic reality. In the cyber network, where [[bbg]] charges focus for every [[nox]] step, this 30% reduction is the difference between a viable program and an uneconomical one. The scheduling GNN makes every Trident developer an expert optimizer without requiring them to understand the details.

Stack integration: The scheduling GNN operates at TIR level before [[warrior-cyber]] lowering. Its decisions affect the [[nox]] trace structure, which affects the Brakedown commitment shape in [[zheng]], which affects proof size and verification cost. The optimal schedule minimizes focus cost across the full pipeline. Scheduling decisions and their resulting trace costs are recorded in [[cybergraph]] as cyberlinks — each (DAG structure, chosen schedule, actual cost) triple contributes to the GNN's training corpus automatically.

## Design

### Problem Formulation

Input: TIR dependency DAG — a directed acyclic graph where each node is a TIR op (one of 54 ops, see `../reference/ir.md`) and each edge is a data or control dependency (DataDep, ControlFlow, MemOrder — matching the edge types in the neural compiler's TirGraph, see `../reference/neural.md`).

Output: a topological ordering of the DAG (a valid TIR op sequence with all dependencies respected). warrior-cyber then lowers this ordered sequence to a nox trace.

The scheduler chooses the ordering by priority scores: at each step, the TIR op with the highest priority score among all dependency-free candidates is scheduled next. The priority scores are predicted by the GNN.

```
DEPENDENCY DAG:         SCHEDULE (priority-ordered):
  A → C → E            Step 1: {A, B} available → pick highest priority
  B → D → E            Step 2: schedule remaining...
  B → F                ...
```

### GNN Architecture

A Graph Neural Network is the natural architecture for this problem: the input (dependency DAG) is a graph, and the output (per-node priority) is a per-node regression.

```
For each node (TIR op):
  node_features = [op_kind (54 kinds), jet_invoked (bool), hash_jet (bool),
                   estimated_nox_cost, distance_to_root, distance_to_leaf]

Message passing (2 layers):
  h_v^(1) = aggregate(h_u^(0) for u in neighbors(v))
  h_v^(2) = aggregate(h_u^(1) for u in neighbors(v))

Priority prediction:
  priority(v) = MLP(h_v^(2))
```

Parameters: approximately 20,000 field elements. Inference: fast (milliseconds, even on large programs). The GNN runs on the CPU-side compilation infrastructure, not on nox — it is a compilation tool, not a Trident program.

This architecture is a simplified variant of the GNN encoder in `../reference/neural.md` (GATv2, d=256). The scheduling GNN uses a lighter 2-layer message-passing network (d=32) since it only needs per-node priority scalars, not full node embeddings for decoding.

### What the GNN Learns

Training reveals three robust scheduling heuristics that the GNN discovers without being explicitly taught:

**Hash jet clustering**: TIR ops that invoke the hash jet ([[hemera]]/Poseidon2) should be grouped together. Interleaved hash + arithmetic scatters [[hemera]] invocations across the [[nox]] trace; clustered hash minimizes the jet invocation overhead contribution to proof cost.

**Front-loading memory ops**: TIR ops that result in RAM read instructions when lowered to nox should be scheduled early. Delaying them creates sequential nox reduction dependencies that inflate trace length unnecessarily.

**Deferring cheap arithmetic**: Low-cost arithmetic TIR ops are often not on the critical path. Scheduling them late, after jet-invoking ops, avoids inflating trace length during the high-jet-cost phase where bottleneck tracking matters most.

The GNN learns these heuristics from data — thousands of (scheduling → nox trace lengths) pairs — without any of these rules being explicitly programmed.

### Training Protocol

For each training program:
1. Generate 1,000 random valid TIR op orderings (random topological sorts of the DAG)
2. Lower each ordering to a [[nox]] trace via [[warrior-cyber]] and measure actual [[nox]] proof cost (`trace_length + sum(jet_costs)`)
3. The target: the ordering that minimizes [[nox]] proof cost
4. Train the GNN to reproduce this ordering's priority assignment

Loss function: pairwise ranking loss over TIR op priorities (op A should have higher priority than B if scheduling A before B reduces nox proof cost).

```
Training data: {(dag, best_schedule, random_schedules): 10,000 programs × 1,000 schedules}
Total proving runs: 10M  (each takes ~10ms → ~28 hours total compute)
```

The [[cost-surrogate]] can substitute for actual proving runs during training data collection, reducing compute from ~28 hours to ~minutes at the cost of some accuracy.

This training compute is a one-time cost. The trained GNN is then applied to every new program at compile time.

### Correctness Guarantee

The scheduler is correct by construction: the greedy topological sort with any priority function always produces a valid dependency-respecting TIR ordering. The GNN affects performance, not correctness. Even a randomly initialized GNN (before training) produces a correct nox trace when warrior-cyber lowers the ordering — just not optimally efficient.

This makes the scheduler safe to deploy incrementally: start with any set of weights (even random), deploy, collect performance data, retrain, improve. No risk of correctness regression.

### Integration with Compiler Ensemble

The scheduler integrates with the [[compiler-ensemble]]. Each specialist optimizer in the ensemble can use the GNN scheduler with different learned priorities:

- Specialist 1 (minimize trace_length): learns priorities that cluster pure-arithmetic TIR ops
- Specialist 2 (minimize hash jet calls): learns priorities that cluster [[hemera]]-invoking TIR ops
- Specialist 3 (minimize poly_eval jet calls): learns priorities that cluster poly_eval-invoking TIR ops
- ...

One GNN architecture, multiple learned priority functions. The ensemble uses the [[cost-surrogate]] to select which specialist's schedule minimizes total nox proof cost.

## Key Tradeoffs

**Greedy suboptimality**: Greedy topological sort with learned priorities is not globally optimal. The optimal schedule requires lookahead — scheduling instruction A first might free instruction B which has the highest local priority later. Without lookahead, the greedy scheduler may miss globally optimal orderings.

**Training data scale**: 10 million nox+zheng proving runs for training data generation is computationally expensive. A faster proxy — using the [[cost-surrogate]] or [[trace-predictor]] rather than actual warrior-cyber proving — reduces this cost at the expense of training accuracy.

**Dynamic priorities**: The GNN assigns static priorities before scheduling begins. But the optimal priority of an instruction may depend on what other instructions have already been scheduled (dynamic scheduling). The GNN approximates this with context from the DAG structure, but pure static priority is an approximation.

**Program-specific vs. general**: A single GNN trained on diverse programs learns general scheduling principles. For specific program families (e.g., always hash-dominated programs), a specialized GNN might perform better. The compiler ensemble handles this: each specialist GNN is trained on programs where that table is the bottleneck.

## Implementation Sketch

The GNN scheduler is implemented as a Rust library (not a Trident program — it is a compilation tool, not a runtime computation). It operates on the TirGraph structure (see `../reference/neural.md` §"Data Flow" for the TirGraph definition):

```rust
// tir/scheduling/gnn.rs
pub struct SchedulingGNN {
    node_embed: Linear,       // TIR op features → 32-dim (op_kind, jet_invoked, etc.)
    message_1: Linear,        // message passing layer 1
    message_2: Linear,        // message passing layer 2
    priority_head: Linear,    // 32-dim → scalar priority
}

impl SchedulingGNN {
    pub fn schedule(&self, dag: &TirDag) -> Vec<TirOpId> {
        let mut features = dag.nodes().map(|n| self.extract_features(n)).collect();
        let embeddings = self.message_pass(&features, dag, 2);
        let priorities: Vec<f32> = embeddings.iter().map(|e| self.priority_head.forward(e)).collect();
        greedy_topological_sort(dag, &priorities)
    }
}

fn greedy_topological_sort(dag: &TirDag, priorities: &[f32]) -> Vec<TirOpId> {
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

The scheduled TIR op ordering is then passed to [[warrior-cyber]] for lowering to a [[nox]] trace. The [[learned-peephole]] optimizer can further refine the TIR sequence before lowering.

The GNN weights are loaded from a file trained offline. The scheduler runs in microseconds per program — negligible compile time overhead.
