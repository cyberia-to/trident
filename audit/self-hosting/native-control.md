# SH1 reusable raw-native control — 2026-09-24

Source implementation: Trident `4317c767607ce83210348d285c1390df0bed5d94`.
The [validation manifest](sh1-control-validation.json) pins every owner, command
and log. [Actual CLI receipts](native-control-observations.json) include source
files, artifact particles and complete Joy execution reports.

Raw ART1 compilation now emits one formula for each reachable concrete function
and generated loop. Bodies use a balanced immutable code table and fixed frame
slots. Calls materialize arguments once in source order; blocks propagate
Continue/Return values. Loop bounds change constants rather than copying bodies.
The flat bundle/proof backend retains its previous lowering.

Runtime array reads and nested projected writes use shared cursor/zipper helpers.
Checked indices precede the RHS; all are evaluated once. Struct, array, tuple and
Noun topology survives mutation. Digest reads retain the balanced four-limb shape;
indexed Digest writes remain rejected by source type checking.

Generic preparation retains defining module/function and concrete sizes without
changing public AST or foreign specialization names. Unreachable generic
instances cannot renumber the native table. Source recursion has a separate
call-graph rejection; generated loop cycles are ordinary deterministic compose.

## Review and regression findings

- Tuple assignment previously omitted component-type and unknown-target checks.
  It could place a Noun into a Field binding. Every target is now checked.
- A local name could inherit a same-named global constant's loop classification.
  The frontend now respects shadowing. Accepted outer-index specialization is
  preserved, including `0..i bounded 0`, `i..3` and same-name nested indices.
- The existing imported-generic execution harness constructed its fixed arena on
  the default test stack. The initial debug workspace run stalled in the arena
  constructor's stack probe. Its execution now uses a joined 64 MiB-stack worker;
  the ordinary debug workspace command completes without an environment override.

Independent read-only review checked helper axes, persistent edits, argument
ordering, scope and return propagation. The tuple assignment issue was fixed
before acceptance. Main review covered table determinism, origin metadata,
planning/emission limits, scalar canonicalization and pure-profile rejection.

## Validation

- `cargo test --workspace --locked`: 891 passed, zero failed/ignored. The 21
  added tests cover control flow, admission and 45 native/legacy/integer
  differential cases. Existing 17 Noun tests also exercise the replacement.
- Joy `CARGO_TARGET_DIR=../trident/target cargo test --workspace --release --locked`:
  89 passed, zero failed/ignored, including an actual source loop calling a helper
  5000 times and failed-publication checks for frames, reductions and node limits.
- Trisha `CARGO_TARGET_DIR=../trident/target cargo test --release --locked -p trisha-rs`:
  362 passed, zero failed; four existing expensive proof tests remain ignored.
  They are not claimed as executed by this delivery.
- Trisha `cargo run --release --locked -p trisha -- bench`, with the same target
  directory: 133/133 fixtures passed, 43/43 independent baselines verified.
- Workspace/all-target checks are warning-free. Noun intrinsic formal audit
  remains UNKNOWN/exit2 because declarations have no analyzable scalar bodies.
  These are execution checks, with no native Zheng compiler-proof claim.

## Actual Joy execution

All runs below used NoTrace and an explicit 65536-frame allowance. Each output
was independently interpreted as its expected canonical atom; the dynamic
outputs were also byte-equal to the corresponding constant-loop outputs.

| Program | Output | ART1 bytes | Reductions | Lifetime nodes | Peak frames |
|---|---:|---:|---:|---:|---:|
| constant 0 | 0 | 8117 | 57 | 94 | 12 |
| constant 1 | 1 | 17578 | 207 | 206 | 20 |
| constant 4097 | 4097 | 17813 | 274639 | 57528 | 32788 |
| constant 5000 | 5000 | 17813 | 335140 | 70170 | 40012 |
| dynamic, input 0 | 0 | 22659 | 590170 | 60213 | 40016 |
| dynamic, input 1 | 1 | 22659 | 590192 | 60237 | 40016 |
| dynamic, input 4097 | 4097 | 22659 | 680304 | 80718 | 40016 |
| dynamic, input 5000 | 5000 | 22659 | 700170 | 85233 | 40016 |

The same dynamic artifact considers all 5000 candidates even after a false
guard. A trace-based regression counts four separate guard inversions when only
the first of four candidates enters the body. Calls/conversions/index updates
have separate once-only inversion observations and distinguishable first-error
tests. A diamond call graph stays compact; 100 live locals execute successfully.

Joy's default 16384-frame allowance deliberately rejects the large loop. Its
previous output remains byte-identical. No automatic retry or limit increase
occurs. The receipts separately report a 26214416-byte arena reservation,
11534336-byte frame buffer and 268435456-byte worker stack allowance. None of
these figures is a measurement of RSS or compiler-scale memory feasibility.

## Remaining gates

SH1 still needs native Seq/Bytes libraries and production JOB1/RES1 admission.
SH2 needs a `.tri` compiler that runs on nox and emits a separately executed
program. Full closure, C2/C3 equality, six-platform acceptance and production
native Zheng proofs retain their independent later gates.
