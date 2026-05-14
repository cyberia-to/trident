---
status: draft
author: mastercyb
area: tooling
planned: 32K
---

# Interactive Proof Explorer

## Motivation

A STARK proof is opaque to the developer. They write code, they prove it, they get a proof — and they have no visibility into what happened. Which AET tables are full? Which source lines generated the most trace rows? Where are the power-of-2 cliffs that will make the next small addition double the proof size? Without answers to these questions, proof optimization is guesswork.

The interactive proof explorer makes the STARK proof transparent. It is a developer tool — not a verification tool — that lets developers navigate the execution trace, identify bottlenecks, understand cost distribution, and predict the impact of code changes before compiling them.

## Design

### Table Fill Visualization

The main view shows the AET table fill levels as a bar chart:

```
AET TABLE HEIGHTS (current program)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Processor  ████████████████████░░░░░░░░░░░░░  1847/2048  (90.2%)  [cliff: 201 rows away]
Hash       ████████████████████████████████░  498/512    (97.3%)  [cliff: 14 rows away!]
RAM        ████████░░░░░░░░░░░░░░░░░░░░░░░░░  312/1024   (30.5%)
U32        ██░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  87/256     (34.0%)
OpStack    █████░░░░░░░░░░░░░░░░░░░░░░░░░░░░  198/512    (38.7%)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Bottleneck: Hash (97.3% full — near 512 cliff)
Next power of 2: Hash → 1024 (+512 rows, doubles Hash table cost)
Recommendation: reduce Hash rows by 15 to stay under 512 cliff
```

The "cliff" indicator is critical: when a table is near a power-of-2 boundary, adding even a few rows doubles the proof cost for that table. The explorer highlights these cliffs prominently.

### Click-to-Source Tracing

The developer clicks on any section of a filled bar to see which source lines generated those trace rows.

For the Hash table at 498/512: clicking on the bar opens a source annotation view:

```
Hash table row contributions by source line:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
my_program.tri:42  hash(leaf)              198 rows  (39.8%)  [inside loop: × 3]
my_program.tri:67  commit(secret)           87 rows  (17.5%)
my_program.tri:23  verify(merkle_root, ...) 213 rows  (42.8%)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total Hash rows: 498
```

Double-clicking a source line jumps to it in the editor. The connection between proof cost and source code is direct.

### Hot Zone Highlighting

The explorer overlays "hot zone" markers on the AET timeline: regions where table height approaches a power-of-2 cliff within a configurable margin (default: 5%). These are displayed in orange and red in the bar chart and in the source code view.

The hot zone detection is automatic. The developer does not need to know the power-of-2 boundaries — the explorer knows them and highlights the dangerous proximity.

### Impact Simulation

Before compiling a proposed change, the developer can simulate its cost impact in the explorer:

```
SIMULATE: Replace hash(leaf) with batch_hash([leaf1, leaf2, leaf3])
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
BEFORE:  Hash rows: 498/512  (97.3%)
AFTER:   Hash rows: ~372/512 (72.7%)  [estimated -126 rows]
CLIFF:   No longer near 512 cliff. Next cliff: 512 rows away.
PROOF:   Current: ~1024-row Hash table → After: still 512-row Hash table
         PROOF SIZE UNCHANGED (both below same cliff)
         But: saved 126 rows of margin for future growth.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

The simulation uses the TIR cost model — fast enough for interactive use. It shows both the row count change and the proof cost impact (which depends on cliff crossings, not just row count). A change that reduces rows by 10% but keeps the table on the same side of a cliff has no proof cost impact; a change that reduces rows by 1% but crosses a cliff halves the proof cost.

### Processor Table Navigation

For fine-grained analysis, the explorer shows the Processor table as a timeline: each row is one executed TASM instruction, color-coded by source function:

```
PROCESSOR TABLE (1847 rows, showing 100-200)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Row 100-115:  [compute_fee]  ████████████████  16 rows
Row 116-210:  [hash(leaf)]   ██████████████... expanding to Hash table
Row 211-240:  [verify_sig]   ██████████████████████████████  30 rows
...
```

The timeline view enables identifying which functions consume Processor table rows and finding optimization opportunities at the instruction level.

## Key Tradeoffs

**Real-time vs. accurate costs**: Interactive simulation uses the TIR cost model (fast, approximate). Final accurate costs require actual proving (slow). The explorer clearly labels which costs are estimates vs. measured.

**Display complexity**: For programs with hundreds of functions and thousands of trace rows, the source annotation view becomes cluttered. The explorer must provide good filtering and aggregation controls to remain usable for large programs.

**Integration with LSP**: The proof explorer is a standalone tool, but its data should integrate with the IDE via LSP. The developer should be able to click a cost hint in the editor and open the proof explorer focused on that line's cost contribution.

## Implementation Sketch

The proof explorer is a TUI (terminal user interface) tool:

```rust
// tools/proof_explorer/main.rs
fn main() {
    let program = load_program(args.tasm_file);
    let proof = load_or_generate_proof(&program, args.input);
    let aet = extract_aet(&proof);

    let ui = ProofExplorerUI {
        aet,
        source_map: SourceMap::from_program(&program),
        cost_model: CostModel::default(),
    };

    ui.run_interactive();
}

// tools/proof_explorer/ui.rs
impl ProofExplorerUI {
    fn render_table_bars(&self) -> Vec<Bar> {
        Table::all().map(|t| {
            let height = self.aet.height(t);
            let cliff = next_power_of_2(height);
            Bar { table: t, height, cliff, proximity: cliff - height }
        }).collect()
    }

    fn render_source_contribution(&self, table: Table) -> Vec<SourceLine> {
        self.aet.rows(table)
            .group_by(|row| self.source_map.line_of(row.origin))
            .map(|(line, rows)| SourceLine { line, row_count: rows.len() })
            .sorted_by(|a, b| b.row_count.cmp(&a.row_count))
            .collect()
    }
}
```

The tool is invoked as `trident explore my_program.warrior` or `trident explore my_program.tasm --input my_input.json`.
