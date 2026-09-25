# Function attributes and declaration-owned purity on nox

Source: Trident `6b6e733ff1a54b91729f8e6ddd4f73afa2a3154a`. [Pinned validation](native-compiler-attributes-validation.json)
records the exact commands, sibling revisions and local macOS ARM64 observations.

C1 accepts repeated pure/requires/ensures function prefixes before optional pub.
Contract payloads are lexical metadata, including empty, unknown, incomplete and
more-than-64-level parenthesized text. They emit no checks or audit AST and establish
no formal truth. Unknown forms and exact asm remain unsupported. Malformed
supported delimiters are syntax errors; invalid source tokens remain lexical errors.
A # outside a function prefix retains its prior unsupported diagnostic.

Each declaration owns its pure flag, including replaced definitions. The direct
name check matches the seed's exact I/O names and three prefixes, even when those
names denote ordinary functions. It neither prohibits variable reads nor infers
transitive helper effects. All expression entrypoints carry the same scope.

Sparse pure declaration IDs preserve packed signature bytes. One immutable scope
is constructed per body and shared by expressions; direct body initialization
avoids rebuilding placeholder metadata. Full-name signature lookup reads its
existing packed span without reconstructing unrelated fields. The original
function-depth source and 196608 arena, all public quotas and prior resource
vectors are retained. Temporary larger-arena diagnostics were removed.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --output /tmp/native-guest-attributes-cli.json
```

All 1078 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha cases remain ignored. All 133 fixture rows and 43 manual
baselines match the assertions delivery. Six new tests cover metadata identity,
scanner errors, every exact I/O name and prefix/near-miss, declaration ownership,
all expression contexts and exact declaration caps. Native raw Rust seed execution
and type checking supply differential evidence.

The [installed corpus](native-attributes-cli.json) contains 1190
commands and 401 observations. All 172 prior
positive ART1 identities are unchanged. Seventeen new positive source packages
compile through fresh JOB1 and execute through Joy; nine metadata variants have
identical complete program identity. Twenty-four rejected sources preserve prior
outputs. Source and evidence commits are followed by all-three installs; the
rebuilt binaries and complete C1 match the executed bytes. C1 particle:
`6ee70c5d760a2c01844ac738b08e3ce6977f5ca525d14f9b20829cf26e1e0048`.

Costs from pinned CLI receipts at old source `eca4ea8`
and new source `6b6e733` are charged reductions and lifetime allocations:

| Case | Reductions old → new | Allocated nodes old → new |
|---|---:|---:|
| precedence | 937329 → 930393 | 103023 → 104425 |
| stack64 | 3692967 → 3688511 | 185465 → 187288 |
| body-chunks | 3969185 → 3964477 | 192428 → 193945 |
| record-wide32 | 19669469 → 19680916 | 473580 → 475301 |
| record-typed-call | 2362436 → 2346380 | 131413 → 131719 |
| record-nested | 2713701 → 2708569 | 133197 → 134543 |
| record-long-type | 17198663 → 17192223 | 352421 → 353718 |

All 88 formal audits are UNKNOWN. Retained wide/long-name and 4096-whitespace
workloads keep SH4 open. True imports/intrinsic ownership, seed export/import
prerequisites, complete compiler scale, generated profiles, C2/C3, six platforms
and separate native Zheng compiler proof gates remain open. Noun stays 128K.
