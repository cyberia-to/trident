# Retention follow-up validation

These commands ran in `docs/0.4-sh7-durable-retention`, based on Trident
`bbcd7e455af1a92c6f2bb971dd3470325fad922f`. The documentation and copied audit
files were pending; production sources, manifests and lockfile were unchanged.
[Build inputs](build-inputs.json) retain the eight clean sibling revisions and
actual Rust 1.89.0 executable identities on `aarch64-apple-darwin`.

The [first command receipt](debug-cargo/receipt.json) records successful
`cargo check --locked --offline` with no warnings. Its complete debug workspace
test attempt was explicitly stopped during a slow exhaustive case; Cargo
returned 101. The [first release attempt](release-two-threads/receipt.json)
also returned 101 after an explicit scheduling restart. Both original logs
and stop records are retained, with [debug](debug-cargo-provenance.json) and
[release](release-two-threads-provenance.json) copy identities. Neither partial
attempt is reported as a complete pass.
The original command streams contain trailing spaces and a terminal blank line. Their bytes remain
unchanged; [the initial whitespace-check result](raw-log-whitespace.json) is
retained, and four exact raw-log paths have whitespace-only Git exemptions.

The complete [eight-thread release run](release-eight-threads/receipt.json)
passed this command:

```sh
cargo test --release --workspace --locked --offline -- --test-threads 8
```

Only this completed command contributes the [counts](release-counts.json):
1231 passed, 0 failed and 5 pre-existing ignored cases across 49 result records.
No test filter was used and no warning was emitted. The elapsed time was
1166.202889 seconds; sampled owned-process-group peak RSS was 4216520704 bytes.
The [runner](run_release_gate.py) enforces an 1800-second wall bound and a
4294967296-byte sampled process-group RSS bound. These are local Rust-test
bounds; compilation-proof bounds stayed fixed. [Original output copies](release-eight-threads-provenance.json)
include every resource sample and both complete command streams.

The suite includes all 17 `formal_audit` CLI/API tests. The selected PATH
resolved [Z3 4.15.4](z3-after-gate.json) immediately after the run; this records
the solver executable, without claiming individual child-process tracing.
The existing full-library formal coverage limitations remain documented in
[the formal audit](../../../../formal-path-validation.md).

All attempts reused the accepted `census/joy/target` build cache, with explicit
`RUSTC`/`RUSTDOC`, cleared compiler wrappers, two Cargo build jobs and
`-D warnings`. There is no fresh compilation-cache timing claim.
[Frozen baseline impact evidence](frozen-baseline-impact.json) identifies the
separate completed distribution gate and verifies unchanged Trident production
paths against its pinned Trident revision. That older binary's proof results
remain scoped to its recorded source closure; this delivery did not rerun
the 198-proof gate or claim those results for a newly installed binary.

[Retention replay](retention-command.json), [documentation checks](documentation-checks.json)
and [the supplied independent source review](source-review.json) retain their
scopes. The full original transport replay is separately preserved in
[root-review](../root-review/receipt.json). This delivery adds no SH8 acceptance.
