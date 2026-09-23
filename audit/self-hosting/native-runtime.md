# Native runtime contract review — 2026-09-23

[SH0.4 contract](../../reference/self-hosting-runtime.md) settles the native
execution strategy. The review inspected Trident's lowerer, nox's reducer and
patterns, Joy's execution/file/publication path, and Zheng's production relation.
This is engineering inspection, not maintainer approval or an implemented SH1.
[Validation receipt](sh0-runtime-validation.json) pins inspected owner revisions
and exact worktree source hashes, Cargo feature profile, commands and log hashes.

## Observed baseline

`cargo run --release --locked --example selfhost_runtime -- --output audit/self-hosting/runtime-baseline.json --check`

The hand-authored loop has the same formula particle for0/1/16/4097 iterations.
It stores its code in the subject and re-enters through compose. Results0/1/16
cost5/20/245; VecTrace and NoTrace agree on result, budget and node count.
At4097 the legacy evaluator returns Malformed after7480 rows and2016 allocated
nodes. Failed cost is unavailable, not7480. The input budget was1000000 and
arena allowance49152: neither was exhausted. Raising those does not remove
recursive depth1000. [Full observations](runtime-baseline.json).

This fixture does not pass through the source lowerer and does not demonstrate
self-compilation. It is the regression oracle for the next evaluator delivery.

## Owner decisions

| Owner | Inspected contract/code | Decision and implementation follow-up |
|---|---|---|
| Trident | reference/language.md; src/ir/tree/lower/nox.rs and nox/loops.rs; ir/tir/builder/{call,stmt,expr}.rs | One body per specialization/loop; native arguments once left-to-right; explicit fallthrough/return propagation. Preserve native dynamic candidate guards deliberately; foreign TIR sequencing differences are recorded. |
| nox | specs/{reduction,artifact,trace}.md; rs/reduce.rs, data/reduction.rs, patterns/, parallel.rs | Explicit bounded heap frames, sequential pure L1 independent of parallel feature; preserve budget partition and postorder trace. Node allowance is lifetime/monotone, distinct from reserved host memory. |
| Joy | specs/cli.md; rs/warrior.rs, file_input.rs; cli/build_cmd.rs | Add explicit complete-artifact run path, NoTrace, immutable IDs and accurate success cost. Atomic output only after full execution/export; compiler-profile jobs await production admission. |
| Zheng | specs/execution.md; rs/src/execution/{relation,relation_eval,statement}.rs | Existing production relation does not prove dynamic compiler execution. Reject unsupported proof requests; SH7/SH8 remain separate. |

The review found and resolved two design traps: reverse argument evaluation in
the old native inline fold, and bounded dynamic guards that continue after a
false result. The new contract does not claim the other backend already shares
these semantics. Existing failure traces also cannot serve as a gas counter.

## First SH1 packages and acceptance

1. nox lifetime node allowance: implemented and locally validated in
   [PR18](https://github.com/cyberia-to/nox/pull/18).182 default/183 std tests;
   see nox/audit/arena-limits-2026-09-23.{md,json} for exact evidence.
2. nox sequential heap evaluator: implemented in
   [PR19](https://github.com/cyberia-to/nox/pull/19);192 default/191 parallel-feature
   tests pass. Acceptance covers all pure tags, real4097-iteration compact
   loop, frame boundary, identical short-run traces/budgets/allocation order;
   both default and parallel-feature builds. Continue preserving the failed
   legacy fixture above. Implementation command: `cargo test --workspace --release --locked`;
   focused cases and feature commands are recorded in nox/audit/sequential-frames-2026-09-23.json.
3. Joy structured raw-artifact execution: implemented in
   [PR5](https://github.com/cyberia-to/joy/pull/5);85 workspace tests passed.
   Acceptance covers topology/shared-DAG round trips,
   program/input/output binding, no retained trace, resource boundaries and
   atomic no-overwrite publication. Then production JOB1/RES1 admission.
4. Trident native Noun source type/intrinsics and balanced data libraries;
   reusable calls/loops; source-level full foundation gates from SH1.

No unresolved source/job/data/runtime design choice blocks those packages.
SH0 contract checks and review passed; the gate is closed. SH1 remains open until
all source, runtime and Joy acceptance conditions hold; later full-workload
SH4 measurements still decide practical compiler-scale capacity.
