---
status: draft
author: mastercyb
area: tooling
planned: 32K
---

# Proof-Cost IDE Integration and CI/CD Gates

## Motivation

Proof cost is invisible in conventional development environments. A developer writes a function, it looks correct, it passes tests, they ship it. Three weeks later, performance profiling reveals that the function dominates proving time because it calls a hash function in a loop. The fix is obvious in retrospect — batch the hashes — but nothing in the development environment pointed to the problem.

Proof cost must be visible during development, not after deployment. IDE integration that colors every line by its AET table contribution makes the expensive lines impossible to ignore. CI/CD gates that reject PRs exceeding declared cost limits make proof cost a regression criterion with the same weight as test failures.

## Design

### Per-Line AET Cost Highlighting

The IDE extension queries the compiler's cost model for each line of Trident code and colors it accordingly:

```
fn example(x: Field) -> Field {
  let a = x * x;              // [green]  1 Processor row
  let b = hash(a);            // [red]    ~200 Hash rows (bottleneck)
  let c = a + b;              // [green]  1 Processor row
  let d = invert(c);          // [yellow] ~95 Processor rows
  d
}
// Tooltip on hash: "Hash: 198 rows (67% of total trace)
//                  Suggestion: batch with other hash calls"
```

Color scheme:
- Green: cost below 10% of function's total trace
- Yellow: cost 10–30% of function's total trace
- Red: cost above 30% of function's total trace — the bottleneck

The highlighting updates incrementally as the developer edits code, using the fast TIR cost model (not full proving). The cost estimate is approximate but directionally correct — sufficient for identifying hotspots.

### LSP Integration

The cost highlighting is exposed via the Language Server Protocol. The IDE queries the Trident LSP (`trident lsp`) for:

- Hover information: "This line costs X Processor rows, Y Hash rows"
- Inline hints: `// cost: ~300 rows` annotations
- Diagnostic messages for lines above a configurable cost threshold
- Quick fixes: "Replace `invert(x)` with `batch_invert([x, y, z])`" when multiple inversions are detected nearby

The LSP computes costs using the same cost model as the compiler, so hover costs match compile-time analysis exactly.

### CI/CD Cost Gates

A project-level configuration file defines proof cost limits:

```yaml
# trident-ci.yml
proof_cost_limits:
  transfer_circuit:
    processor: 2048
    hash: 512
    ram: 1024
    total: 4096
  
  verify_signature:
    processor: 1024
    hash: 1024
    total: 2048

# CI behavior on violation:
on_violation: reject  # or: warn | annotate
report_format: github_pr_comment  # or: json | text
```

The `trident-ci` command runs as part of the CI pipeline. It compiles each named circuit, measures its AET table contributions, and compares against the declared limits. If any limit is exceeded, the CI fails with a detailed report:

```
CI CHECK: proof_cost_limits
  transfer_circuit:
    processor: 2187 / 2048  [FAIL: +139 rows, +6.8%]
    hash:      423 / 512    [PASS]
    ram:       891 / 1024   [PASS]
    total:     3501 / 4096  [PASS]
  
  FAILED. PR rejected.
  Suggestion: The processor overflow is in function `compute_fee` (line 47).
              Consider using batch_invert for the 3 invert() calls.
```

### Regression Detection

CI gates catch regressions, not just absolute violations. If a PR increases a circuit's cost by more than a configurable percentage (say 5%), CI flags it even if the absolute cost is within limits:

```yaml
proof_cost_limits:
  transfer_circuit:
    processor: 2048
    max_regression_pct: 5  # reject if processor cost increases by >5%
```

This prevents gradual cost inflation: each PR adds a few rows, stays within limits, but over 50 PRs the total increases 50%. Regression detection catches this incrementally.

### IDE Integration Architecture

The IDE integration is implemented as a Trident LSP extension:

```
Editor (Zed/VSCode/Helix) ← LSP protocol → trident lsp ← cost model → TIR cost analysis
```

The LSP server:
1. Receives file change notifications from the editor
2. Incrementally recompiles the changed file to TIR (fast — skips TASM generation)
3. Runs the TIR cost model to estimate AET contributions per line
4. Returns semantic token colors and hover text to the editor

The incremental TIR compilation ensures that cost highlighting updates quickly (target: <100ms for single-line edits in typical files).

## Key Tradeoffs

**Cost model accuracy**: The TIR cost model is an approximation. It does not run proving — it estimates from the instruction mix. Accuracy is typically within 10–20% of actual proving cost. This is sufficient for IDE guidance but may misclassify borderline cases in CI gates. The CI gate should use a conservative estimate (upper bound) to avoid false passes.

**Incremental update cost**: Recomputing cost highlighting on every keystroke is too expensive. The LSP debounces — cost highlighting updates after 200ms of inactivity, not on every character. This is imperceptible to the developer.

**Context dependence**: A function's cost depends on its call context (which tables are already filled by the caller). The LSP shows per-function cost in isolation, which may differ from the cost in context. The developer should use the proof explorer for context-aware cost analysis.

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

The cost gate CLI command is `trident audit --cost-gates trident-ci.yml`, integrated into standard CI workflows via a GitHub Action or similar.
