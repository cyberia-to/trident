---
status: draft
author: mastercyb
area: tooling
planned: 32K
---

# REPL with Inline Proof Cost Feedback

**Related proposals:** [[proof-cost-ide]], [[proof-explorer]]

## Motivation

The proof cost of Trident code is not intuitive. A developer new to proof systems does not know that `hash(x)` costs ~200 nox steps and `x * x` costs 1. They do not know that `invert(x)` costs ~95 nox steps but `batch_invert([x, y, z])` costs ~100 steps for all three combined. They do not know that crossing a power-of-2 trace-length cliff doubles the zheng proving time.

The REPL is the fastest path to building this intuition. Every expression shows its nox step count and jet invocations immediately. The developer types an expression, sees the cost, types a variant, compares. After 20 minutes in the REPL, they have internalized the relative costs of all common operations. After an hour, they think in proof cost naturally.

No documentation can substitute for this immediate feedback loop.

## Design

### Basic Interaction

```
trident> let x = 42;
x = 42
  cost: 1 nox step  |  jets: none

trident> let y = hash(x);
y = 0x3f2a8c1d...
  cost: 198 nox steps  |  jets: poseidon2 ×1  (hemera Poseidon2 hash)

trident> let z = invert(y);
z = 0x7b1c4e8f...
  cost: 97 nox steps  |  jets: none

trident> let w = z * z;
w = 0x2a9f3d...
  cost: 1 nox step  |  jets: none
```

Every expression shows:
- The computed value (for immediate feedback that the computation is correct)
- The total cost in nox reduction steps
- Jet invocations (jets are verified shortcuts that compress nox steps — showing them separately lets the developer see where the nox VM takes shortcuts vs. reduces fully)

### Proof Summary Command

After building up several expressions, `:proof_summary` shows the accumulated nox trace and zheng proof cost:

```
trident> :proof_summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
SESSION PROOF SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Expressions:    4
nox steps:      297
Jet calls:      poseidon2 ×1

Step breakdown by source:
  let x = 42       1 steps   ( 0.3%)  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░
  let y = hash(x)  198 steps (66.7%)  ████████████████████████████████
  let z = invert   97 steps  (32.7%)  █████████████████████░░░░░░░░░░░
  let w = z*z      1 step    ( 0.3%)  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░

Trace pads to:  512 steps (next power of 2 — zheng sumcheck boundary)
Bottleneck:     hemera hash (66.7% of trace)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Suggestion: Session dominated by hemera Poseidon2. Batch hash calls:
            Instead of hash(a); hash(b); use hash_batch([a, b]).
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

### Cost Comparison Mode

The REPL supports side-by-side cost comparison:

```
trident> :compare
  A> invert(x)
  B> pow(x, p-2)

Comparing A vs B:
  A: invert(x)    →  97 nox steps  |  jets: none
  B: pow(x, p-2)  →  95 nox steps  |  jets: none
  Winner: B (-2 steps, -2.1%)
  Note: For batch inversions, Montgomery's trick beats both significantly.
```

The `:compare` mode is especially useful for evaluating optimization candidates without writing a full program. For a deeper view of the trace differences, use [[proof-explorer]] after exporting the session.

### Pattern Library

The REPL includes a pattern library that suggests common optimizations when it detects expensive patterns:

```
trident> invert(a); invert(b); invert(c);
PATTERN DETECTED: 3 separate inversions
Current cost: 97 + 97 + 97 = 291 nox steps
Better:       batch_invert([a, b, c]) = ~100 nox steps
              → 191 steps saved (65.6% reduction)

Type `:accept` to rewrite or `:keep` to proceed as-is.
```

The pattern library is extensible — as the algebraic identity explorer discovers new optimization rules, they appear in the REPL's suggestion system.

### Cliff Warning System

When accumulated nox trace length approaches a power-of-2 boundary, the REPL warns proactively. The boundary matters because zheng pads the trace to the next power of 2 before running sumcheck — crossing it doubles the sumcheck round count and proving time:

```
trident> let expensive = [hash(x) for x in my_array];
y = [...]
  cost: 594 nox steps  |  jets: poseidon2 ×10

WARNING: Trace at 594 steps — already over 512 cliff!
         zheng pads trace to 1024 steps for sumcheck.
         To stay under 512: reduce hash calls by ~82 steps.
         Recommendation: batch_hash(my_array) uses ~214 steps total.
```

The cliff system tracks the accumulated session trace length and predicts when cliffs will be crossed. Use [[proof-explorer]] to visualise the full trace after a session.

### REPL History and Export

The REPL tracks the session history. When the developer has built up a useful computation, `:export` generates a Trident function from the session:

```
trident> :export
Generated function from session:

fn session_computation(x: Field) -> Field {
    let y = hash(x);
    let z = invert(y);
    z * z
}
// Proof cost: 297 nox steps, dominated by hemera hash (198 steps)
// File: saved to session_export.tri
```

This enables using the REPL as a rapid prototyping environment for cryptographic code snippets, then exporting them to proper source files.

## Vision

The Trident REPL is the entry point to the [[cybergraph]]. Every evaluated expression that produces a result also produces a [[nox]] trace and, optionally, a [[zheng]] proof. The REPL can submit this as a cyberlink: `trident> let y = hemera(x); :submit` — and the result becomes a permanent entry in the knowledge graph. Other agents can query it. In a world where AI agents interact through the [[cybergraph]], the REPL is how humans stay in the loop: type a computation, see the cost in [[bbg]] focus units, submit the proof, observe the cyberlink.

The REPL is also how developers build intuition for [[nox]] cost that no documentation can substitute. After 20 minutes of seeing `hash(x)` cost 198 steps and `x * x` cost 1, those numbers become visceral. After an hour, the developer thinks in proof cost naturally. Every optimization pattern the algebraic identity explorer discovers eventually surfaces as a REPL suggestion — the pattern library is the accumulated knowledge of the ecosystem expressed as interactive feedback.

The `:query existing_cid` command completes the loop: fetch a previously computed result from the [[cybergraph]] without re-running it. If `hemera(x)` was computed before and its result is a particle, `:query` retrieves it in milliseconds. Memoization through the knowledge graph replaces redundant computation — the REPL is the developer-facing interface to the system's global memoization layer.

## Stack Integration

`:prove` in the REPL calls [[warrior-cyber]] directly, running the full [[zheng]] prover on the session trace and reporting real trace length, sumcheck rounds, and commitment sizes. `:submit` calls [[soft3]]'s `cyberlink()` to record the computation as a permanent particle in the [[cybergraph]] — the [[hemera]] CID of the result identifies it. `:query existing_cid` fetches a previously computed particle via [[soft3]]'s `query()` without re-running the computation. The REPL connects the interactive development loop to the planetary knowledge graph.

## Key Tradeoffs

**Actual vs. estimated costs**: The REPL can show either estimated costs (from the TIR cost model, fast) or actual costs (from running zheng, slow). For interactive use, estimated nox step counts are shown by default. Actual costs are available via `:prove` which runs the full zheng prover for the session and reports real trace length, sumcheck rounds, and commitment sizes.

**State accumulation**: The REPL accumulates state across expressions. The proof cost shown for each expression is its marginal cost — the additional rows it adds to the session's trace. This matches how the developer thinks ("how expensive is this one thing?") but may differ from the expression's standalone cost (if it reuses computations from earlier in the session).

**REPL vs. production**: REPL code is exploratory. Costs measured in the REPL may differ from costs in production code due to context, inlining, and optimization passes that only run on full programs. The REPL shows costs with optimizations disabled (to show the true cost of each expression in isolation). The `:optimized` flag enables optimizations for closer-to-production cost estimates.

## Implementation Sketch

```rust
// cli/repl.rs
pub struct TridentRepl {
    session: ReplSession,
    cost_model: CostModel,
    pattern_library: PatternLibrary,
}

struct ReplSession {
    bindings: HashMap<String, (FieldElement, CostAccumulator)>,
    total_cost: CostAccumulator,
}

impl TridentRepl {
    fn eval(&mut self, input: &str) -> ReplResult {
        let expr = parse(input)?;
        let tir = lower_expr(&expr, &self.session.bindings)?;
        let cost = self.cost_model.estimate(&tir);
        let value = execute(&tir)?;

        self.session.record(expr.binding_name(), value, cost);
        self.pattern_library.check(&tir)  // check for optimization suggestions

        ReplResult {
            value,
            cost,
            suggestions: self.pattern_library.suggestions(),
            cliff_warning: self.session.cliff_warning(),
        }
    }
}
```

The REPL is invoked as `trident repl` and connects to the same LSP infrastructure as the IDE integration, sharing the cost model and pattern library.
