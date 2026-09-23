# SH0.3 — compiler jobs, results and complete artifacts

Date: 2026-09-23. Trident base `75f84af02543cb67d3ea6acec70ba0214adef76d`.
Delivery: `feat/0.4-sh0-job-codec` into `release/0.4`.
Contract: [native jobs](../../reference/self-hosting-jobs.md).

The delivery defines exact JOB1/PKG1/MOD1/OPT1/LIM1/RES1/ART1/DIA1 records,
canonical module/flag/diagnostic ordering, exact source bytes and versioned module
identities, entry profiles, guest resolution rules, producer/job binding and
failure boundaries. ART1 excludes provenance/counters/admission limits so C2/C3
can have equal executable bytes. RES1 binds the exact job independently.

Generic complete noun transport is implemented in nox's `artifact` module,
merged through [nox PR16](https://github.com/cyberia-to/nox/pull/16) at
`0b84e803f16d7dc3feba0907c5f89ee9582c7131`. Its NOXDAG01 codec preserves topology
and sharing, rejects incomplete/noncanonical containers and enforces byte/node/
depth limits with iterative traversal. The nox workspace passed178 tests,
including nine new codec tests; its own audit records exact source/log hashes.

## Trident conformance evidence

`examples/selfhost_jobs.rs` constructs four complete golden containers and
checks bounded structural admission. The compiler-profile fixture and output
formulas are hand-authored test data. No `.tri` compiler executes in this harness.
It does not lex source, resolve imports or repair generated code on the host.

Nine protocol tests passed, plus all18 existing native-data tests:

- complete container encode/decode round trips and execution of the extracted
  hand-authored nox formula, independently expected to return14;
- exact raw byte preservation, including NUL/CR/LF/invalid UTF-8 (source lexical
  checking remains the guest compiler's responsibility);
- different producer identities change JOB1/RES1 while keeping ART1 bytes equal;
  mismatched compiler, job/result or admitted job context rejects;
- sorted/unique logical names and cfg flags, origin/version labels, valid
  matched entry profiles, supported options and present entry module;
- host caps, requested caps, accumulated validation work, total source byte
  limits and independent compiler-container byte/node/depth admission;
- exact record shape/arity, nonempty error payloads, diagnostic order, module
  and byte spans, error codes and the diagnostic-overflow marker;
- byte-for-byte reproducibility of the four checked-in
  [golden containers](job-vectors.json), including full particle identities.

The SH0.2 reference model gains a borrowed validation allowance so nested records
share work accounting. Its sparse height32 test is invoked only from the data
harness, avoiding duplicate test counts in the job harness. Existing SH0.2
vectors and the compiler source inventory stay unchanged.

The admitted Job stores its root identity and exposes immutable views. Successful
result admission retains both the original ART1 root and formula root, so a
consumer can compare exact artifact bytes without rebuilding metadata. Independent
read-only review prompted checks of the separate compiler container and rejected
mixing of admitted context between jobs. Profile and entry-name ambiguities were
resolved in the contract before freezing these fixtures.

## Reproduce and limits

Use the pinned sibling revisions in
[sh0-native-jobs-validation.json](sh0-native-jobs-validation.json).

```sh
cargo test --release --locked --example selfhost_jobs --example selfhost_data
cargo run --release --locked --example selfhost_jobs -- --output audit/self-hosting/job-vectors.json --check
cargo run --release --locked --example selfhost_data -- --output audit/self-hosting/native-data-vectors.json --check
cargo run --release --locked --example selfhost_inventory -- --root . --output audit/self-hosting/compiler-subset.json --check
cargo check --workspace --all-targets --locked
cargo test --workspace --release --locked
```

The default-feature workspace regression suite passed853 tests, with zero failed
or ignored tests. Source/model checks and rustfmt passed. Reduction, arena-node
and evaluator-frame limits receive range/cap admission in this harness; actual
consumption and the compiler-plus-job loaded arena remain SH0.4/SH1 work. There
is no wall-time/RSS claim, Joy structured-run claim, self-compilation or Zheng
proof claim. Neural features, cross-platform release certification and Triton
performance benchmarks were not rerun for this protocol delivery.

SH0.3 is complete as a contract/conformance substep, with a real shared nox
transport implementation. Next is SH0.4 runtime/control-flow policy, then SH0.5
joint owner review and SH1 source/runtime integration. No overall roadmap
temperature changes at this step. Master and published package versions remain
unchanged; work continues in the 0.4 integration branches.
