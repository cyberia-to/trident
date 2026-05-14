---
status: draft
author: mastercyb
area: tooling
planned: 32K
---

# REPL with Inline Proof Cost Feedback

## Motivation

The proof cost of Trident code is not intuitive. A developer new to proof systems does not know that `hash(x)` costs 200 rows and `x * x` costs 1. They do not know that `invert(x)` costs 95 rows but `batch_invert([x, y, z])` costs 100 rows for all three combined. They do not know that crossing a power-of-2 cliff doubles the proof cost.

The REPL is the fastest path to building this intuition. Every expression shows its proof cost immediately. The developer types an expression, sees the cost, types a variant, compares. After 20 minutes in the REPL, they have internalized the relative costs of all common operations. After an hour, they think in proof cost naturally.

No documentation can substitute for this immediate feedback loop.

## Design

### Basic Interaction

```
trident> let x = 42;
x = 42
  cost: 1 Processor row  |  tables: Processor

trident> let y = hash(x);
y = 0x3f2a8c1d...
  cost: 198 rows  |  tables: Processor(5) + Hash(193)

trident> let z = invert(y);
z = 0x7b1c4e8f...
  cost: 97 rows  |  tables: Processor(97)

trident> let w = z * z;
w = 0x2a9f3d...
  cost: 1 row  |  tables: Processor(1)
```

Every expression shows:
- The computed value (for immediate feedback that the computation is correct)
- The total cost in AET rows
- The per-table breakdown

### Proof Summary Command

After building up several expressions, `:proof_summary` shows the accumulated state:

```
trident> :proof_summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
SESSION PROOF SUMMARY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Expressions:  4
Total rows:   297

Table breakdown:
  Processor  103 rows  (34.7%)  ████████████░░░░░░░░░░░░░░░░░░░░
  Hash       193 rows  (65.0%)  ████████████████████████████████
  RAM          1 row   ( 0.3%)  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░

Pads to:     512 rows (next power of 2)
Bottleneck:  Hash (65% of trace)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Suggestion: Your session is dominated by Hash. Batch hash calls:
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
  A: invert(x)    →  97 rows  |  Processor(97)
  B: pow(x, p-2)  →  95 rows  |  Processor(95)
  Winner: B (-2 rows, -2.1%)
  Note: For batch inversions, Montgomery's trick beats both significantly.
```

The `:compare` mode is especially useful for evaluating optimization candidates without writing a full program.

### Pattern Library

The REPL includes a pattern library that suggests common optimizations when it detects expensive patterns:

```
trident> invert(a); invert(b); invert(c);
PATTERN DETECTED: 3 separate inversions
Current cost: 97 + 97 + 97 = 291 Processor rows
Better:       batch_invert([a, b, c]) = ~100 Processor rows
              → 191 rows saved (65.6% reduction)

Type `:accept` to rewrite or `:keep` to proceed as-is.
```

The pattern library is extensible — as the algebraic identity explorer discovers new optimization rules, they appear in the REPL's suggestion system.

### Cliff Warning System

When accumulated cost approaches a power-of-2 boundary, the REPL warns proactively:

```
trident> let expensive = [hash(x) for x in my_array];
y = [...]
  cost: 594 rows  |  tables: Hash(580) + Processor(14)

WARNING: Hash table at 580/512 rows — already over 512 cliff!
         Proof now pads Hash to 1024 rows.
         To stay under 512: hash at most 2.5 items per session.
         Recommendation: batch_hash(my_array) uses ~214 rows total.
```

The cliff system knows all power-of-2 boundaries and predicts when they will be crossed.

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
// Proof cost: 297 rows, dominanted by Hash (193 rows)
// File: saved to session_export.tri
```

This enables using the REPL as a rapid prototyping environment for cryptographic code snippets, then exporting them to proper source files.

## Key Tradeoffs

**Actual vs. estimated costs**: The REPL can show either estimated costs (from the TIR cost model, fast) or actual costs (from running the prover, slow). For interactive use, estimated costs are shown by default. Actual costs are available via `:prove` which runs the full prover for the session.

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
