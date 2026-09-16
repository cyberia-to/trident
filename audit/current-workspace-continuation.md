# Current workspace continuation after FINAL5

2026-09-12. Live-source test receipts following the owner-only deep-stack move,
alternate XField-width repair and formal helper support. This is not a frozen
candidate, an installed-platform smoke, or a replacement for the separate
FINAL5 198-proof gate. No ignored test was enabled. Production files were not
edited during this validation; the formal owner corrected one stale test.

## Trident

Command:

```sh
CARGO_BUILD_JOBS=2 RAYON_NUM_THREADS=4 cargo test --manifest-path trident/Cargo.toml --workspace --release --features neural --locked -- --test-threads=1
```

First receipt `/tmp/current-trident-workspace-continuation.log`: exit 101,
723 passed, one failed in the library harness. The existing
`verify::solve::tests::test_inlined_function_verification` expected `Unknown`
for a now-supported helper asserting `x + 0 == x`; the solver correctly returned
`Safe`. The formal owner updated that expectation and added a false helper
`x + 1 == x` called with zero, requiring `StaticViolation`. The owner first
validated the focused regression separately.

Complete repeated command `/tmp/current-trident-workspace-continuation-final.log`:
**exit 0; 894 passed, 0 failed, 0 ignored across 19 test/doc result groups.**
No Rust compilation warning appeared. Three intentionally rendered language
warning messages occur in the diagnostic unit tests (`unused x`, `unused y`,
`redundant range check`); zero compiler warnings does not mean an empty log.

## Trisha

Command:

```sh
CARGO_BUILD_JOBS=2 RAYON_NUM_THREADS=4 cargo test --manifest-path trisha/Cargo.toml --workspace --all-features --release --locked -- --test-threads=1
```

`/tmp/current-trisha-workspace-continuation.log`: all 57 completed ordinary
runtime/unit/integration result groups passed, **450 passed, 0 failed, 5
ignored**. The overall command then exited 101 during Honeycrisp doctest
compilation, before a doctest result: E0463 could not find crate `trident`, with
secondary `Prover`/`Verifier` name-resolution errors.

The exact `--extern` artifact named by that failing invocation,
`target/release/deps/libtrident-97717fb0d6e8c982.rlib`, was absent when inspected.
Another agent had compiled a different Trisha integration/feature selection in
the same target directory during the broad run. Artifact turnover is therefore
a plausible cause, not a demonstrated compiler/source defect or a proven race
reproduction. No cleanup was performed by this reviewer; root confirmed no
cleanup on its side. No dependency, implementation or doctest was disabled.

An isolated Honeycrisp `--all-features --release --locked --doc` retry rebuilt
its dependency selection and passed (zero doctests); metadata diagnostics were
retained in `/tmp/current-honeycrisp-doctest-diagnosis.log`. The exact workspace
all-features documentation gate is repeated after other Trisha builds stop;
its final receipt is recorded below. The earlier 450 ordinary test passes are
preserved rather than overwritten by a zero-test doctest receipt.

The five explicitly ignored gates are the genuine custom-lock transaction
prepare/mock gateway test, its transaction/binding mutation counterpart, the
typed-entry proof, imported-generic proof, and full recursive outer proof.
They remain separate gated proofs with their own required fixtures/resources.

No Rust compilation warnings appeared in the ordinary suite. Four source
warnings were intentionally printed by existing fixture compilation:
`vm.core.convert` twice, `vm.crypto.hash` once, `os.neptune.xfield` once. These
are distinguished from Rust warnings and are not silently omitted.

Final documentation retry:

```sh
CARGO_BUILD_JOBS=2 RAYON_NUM_THREADS=4 cargo test --manifest-path trisha/Cargo.toml --workspace --all-features --release --locked --doc -- --test-threads=1
```

`/tmp/current-trisha-workspace-doc-retry.log`: **exit 0**, all four documentation
harnesses (`trisha_honeycrisp`, `trisha_neptune`, `trisha_rs`, `trisha_wgpu`)
compile successfully, each containing zero doctest examples. No warning was
emitted. This closes the earlier failed documentation compilation separately
from the preserved 450 ordinary test passes; it does not invent four test cases.

Literal per-harness counts, exit codes and SHA256 log identities are in
[`current-workspace-continuation.json`](current-workspace-continuation.json).
Tools were host Homebrew Rust/Cargo/rustdoc 1.95.0 on macOS ARM64. Frozen
candidate native MSRV/platform/proof receipts remain separate evidence. No
source defect beyond the reviewed stale formal test was repaired in this run.

## Final all-target/all-feature checks

Both commands completed with exit 0 and no Rust warnings:

```sh
CARGO_BUILD_JOBS=2 cargo check --manifest-path trident/Cargo.toml --workspace --all-targets --all-features --release --locked
CARGO_BUILD_JOBS=2 cargo check --manifest-path trisha/Cargo.toml --workspace --all-targets --all-features --release --locked
```

Logs `/tmp/current-trident-all-target-check.log` (57.64s Cargo duration) and
`/tmp/current-trisha-all-target-check.log` (58.66s) are hash-bound in the JSON
receipt. Trisha check began only after the tagged genuine proof owner confirmed
its command/monitor completed and released the shared target directory.
No production code changes were required by these final checks.

External dependency deltas were additionally reviewed in
[`external-dependency-continuation.md`](external-dependency-continuation.md).
Their separate cryptographic/source-freeze requirements are not replaced by a
successful Cargo check.
