# Persistent native record writes

Source: Trident `324a0150ef54f0b9592afb4832c5b41ad101a6ec`. [Pinned validation](native-compiler-record-writes-validation.json)
records sibling revisions, commands and local macOS ARM64 observations.

C1 compiles static field assignments on mutable local roots, including nested
records and complete nominal/Noun/Digest/tuple fields. The RHS executes once
against the old environment. Relative field axes reconstruct only the selected
root slot; old snapshots, sibling fields and other frame slots remain intact.
Ordinary source callees and calls in the RHS use the existing table/frame ABI.

Constructor continuations now decode their field layout once and keep owned
field lists plus an indexed initializer tree. Complete-name and privacy checks
remain enforced. Public Seq/Bytes validation is unchanged. Frequent token/error
updates move to the front of the private body state; body_store transport keeps
its prior schema. These changes preserve the old default-arena corpus.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --output /tmp/native-record-writes-cli.json
```

All 1040 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings.
Four existing Trisha cases remain ignored. All 133 fixture result/cycle rows and
43 manual baselines match the previous delivery. Six new tests cover complete
values, snapshots, callee/RHS calls, lexical slots, single-evaluation traces,
runtime traps, semantic rejection and exact AST/formula-depth limits, plus the
explicit full-source resource boundary described below.

The [installed corpus](native-record-writes-cli.json) runs 773
commands with 262 observations. All 119 prior positive ART1
identities remain identical; eight new positive JOB1 programs execute on Joy.
Compilation/runtime failures preserve existing output files. The CLI run began
on the eventual source commit's unchanged implementation tree. Post-commit
owner installs reproduce all three binaries, and rebuilding C1 reproduces its
complete executed artifact. C1 particle:
`a76d4eeeaf5be4a467c6fb75bb323191a86c455f0468790e72ac3417afc4ab02`.

Costs from the pinned CLI receipts at old source `60ea10e` and new source
`324a015` are charged reductions and lifetime arena allocations:

| Case | Reductions old → new | Allocated nodes old → new |
|---|---:|---:|
| precedence | 1082747 → 1081367 | 95290 → 96923 |
| stack64 | 4755242 → 4753806 | 191733 → 193353 |
| body-chunks | 4967989 → 4936297 | 195610 → 193070 |
| record-wide32 | 31715955 → 24136175 | 647971 → 483420 |
| record-typed-call | 2801038 → 2787354 | 126019 → 127444 |
| record-nested | 3258938 → 3207103 | 128374 → 129237 |
| record-long-type | 20993086 → 20980601 | 350479 → 351968 |

The direct emitter component executes seven path cases: combined 61–65-bit
paths and chains of 63/64 PROJECT nodes. It compares complete old/new records,
including every sibling, and isolates codegen from source admission. The full
compiler still exhausts 786432 nodes for the five unchanged wide source
programs in `record-write-wide-61-arena` through `record-write-wide-65-arena`.
They remain explicit SH4 resource boundaries, outside positive full-C1
acceptance. The combined long-name and 4096-whitespace boundaries also remain
open. Each retained wide source compiles and evaluates to 3199 through the Rust
seed; this does not establish guest compiler acceptance.

All 74 formal audits return UNKNOWN and establish no formal proof. Arrays,
constants/attributes/asserts, true imports, full SH3/SH4, generated compiler
profiles, C2/C3, six-platform acceptance and native Zheng compilation proofs
remain open. Noun roadmap temperature remains 128K.
