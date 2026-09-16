# Terminal return values — 2026-09-12

The preserved FINAL3 nox compiler accepted
`fn main(x: Field) -> Field { if x == 0 { 3 } else { 4 } }` but returned
`0` for both inputs 0 and 1. Each execution used15 reductions. The equivalent
Triton helper returned3 and4 through explicit public output. Reproducers remain
in `/tmp/terminal-branch-review/`. FINAL3 is unchanged and predates this fix.

The canonical language contract forwards values of terminal conditional and
match branches into the function result. Shared AST normalization makes those
returns explicit while preserving source spans and lexical scopes. The nox
compiler now applies it once to each entry and inlined function body. It does
not change intermediate branch/loop tails into function returns. Nox match
lowering remains separately unsupported; normalization does not implement it.
Typechecking and scalar formal analysis use the same function return rule.

Three new real nox VM tests pass in both source profiles: selected scalar values,
nested helper branches with locals/explicit returns/aggregate results, and
discarded intermediate branch and loop tails. The complete source surface
passes48 tests, zero failed/ignored/warnings in5.45s:
`/tmp/nox-terminal-full-surface.log`. Test sources are
`tests/nox_surface/terminal.rs`.

Joy's two new tests pass in release in10.90s with4 Rayon threads and
`TVM_LDE_TRACE=no_cache`: `/tmp/joy-terminal-return-proofs.log`.
Four JOYEXEC2 public certificates cover both selected outputs7 and9 under both
source profiles. One genuine JOYZK003 private STARK binds public input0 and
secret35 to output42 through a terminal conditional. Fresh decoding and
verification pass; changing public input, replacing the output with the old
incorrect zero, or changing reduction cost rejects in every case. Public
certificates disclose the execution witness; the private case uses the separate
randomized Triton7 STARK protocol.

These are source/compiler/proof regressions. They do not close general dynamic
nox execution, whole-library formal verification or final artifact gates.
