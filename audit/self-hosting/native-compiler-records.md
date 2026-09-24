# Native source structures and field reads

Source: Trident `60ea10e0832c9a59cdde53a9d7bfee6a8cb8254f`. [Pinned validation](native-compiler-records-validation.json)
records sibling revisions, commands and local macOS ARM64 observations.

C1 admits module-local nominal declarations, typed annotations/functions,
constructors with named or shorthand initializers, and postfix field reads.
Empty structures and up to 32 fields retain complete names and native values.
Layouts and signatures resolve source-order types; function bodies see the final
immutable registry. Constructors evaluate once in declaration order, independent
of initializer order. Field reads evaluate their base once. Private field and
nominal identity rules come from the accepted descriptor owner.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --output /tmp/native-records-cli.json
```

All 1034 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings.
Four existing Trisha tests remain ignored. All 133 fixture result/cycle rows
and 43 manual baselines match the previous nominal-descriptor delivery.
Eight new tests compare complete native results with independent expected values
and the Rust seed, inspect trace counts, distinguish traps in declaration order,
and exercise exact registry, field-count and emitted-depth boundaries.

The [installed corpus](native-records-cli.json) runs 712 commands
with 240 observations. All 105 previous positive ART1
identities remain identical; 14 new positive programs execute on Joy. Negative
cases preserve earlier program/output files across compilation and runtime errors.
The guest compiler alone reads fresh source JOB1 packages after C1 is built.

The CLI run began on the eventual source commit's unchanged implementation tree.
Post-commit owner installs reproduce all three binary files byte for byte, and a
fresh build reproduces the complete executed C1 artifact. C1 particle:
`d0e003e714652ee701ff97db5d67192a46bef9571214d30693733c3f111b25f7`.

Costs below come from the pinned installed receipts at old source `1e36ceb`
and new source `60ea10e`. Counts are charged reductions and lifetime
arena allocations; the existing 64-group/default-arena case is retained.

| Case | Reductions old → new | Allocated nodes old → new |
|---|---:|---:|
| precedence | 1077593 → 1082747 | 88395 → 95290 |
| stack64 | 4727716 → 4755242 | 183885 → 191733 |
| body-chunks | 4947310 → 4967989 | 187906 → 195610 |
| tuple-whole-noun | 2912141 → 2920161 | 126807 → 134055 |
| tuple-nested-types | 6076620 → 6092867 | 199643 → 207689 |
| tuple-digest-helper | 7592228 → 7607836 | 207719 → 215710 |

The new record cases use an explicit 786432-node compiler allowance. Their
emitted programs execute with Joy's default allowance. Separate 257-byte type
and field-name cases succeed; a valid combined workload repeating those names
exhausts the 786432-node arena. Its complete source and protected-output check
remain in the receipt as `record-long-arena`. The older valid 4096-whitespace
case also remains a resource failure. Both are open SH4 scaling work.

Unknown nominal types and field selection on scalar/Noun values now receive
semantic diagnostic 5; qualified type syntax still receives unsupported code 6.
All 71 formal audits return UNKNOWN and establish no formal proof. Static field
writes, arrays, constants/attributes/asserts, true imports, full SH3/SH4,
generated compiler profiles, C2/C3, six-platform acceptance and native Zheng
compilation proofs remain open. Noun roadmap temperature remains 128K.
