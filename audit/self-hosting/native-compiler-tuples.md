# Native compiler: typed tuples and ordered destructuring

Source: Trident `850525c18fc5683f3a2da9251ca9ff455973d0f6`, Joy `d662ef9f07b6904783c98fd215f75764dfca5254`,
nox `c9f7486a74fe81bfc194b598da40f6343ecb2ef1`, Trisha `908d5e22a669e4aa0b6c02b5e9134e67c1a25675`.
[The pinned validation](sh3-native-tuples-validation.json) records every sibling,
command and local macOS ARM64 observation.

The compiler running inside nox accepts tuple annotations, values, typed
arguments/results, flat destructuring declarations and tuple assignment.
Nested values remain complete native subtrees; Unit retains its zero component.
Tuple values use zero-terminated cons-lists, while Digest destructuring follows
its four balanced limbs. Equality requires identical types without recursive
Noun components. [The subset contract](../../reference/self-hosting.md#native-compiler-subset-contract)
records the grammar, diagnostics and independent quotas.

Destructuring evaluates the RHS once before named bindings or ordered writes.
Swaps read both old values; duplicate names/targets leave the last component.
Discard patterns evaluate the whole RHS and consume no named projection/write.
Bare discard declarations, nested patterns, tuple source indexing and
parenthesized single-name assignment remain outside this subset.

## Validation

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-tuples-cli.json
```

Nine added native compiler tests check full nested outputs against the raw Rust seed, tuple and
Digest transport through functions, grouping/call/index delimiter ownership,
annotation grammar, duplicates, scopes, persistence, equality, traps and ordered
evaluation. A passing raw-entry control anchors the negative oracle. Canonical
source-type descriptors and first unread token spans are checked independently.
A separate scalar differential checks tuple discard recognition and extends the
compiling-entry census without claiming whole-module coverage.
Exact type depth/arity/logical-node, hidden storage, mixed operator-stack and
runtime limits execute. Independent formula-depth checks separately exercise
first and last component dominance and returned tuple projections.

All 1017 Trident, 122 Joy and 380 Trisha CPU tests pass with zero
Rust warnings; four existing Trisha cases remain ignored. All 133 fixture
result/cycle rows and 43 manual baselines match the Digest delivery.
Formal audits of 52 compiler modules, entry and eleven fixtures return UNKNOWN;
these execution results establish no compiler proof.

The old 64-group default-arena acceptance remains in force. Tuple frames retain
only start/base/count/head and derive type children once at close. Ordinary
groups avoid tuple state; their frequently edited frame list leads the
persistent parser record, reducing path copying. Lexer slash lookahead is
conditional and parentheses precede identifier/decimal classification.
The previously rejected resource workload is retained as an SH4 issue.

At the exact source tree recorded by the commit above, the
[CLI receipt](native-tuples-cli.json) records 614 commands and 207
observations. The fixed C1 precedes creation of fresh packages. It compiles
inside nox; Joy publishes and separately executes the returned ART1 program.
All 91 earlier positive program identities are unchanged. New tuple jobs
request 786432 arena nodes; emitted programs use Joy defaults. Compiler errors
and runtime traps preserve previous output files.
The post-commit rebuild reproduces all three installed binaries byte for byte,
and rebuilding C1 reproduces the complete executed artifact. The pinned receipt
records both sets of hashes and the confirming build command.
C1 particle: `1be92f26a0ed739ff3c2b9fe43dd3bbf6d5ed6b21ae242a8769084535b74b792`.

Costs below use those commands and revisions. Allocated nodes are lifetime
allocations, independent of reserved memory or RSS.

| Case | Program bytes | Compile reductions | Compile nodes | Run reductions | Run nodes | Run frames |
|---|---:|---:|---:|---:|---:|---:|
| precedence | 1536 | 1077593 | 88346 | 5 | 22 | 3 |
| stack64 | 749 | 4727716 | 183836 | 1 | 9 | 1 |
| body-chunks | 5804 | 4947310 | 187857 | 92 | 82 | 15 |
| function-nested | 5871 | 3439899 | 148320 | 38 | 76 | 10 |
| noun-input | 1745 | 1019820 | 80557 | 8 | 25 | 4 |
| digest-identity | 8919 | 6677345 | 185844 | 352 | 117 | 11 |
| tuple-whole-noun | 7755 | 2912070 | 126752 | 57 | 106 | 10 |
| tuple-helper | 11135 | 4496512 | 158344 | 79 | 145 | 13 |
| tuple-nested-types | 22700 | 6076378 | 199582 | 240 | 289 | 26 |
| tuple-swap | 14071 | 4417726 | 174592 | 114 | 194 | 14 |
| tuple-unit | 18462 | 5226208 | 180026 | 158 | 233 | 22 |
| tuple-digest-helper | 13959 | 7592228 | 207670 | 110 | 194 | 14 |

Compilation cost compared with Digest delivery `dbf3c13`:

- precedence: 1083860 → 1077593 reductions; 80332 → 88346 nodes.
- stack64: 5286905 → 4727716 reductions; 169976 → 183836 nodes.
- body-chunks: 4964610 → 4947310 reductions; 178560 → 187857 nodes.
- function-nested: 3461678 → 3439899 reductions; 140213 → 148320 nodes.
- noun-input: 1021236 → 1019820 reductions; 72676 → 80557 nodes.
- digest-identity: 6728611 → 6677345 reductions; 179617 → 185844 nodes.

Next: nominal structs/private fields and persistent nested writes, then fixed
arrays, constants, attributes/asserts and real imports. Full compiler scale,
C2/C3, six-platform acceptance and native Zheng proofs remain open. Noun stays
128K; the valid 4096-byte whitespace workload remains an SH4 issue.
