# Explicit compiler-profile seed and SH1 acceptance

Date: 2026-09-24. Trident source
`6b8d4e37b1dbbbc86dcb952c38f667f8277a5cea`, Joy source
`5db7fa3559382de7bc3c93e7712daf28b730e24d`.
Commands, full sibling revisions, earlier fixture syntax failures and final
results: [validation receipt](sh1-profile-validation.json).
Installed execution: [Joy receipt](../../../joy/audit/self-hosting/compiler-profile-cli.json).

The native seed API now explicitly emits raw(0,0) or compiler(1,1) ART1.
Existing raw entry APIs retain the same canonical bytes. Both profiles use the
same pure Noun-to-Noun ABI and formula; the profile changes only ART1 metadata.
Joy exposes `build --emit artifact --artifact-profile compiler-job` and checks
JOB1/RES1 during execution. Selecting that profile does not certify compiler
behavior. Other output formats reject the profile flag before publication.

## Source guest acceptance

The fixture `joy/rs/tests/fixtures/compiler_transport.tri` computes the input
identity and constructs a bound RES1 containing an ART1 whose formula returns14.
The installed CLI builds the fixture, runs it on a supplied JOB1, validates and
extracts its program, then executes that program separately. Complete compiler,
RES1 and ART1 bytes agree with the independent expected containers. Invalid
input preserves an existing output with `--force`.

At the source revisions above, the installed runner recorded708 reductions,
690 allocated nodes and39 peak frames for source-guest execution, then1 reduction,
10 nodes and1 frame for the extracted program. The seed particle is
`ed1a548d05d42b4a664973b2092d5f3ceea57d12e3cfe794d8fd827796b21e21`.
These are local macOS arm64 development observations, not CI or release evidence.
Reservations do not measure peak RSS.

Reproduction from the Joy checkout:

```sh
JOY_SEED_FIXTURES=/tmp/joy-04-seed-fixtures CARGO_TARGET_DIR=../trident/target cargo test --release --locked -p joy-rs --lib trident_source_seed
python3 audit/self-hosting/run-compiler-profile.py --joy ../install/bin/joy --fixtures /tmp/joy-04-seed-fixtures --output audit/self-hosting/compiler-profile-cli.json
```

The fixture deliberately quotes its output program and does not parse supplied
source bytes. SH2 still requires an actual guest compiler.

## Regression gates

At the listed source revisions:

| Command | Result |
|---|---|
| Trident `cargo test --workspace --locked` | 909 passed, no failures/ignored tests |
| Trident `cargo check --workspace --all-targets --locked` | Pass, no Rust warnings |
| Joy `CARGO_TARGET_DIR=../trident/target cargo test --workspace --release --locked` | 112 passed, no failures/ignored tests |
| Joy `CARGO_TARGET_DIR=../trident/target cargo check --workspace --all-targets --locked` | Pass, no Rust warnings |
| Trisha `CARGO_TARGET_DIR=../trident/target cargo test --release --locked -p trisha-rs` | 362 passed, 4 existing heavy proof tests ignored |
| Trisha `CARGO_TARGET_DIR=../trident/target cargo run --release --locked -p trisha -- bench` | 133/133 fixtures, 43/43 independent baselines verified; unchanged cycle rows |

Joy `../trident/target/debug/trident audit rs/tests/fixtures/compiler_transport.tri --json`
reports UNKNOWN (aggregate returns unsupported), exit2. No formal proof is claimed.

## SH1 verdict

The foundation gate is closed by the combined receipts:

- [Native runtime](native-runtime.md): bounded sequential heap frames, lifetime
  arena allowance and traced/run-only agreement.
- [Native Noun](native-noun.md): complete source values and artifact transport.
- [Reusable control](native-control.md): runtime indexing, calls and compact
  loops beyond the old recursive evaluator depth.
- [Private wrappers](native-wrapper-privacy.md) and
  [collections](native-collections.md): canonical persistent Seq/Bytes and bounds.
- [Joy compiler admission](../../../joy/audit/self-hosting/compiler-jobs.md):
  complete identity-bound jobs/results and requested resource limits.
- This source-guest profile/export/execute acceptance.

Next is SH2: guest lexing/parsing and native program generation, with fixed seed
identity and newly supplied source packages. Full source closure, compiler-scale
memory, C2/C3 equality, platform matrix and dynamic proofs retain SH4–SH8 gates.
