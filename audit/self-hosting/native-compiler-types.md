# Native compiler: canonical type descriptors

Source: Trident `adbb00ece15377c53f8665f776ada9107e0d2a26`, Joy `d662ef9f07b6904783c98fd215f75764dfca5254`,
nox `c9f7486a74fe81bfc194b598da40f6343ecb2ef1`, Trisha `908d5e22a669e4aa0b6c02b5e9134e67c1a25675`.
[The pinned validation](sh3-native-types-validation.json) records every sibling,
command and local macOS ARM64 observation.

Compiler type records now preserve complete canonical Noun descriptors through
AST, bindings, signatures and function-body storage. Primitive records keep
their exact bytes. The bounded tuple constructor derives ordered child identity,
logical node count, nesting depth and recursive Noun containment. A shared
subtree contributes for every logical occurrence. Constructor-produced metadata
is trusted internally; source packages continue to supply bytes.

This delivery supplies the internal foundation for source aggregates. Digest
operations and tuple syntax remain the next language increments. Their absence
keeps SH3 open. [The contract](../../reference/self-hosting.md#native-compiler-subset-contract)
defines type ownership and independent capacity bounds.

## Validation

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-types-cli.json
```

At the revisions above, six descriptor tests check primitive record bytes,
complete nested type round trips, ordered child identity, recursive Noun
containment, invalid/empty children, arity16, depth64 and logical node boundaries
4095/4096/4097. The shared-DAG cases reject complexity beyond the logical
allowance even when storage shares every repeated child. Node, depth and arity
limits are checked independently at their exact boundary and one below.

All 999 Trident, 122 Joy and 380 Trisha CPU tests pass; four
existing Trisha cases remain ignored. There are zero Rust warnings. All 133
Trisha fixture result/cycle rows and 43 manual baselines match the previous
Noun delivery. Formal audits of 44 compiler library modules, entry and seven
fixtures return UNKNOWN; this is no formal compiler proof.

After rebuilding the installed owners from the committed source, the
[CLI receipt](native-types-cli.json) records 476 commands and 161
observations. Every previous positive case, including complete Noun outputs,
retains its ART1 particle (81 cases). The fixed C1 precedes fresh source
creation and runs compilation inside nox; Joy publishes and executes the emitted
program. C1 particle: `d55ac5797381e7f8fb808892114bd144300ff3396523c48b7e08c95a20e7c366`.

The following costs use that command and the pinned revisions above. Allocated
nodes are lifetime allocations, not reserved memory or RSS.

| Case | Program bytes | Compile reductions | Compile nodes | Run reductions | Run nodes | Run frames |
|---|---:|---:|---:|---:|---:|---:|
| precedence | 1536 | 1079313 | 78484 | 5 | 22 | 3 |
| body-chunks | 5804 | 4942607 | 176500 | 92 | 82 | 15 |
| function-nested | 5871 | 3454952 | 138294 | 38 | 76 | 10 |
| loop-count-5000 | 15832 | 2357435 | 101719 | 335051 | 70156 | 40012 |
| noun-input | 1745 | 1020765 | 70867 | 8 | 25 | 4 |
| noun-padded-frame | 9374 | 2521395 | 99678 | 66 | 129 | 11 |
| noun-share | 1939 | 1748499 | 82823 | 10 | 28 | 4 |
| noun-helper | 3088 | 1977681 | 92947 | 18 | 40 | 6 |
| noun-loop | 16373 | 3672695 | 122354 | 260 | 228 | 36 |

Compilation overhead compared with Noun delivery `c5d1901`:

- precedence: 1079517 → 1079313 reductions; 78314 → 78484 nodes.
- body-chunks: 4943536 → 4942607 reductions; 176330 → 176500 nodes.
- function-nested: 3455329 → 3454952 reductions; 138125 → 138294 nodes.
- loop-count-5000: 2357556 → 2357435 reductions; 101550 → 101719 nodes.
- noun-input: 1020783 → 1020765 reductions; 70697 → 70867 nodes.
- noun-padded-frame: 2521437 → 2521395 reductions; 99509 → 99678 nodes.
- noun-share: 1748525 → 1748499 reductions; 82653 → 82823 nodes.
- noun-helper: 1977727 → 1977681 reductions; 92777 → 92947 nodes.
- noun-loop: 3672741 → 3672695 reductions; 122185 → 122354 nodes.

Next: Digest identity/read indexing, source tuples and destructuring, then nominal
structs/fixed arrays, constants, attributes and real imports. Full compiler scale,
C2/C3, six-platform acceptance and native Zheng compiler proofs remain open.
Noun stays 128K; the valid 4096-byte whitespace workload remains an SH4 issue.
