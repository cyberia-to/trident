# Fixed Field arrays on nox

Source: Trident `a4275317a15eb8b58c5eca91a63c30e1515b0b44`. [Pinned validation](native-compiler-arrays-validation.json)
records commands, sibling revisions and local macOS ARM64 observations.

C1 accepts fixed Field-array annotations and literals, complete value transfer,
whole-array replacement, equality and Field/U32 reads. A shared checked Get
helper follows reachable source functions and loops. Base and index execute once
in source order. Literal index checks use original u64 spelling before Field
normalization. Empty arrays, snapshots and array fields retain exact types.

Array markers own their delimiters. Helper metadata follows every expression
boundary and is stored in checked bodies. Emission restores a compact record;
public Seq/Bytes validation stays intact. Hot fields and reused lexer lookahead
preserve the existing default-arena corpus.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --output /tmp/native-arrays-cli.json
```

All 1048 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha cases remain ignored. All 133 fixture rows and 43 manual
baselines match the prior delivery. New tests cover canonical types, logical
quotas, empty/singleton/24/31/32/33/65 elements, nested values, source-order traps,
once-only traces, helper/table boundaries and independent complete artifact depth.
The independent oracle uses the native raw Rust seed; its legacy assembly API
has no dynamic array indexing.

The [installed corpus](native-arrays-cli.json) runs 851 commands
with 288 observations. All 127 prior positive ART1
identities remain identical, with 14 additional positive fresh JOB1 programs.
Compilation and runtime failures preserve prior output files. The CLI started
on the eventual source commit's unchanged production tree. Post-commit installs
reproduce all three binaries and the complete executed C1 artifact. C1 particle:
`d9c86de1d644e7826a75ccbf264c4df4dba36306c3173f18935798a523f8428e`.

Costs from the pinned CLI receipts at old source `324a015` and new source
`a427531` are charged reductions and lifetime arena allocations:

| Case | Reductions old → new | Allocated nodes old → new |
|---|---:|---:|
| precedence | 1081367 → 1075517 | 96923 → 99547 |
| stack64 | 4753806 → 4730100 | 193353 → 192984 |
| body-chunks | 4936297 → 4734596 | 193070 → 194469 |
| record-wide32 | 24136175 → 24085412 | 483420 → 483147 |
| record-typed-call | 2787354 → 2761215 | 127444 → 129206 |
| record-nested | 3207103 → 3194145 | 129237 → 131553 |
| record-long-type | 20980601 → 20965901 | 351968 → 354445 |

The 4096-function component test isolates code-table completion from source
parsing and body generation. Its test-only arena permits 12582912 nodes and its
budget permits 1000000000 reductions; this establishes the final planner chunk's
semantics, not whole-source acceptance within the JOB1 arena. Default 196608 and
requested 786432 source-compiler allowances remain unchanged. Actual component
costs and their command are recorded in the validation JSON.

The retained wide record sources, combined long names and 4096-whitespace
workload remain explicit SH4 resource boundaries. All 79 formal audits are
UNKNOWN and establish no proof. Constants/attributes/asserts, true imports,
full SH3/SH4, generated profiles, C2/C3, six platforms and native Zheng compiler
proofs remain open. Noun temperature remains 128K.
