---
status: draft
author: mastercyb
area: tooling
planned: 32K
---

# Proof-Cost IDE Integration and CI/CD Gates

**Related proposals:** [[proof-cost-types]], [[proof-explorer]], [[trace-predictor]], [[trident-repl]]
**Reference:** [reference/cli.md — trident bench](../reference/cli.md)

## Motivation

Proof cost is invisible in conventional development environments. A developer writes a function, it looks correct, it passes tests, they ship it. Three weeks later, performance profiling reveals that the function dominates proving time because it calls a hash function in a loop. The fix is obvious in retrospect — batch the hashes — but nothing in the development environment pointed to the problem.

Proof cost must be visible during development, not after deployment. IDE integration that colours every line by its nox reduction step cost makes the expensive lines impossible to ignore. CI/CD gates that reject PRs exceeding declared cost limits make proof cost a regression criterion with the same weight as test failures.

## Design

### Per-Line Cost Highlighting

The IDE extension queries the compiler's cost model for each line of Trident code and colours it accordingly. Cost is measured in nox reduction steps (and jet invocations where relevant) — the fundamental unit of proof cost in the nox/zheng stack:

```
fn example(x: Field) -> Field {
  let a = x * x;              // [green]  1 nox step
  let b = hash(a);            // [red]    ~200 nox steps (hemera Poseidon2 — bottleneck)
  let c = a + b;              // [green]  1 nox step
  let d = invert(c);          // [yellow] ~95 nox steps
  d
}
// Tooltip on hash: "hemera hash: ~198 nox steps (67% of function trace)
//                  Suggestion: batch with other hash calls"
```

Color scheme:
- Green: cost below 10% of function's total trace length
- Yellow: cost 10–30% of function's total trace length
- Red: cost above 30% of function's total trace length — the bottleneck

The highlighting updates incrementally as the developer edits code, using the fast TIR cost model (not full proving). The cost estimate is approximate but directionally correct — sufficient for identifying hotspots. See [[trace-predictor]] for the neural model that can supply faster estimates.

### LSP Integration

The cost highlighting is exposed via the Language Server Protocol. The IDE queries the Trident LSP (`trident lsp`) for:

- Hover information: "This line costs X nox steps (Y jet invocations)"
- Inline hints: `// cost: ~300 rows` annotations
- Diagnostic messages for lines above a configurable cost threshold
- Quick fixes: "Replace `invert(x)` with `batch_invert([x, y, z])`" when multiple inversions are detected nearby

The LSP computes costs using the same cost model as the compiler, so hover costs match compile-time analysis exactly. The cost model reports nox reduction steps and jet invocations; the hover text translates these to human-readable terms (e.g. "hemera hash call: ~198 steps").

### CI/CD Cost Gates

A project-level configuration file defines proof cost limits. Costs are expressed in nox reduction steps and trace length (the two dimensions zheng cares about), not AET table rows (which are a Triton VM concept):

```yaml
# trident-ci.yml
proof_cost_limits:
  transfer_circuit:
    nox_steps: 4096
    trace_length: 4096   # must be a power of 2; zheng pads to next power of 2
    jets: 64
  
  verify_signature:
    nox_steps: 2048
    trace_length: 2048

# CI behavior on violation:
on_violation: reject  # or: warn | annotate
report_format: github_pr_comment  # or: json | text
```

The scoreboard for CI cost gates is `trident bench` (`reference/cli.md`), which already tracks nox step counts and trace lengths for baseline programs. CI gates extend this to named circuits with explicit limits. `trident bench` runs against the `baselines/triton/` hand-optimized floor — CI rejects PRs that regress against either the declared limits or the baselines:

```
CI CHECK: proof_cost_limits
  transfer_circuit:
    nox_steps:    4312 / 4096  [FAIL: +216 steps, +5.3%]
    trace_length: 4096 / 4096  [PASS — no cliff crossed]
    jets:         51 / 64      [PASS]
  
  FAILED. PR rejected.
  Suggestion: The step overflow is in function `compute_fee` (line 47).
              Consider using batch_invert for the 3 invert() calls.
```

### Regression Detection

CI gates catch regressions, not just absolute violations. If a PR increases a circuit's nox step count by more than a configurable percentage (say 5%), CI flags it even if the absolute cost is within limits:

```yaml
proof_cost_limits:
  transfer_circuit:
    nox_steps: 4096
    max_regression_pct: 5  # reject if nox step count increases by >5%
```

This prevents gradual cost inflation: each PR adds a few steps, stays within limits, but over 50 PRs the total increases 50%. Regression detection catches this incrementally. The `trident bench` scoreboard tracks the baseline trend across commits.

### IDE Integration Architecture

The IDE integration is implemented as a Trident LSP extension:

```
Editor (Zed/VSCode/Helix) ← LSP protocol → trident lsp ← cost model → TIR cost analysis
```

The LSP server:
1. Receives file change notifications from the editor
2. Incrementally recompiles the changed file to TIR (fast — skips nox codegen and proving)
3. Runs the TIR cost model to estimate nox reduction steps per line
4. Returns semantic token colours and hover text to the editor

The incremental TIR compilation ensures that cost highlighting updates quickly (target: <100ms for single-line edits in typical files).

## Vision

When [[bbg]] focus pricing becomes visible in the editor, smart contract economics are no longer emergent surprises — they are first-class design constraints. A developer sees every line annotated inline: "this [[hemera]] call costs 32 [[nox]] steps — the rest of your program costs 18 total." The bottleneck is highlighted in red before it reaches production. Before a package reaches [[Atlas]], the CI gate rejects it if the declared focus bound is exceeded.

The [[cybergraph]] makes this richer than a local cost model alone. The IDE queries the graph for existing benchmarks of similar programs. If `hemera(x)` has been profiled before — its trace is a particle in the graph — the IDE shows the actual [[nox]] step count from that particle, not a model estimate. The more programs the ecosystem runs, the more accurate the inline annotations become. Cost intelligence accumulates in the knowledge graph and is shared across every developer's IDE simultaneously.

The CI gate closes the loop: packages whose focus cost exceeds the declared bound are rejected before they reach [[Atlas]]. Focus economy becomes legible. The developer who once shipped a program that dominated proving time because it called a hash function in a loop now sees that problem on line 1, highlighted red, before writing line 2.

## Stack Integration

The IDE queries [[cybergraph]] particles via [[soft3]]'s `query(cid, dimension)` to fetch previously measured [[nox]] step counts for known function signatures. [[hemera]] CIDs identify function variants: if `hemera(source_text)` matches a particle in the graph, the trace data is fetched directly rather than estimated. The [[bbg]] focus budget constraint is the unit the CI gate enforces — the same unit the state machine uses on-chain. IDE, CI, and runtime speak the same accounting language.

## Key Tradeoffs

**Cost model accuracy**: The TIR cost model is an approximation. It does not run proving — it estimates from the instruction mix. Accuracy is typically within 10–20% of actual proving cost. This is sufficient for IDE guidance but may misclassify borderline cases in CI gates. The CI gate should use a conservative estimate (upper bound) to avoid false passes.

**Incremental update cost**: Recomputing cost highlighting on every keystroke is too expensive. The LSP debounces — cost highlighting updates after 200ms of inactivity, not on every character. This is imperceptible to the developer.

**Context dependence**: A function's cost depends on its call context (which parts of the trace are already occupied by the caller). The LSP shows per-function cost in isolation, which may differ from the cost in context. The developer should use [[proof-explorer]] for context-aware cost analysis.

**CI performance**: Running full cost analysis for every CI job adds time to the CI pipeline. For large codebases with many circuits, the analysis may take significant time. Incremental CI (only re-analyze changed circuits) is needed for large projects.

## Implementation Sketch

```rust
// lsp/cost_hints.rs
pub fn compute_line_costs(
    file: &TridentFile,
    tir: &TirModule,
    cost_model: &CostModel,
) -> Vec<LineCost> {
    let line_spans = tir.source_spans();
    line_spans
        .iter()
        .map(|span| {
            let tir_nodes = tir.nodes_at_span(span);
            let cost = tir_nodes.iter().map(|n| cost_model.estimate(n)).sum();
            LineCost { line: span.start_line, cost }
        })
        .collect()
}

// ci/cost_gates.rs
pub fn check_cost_gates(
    circuits: &[Circuit],
    limits: &CostLimits,
    cost_model: &CostModel,
) -> Vec<CostViolation> {
    circuits
        .iter()
        .flat_map(|circuit| {
            let actual = cost_model.measure_circuit(circuit);
            limits.check(circuit.name(), &actual)
        })
        .collect()
}
```

The cost gate CLI command is `trident audit --cost-gates trident-ci.yml`, integrated into standard CI workflows via a GitHub Action or similar. `trident bench` (see `reference/cli.md`) is the underlying scoreboard that CI queries.
