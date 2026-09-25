# Typed constants in the native compiler

Source: Trident `0051cd578d72eafeebf609033ac39d1bb9a63ef5`. [Pinned validation](native-compiler-constants-validation.json)
records commands, sibling revisions and local macOS ARM64 observations.

C1 resolves module-local Field and U32 constants before checking function
bodies. Literal and exact-type alias initializers support forward references,
final-name replacement and grouping. Every declaration is checked, including
replaced and unused initializers. Cycles, unknown references, raw U32 overflow
and unsupported initializer expressions diagnose before artifact publication.
Full name bytes, final visibility and original literal spans remain available.

Local and parameter bindings shadow constants. A distinct constant AST kind
keeps runtime Field normalization separate from raw literal array-index checks.
Constants add no emitted wrapper or function table entry. Declaration counts,
group nesting and expression nodes retain independent limits. Named array
extents, named loop bounds and qualified imports remain outside this slice.

The larger parser context initially exceeded an existing resource gate. Hot
state fields now precede cold metadata; typed zero constants replace redundant
checked conversions. Balanced tree geometry specializes small lengths, and
empty/singleton updates return after their original bounds assertions. Tests
compare height and capacity separately at every U32 power boundary and verify
pair leaves, persistent snapshots and invalid indices. Canonical layout and
collection validation visit accounting are unchanged. The original function
depth fixture, source and 196608-node arena pass without weakened budgets.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --output /tmp/native-guest-constants-cli.json
```

All 1066 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha cases remain ignored. All 133 fixture rows and 43 manual
baselines match the array delivery. New tests cover final typed bindings,
raw integer boundaries, visibility/provenance, alias chains, local shadowing,
coverage, distinct literal/computed indexing and independent resource caps.

The [installed corpus](native-constants-cli.json) contains 965
commands and 326 observations. All 141 prior
positive ART1 identities remain identical; 19 new positive source packages
compile through fresh JOB1 and execute through Joy. Eighteen rejected sources
preserve earlier programs; a constant index outside its array compiles and
then traps without replacing earlier runtime output. The CLI started on the
eventual source commit's unchanged production tree. Post-commit installs
reproduce all three binaries and the complete executed C1 artifact. C1 particle:
`4835d7f3d7259e92c3223e161f39061c76d024f634c67f7d7ba201b9aa71e7d0`.

Costs from the pinned CLI receipts at old source `a427531` and new source
`0051cd5` are charged reductions and lifetime arena allocations:

| Case | Reductions old → new | Allocated nodes old → new |
|---|---:|---:|
| precedence | 1075517 → 937649 | 99547 → 102389 |
| stack64 | 4730100 → 3693287 | 192984 → 184832 |
| body-chunks | 4734596 → 4002744 | 194469 → 192419 |
| record-wide32 | 24085412 → 19670520 | 483147 → 472965 |
| record-typed-call | 2761215 → 2362999 | 129206 → 130798 |
| record-nested | 3194145 → 2714021 | 131553 → 132564 |
| record-long-type | 20965901 → 17198983 | 354445 → 351789 |

Valid wide-record sources, combined long names and the 4096-whitespace workload
retain explicit SH4 resource boundaries. All 83 formal audits are UNKNOWN and
establish no proof. Function attributes/asserts, true imports, full SH3/SH4,
generated profiles, C2/C3, six platforms and native Zheng compiler proofs remain
open. Noun temperature stays 128K.
