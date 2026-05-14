---
status: draft
author: mastercyb
area: tooling
planned: 32K
---

# Interactive Proof Explorer

**Related proposals:** [[proof-cost-ide]], [[trident-repl]], [[trace-predictor]]

## Motivation

A zheng proof is opaque to the developer. They write code, they prove it, they get a proof — and they have no visibility into what happened. How long is the nox trace? How many sumcheck rounds did zheng run? How large are the Brakedown commitment layers? Where are the power-of-2 cliffs in trace length that will make the next small addition double the proving time? Without answers to these questions, proof optimization is guesswork.

The interactive proof explorer makes the zheng proof transparent. It is a developer tool — not a verification tool — that lets developers inspect the nox execution trace, understand the zheng proof structure, identify bottlenecks, and predict the impact of code changes before compiling them.

## Design

### Proof Structure Overview

The main view shows the inspectable dimensions of a zheng proof:

```
ZHENG PROOF STRUCTURE (current program)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
nox trace length    ████████████████████░░░░░░░░░░░  1847/2048  (90.2%)  [cliff: 201 steps away]
sumcheck rounds     ████████████████████████████████  21 rounds  (log₂ of trace vars)
Brakedown cols      ████████░░░░░░░░░░░░░░░░░░░░░░░  312 cols   (commitment width)
commitment size     ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  87 KB      (Brakedown layers)
hemera calls        █████░░░░░░░░░░░░░░░░░░░░░░░░░░  198        (Poseidon2 hashes)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Bottleneck: trace length (90.2% of next power-of-2 — 201 steps to cliff)
Next cliff: 2048 → 4096 (+2048 nox steps, doubles sumcheck rounds)
Recommendation: reduce trace by 201 steps to stay below the 2048 cliff
```

The "cliff" indicator is the trace length power-of-2 boundary: zheng pads the nox trace to the next power of 2 before running sumcheck, so crossing a cliff doubles the number of sumcheck rounds and proportionally increases proving time. The explorer highlights these cliffs prominently.

### Click-to-Source Tracing

The developer clicks on any dimension in the overview bar to see which source lines generated those nox steps.

Clicking on "trace length 1847/2048" opens a source annotation view:

```
nox step contributions by source line:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
my_program.tri:42  hash(leaf)              198 steps  (10.7%)  [hemera Poseidon2, inside loop: × 3]
my_program.tri:67  commit(secret)           87 steps  ( 4.7%)  [hemera Poseidon2]
my_program.tri:23  verify(merkle_root, ...) 213 steps  (11.5%)  [nox pattern: merkle-verify]
...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total nox steps: 1847
```

Double-clicking a source line jumps to it in the editor. The connection between proof cost and source code is direct.

### Hot Zone Highlighting

The explorer overlays "hot zone" markers when the trace length approaches a power-of-2 cliff within a configurable margin (default: 5% of the cliff value). These are displayed in orange and red in the overview and in the source annotation view.

The hot zone detection is automatic. The developer does not need to know the power-of-2 boundaries — the explorer knows them and highlights the dangerous proximity. nox's 16 patterns + 1 hint + 5 jets each carry known step costs; the explorer aggregates these from the source map.

### Impact Simulation

Before compiling a proposed change, the developer can simulate its cost impact in the explorer:

```
SIMULATE: Replace hash(leaf) with batch_hash([leaf1, leaf2, leaf3])
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
BEFORE:  nox trace: 1847/2048 steps (90.2%)
AFTER:   nox trace: ~1721/2048 steps (84.0%)  [estimated -126 steps]
CLIFF:   No longer in hot zone. Margin to 2048 cliff: 327 steps.
PROOF:   Both before and after are below the 2048 cliff.
         sumcheck rounds: unchanged (same trace-length power-of-2 bucket)
         Brakedown commitment: slightly smaller (fewer trace columns used)
         But: saved 126 steps of margin for future growth.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

The simulation uses the TIR cost model — fast enough for interactive use. It shows both the step count change and the proof cost impact (which depends on cliff crossings, not just step count). A change that reduces steps by 10% but stays in the same power-of-2 bucket leaves the sumcheck round count unchanged; a change that reduces steps by 1% but crosses a cliff halves the proving time.

### Trace Timeline Navigation

For fine-grained analysis, the explorer shows the nox execution trace as a timeline: each step is one nox reduction, colour-coded by source function. Jet invocations are highlighted separately (jets compress multiple nox steps into a single verified shortcut):

```
NOX TRACE (1847 steps, showing 100-200)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Steps 100-115:  [compute_fee]    ████████████████  16 steps
Steps 116-210:  [hash(leaf)]     ██████████████... 95 steps (hemera Poseidon2)
                                 [jet: poseidon2]  highlighted — jet covers steps 180-210
Steps 211-240:  [verify_sig]     ██████████████████████████████  30 steps
...
```

The timeline view enables identifying which functions consume nox trace steps and where jet invocations provide compression. See [[trident-repl]] for interactive step-by-step exploration.

## Vision

The proof explorer is the developer's window into the [[zheng]] proof. In a live cyber ecosystem, every deployed program's zheng proof is a particle in the [[cybergraph]]. The proof explorer is a [[soft3]] query tool — it fetches the proof particle, decodes the Brakedown commitment, and renders the [[nox]] trace timeline. A developer debugging a focus-cost regression traces execution step-by-step, watches the [[hemera]] jet calls light up in the timeline, identifies the hot zone, and clicks to the source line.

The [[zheng]] proof is not an opaque blob — it is a structured artifact that tells the full story of the computation. Brakedown commitment columns map to witness dimensions; the sumcheck transcript records every round of the IOP. The proof explorer decodes both. When a trace crosses a power-of-2 cliff and proving time doubles, the explorer shows exactly which source lines pushed the trace over. The fix is obvious the moment you see it.

This matters most at scale: as the [[cybergraph]] fills with proofs from many programs, the explorer becomes a comparative tool. A developer queries two particles — the proof before and after an optimization — and diffs their nox traces side by side. The graph stores the history permanently. Regressions are findable by any auditor, not just the original developer.

## Stack Integration

The proof explorer reads [[zheng]] proofs via [[soft3]]'s `query(proof_cid, dimension)`. The proof structure — Brakedown commitment columns and sumcheck transcript — maps to dimensions of the [[bbg]] state. The [[hemera]] Merkle tree inside the Brakedown commitment uses the same hash primitive as the [[cybergraph]]'s content addressing, so the proof's internal structure and the graph's addressing scheme are aligned. Fetching a proof particle and decoding its Brakedown layers are the same operation.

## Key Tradeoffs

**Real-time vs. accurate costs**: Interactive simulation uses the TIR cost model (fast, approximate). Final accurate costs require actual proving (slow). The explorer clearly labels which costs are estimates vs. measured.

**Display complexity**: For programs with hundreds of functions and thousands of trace rows, the source annotation view becomes cluttered. The explorer must provide good filtering and aggregation controls to remain usable for large programs.

**Integration with LSP**: The proof explorer is a standalone tool, but its data should integrate with the IDE via LSP. The developer should be able to click a cost hint in the editor and open the proof explorer focused on that line's cost contribution.

## Implementation Sketch

The proof explorer is a TUI (terminal user interface) tool that reads a `ProgramBundle` and either uses an embedded zheng proof or generates one on demand:

```rust
// tools/proof_explorer/main.rs
fn main() {
    let bundle = ProgramBundle::load(args.bundle_path)?;
    let proof  = bundle.zheng_proof
        .unwrap_or_else(|| zheng_prove(&bundle.nox, &args.input));
    let trace  = extract_nox_trace(&bundle.nox, &args.input);

    let ui = ProofExplorerUI {
        trace,
        proof_meta: ZhengProofMeta::from(&proof),  // sumcheck rounds, commitment sizes
        source_map: SourceMap::from_bundle(&bundle),
        cost_model: CostModel::default(),
    };

    ui.run_interactive();
}

// tools/proof_explorer/ui.rs
impl ProofExplorerUI {
    fn render_overview(&self) -> Overview {
        let trace_len  = self.trace.len();
        let cliff      = next_power_of_2(trace_len);
        let rounds     = self.proof_meta.sumcheck_rounds;
        let commit_kb  = self.proof_meta.brakedown_commitment_size_kb;
        Overview { trace_len, cliff, rounds, commit_kb }
    }

    fn render_source_contribution(&self) -> Vec<SourceLine> {
        self.trace.steps()
            .group_by(|step| self.source_map.line_of(step.origin))
            .map(|(line, steps)| SourceLine { line, step_count: steps.len() })
            .sorted_by(|a, b| b.step_count.cmp(&a.step_count))
            .collect()
    }
}
```

The tool is invoked as `trident explore my_program.warrior` or `trident explore my_program --input my_input.json`.
