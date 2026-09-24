# Native compiler: Digest identity and checked reads

Source: Trident `dbf3c13940570932ab19932392c8854155d9d7e3`, Joy `d662ef9f07b6904783c98fd215f75764dfca5254`,
nox `c9f7486a74fe81bfc194b598da40f6343ecb2ef1`, Trisha `908d5e22a669e4aa0b6c02b5e9134e67c1a25675`.
[The pinned validation](sh3-native-digest-validation.json) records every sibling,
command and local macOS ARM64 observation.

The compiler running inside nox accepts Digest types and
`nox_noun_identity(Noun)->Digest`, complete Digest equality, and checked read
indexing with Field/U32 expressions. A Digest occupies one native frame slot
and remains the balanced four-word identity through locals, helper calls and
loop assignments. Entry ABIs remain the scalar or raw Noun forms.

A read evaluates the base once, then the index once, and checks index<4 at
runtime. Even literal out-of-range indices compile and trap only when reached,
matching the Rust raw seed. Decimal Field normalization precedes the check.
Index brackets can follow across LF. Wrong types and malformed delimiters reject
before program publication; indexed writes and arrays remain outside this
increment. [The subset contract](../../reference/self-hosting.md#native-compiler-subset-contract)
records the language and bounds.

## Validation

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-digest-cli.json
```

At these revisions, eight Digest tests check all four identity words against
independently encoded input particles, including atoms and nested/shared trees.
They also cover persistence, calls, loops, equal/unequal identities, postfix
precedence/newlines, dynamic indices, malformed delimiters and preserved array
diagnostics. The negative differential oracle uses raw ART1 and first accepts a
valid Noun-entry control. Distinct traps establish base-before-index order;
trace rows establish once-only evaluation. Exact runtime limits, node quotas
and delimiter capacity execute. Independent DAG depth checks exercise shallow
guards and separately dominant base/index formulas.

All 1007 Trident, 122 Joy and 380 Trisha CPU tests pass with zero
Rust warnings; four existing Trisha cases remain ignored. All 133 fixture
result/cycle rows and 43 manual baselines match the type-descriptor delivery.
Formal audits of 46 compiler modules, entry and nine fixtures return UNKNOWN;
these execution results establish no compiler proof.

After rebuilding installed owners from the committed source, the
[CLI receipt](native-digest-cli.json) records 536 commands and 181
observations. The fixed C1 precedes creation of fresh source packages. It performs
compilation in nox, and Joy publishes then separately executes its returned
program. All 81 earlier positive ART1 identities remain unchanged. New
Digest jobs request 786432 arena nodes; emitted programs use Joy defaults.
Failed compiler jobs and runtime traps preserve previous output files.
C1 particle: `308ec57abfba07868bf91fbc58d8bf6c2c2b6986748d1d1b99cf08fdc54addc4`.

The following costs use those commands and revisions. Allocated nodes count
lifetime allocations, not reserved memory or RSS.

| Case | Program bytes | Compile reductions | Compile nodes | Run reductions | Run nodes | Run frames |
|---|---:|---:|---:|---:|---:|---:|
| precedence | 1536 | 1083860 | 80332 | 5 | 22 | 3 |
| body-chunks | 5804 | 4964610 | 178560 | 92 | 82 | 15 |
| function-nested | 5871 | 3461678 | 140213 | 38 | 76 | 10 |
| loop-count-5000 | 15832 | 2361018 | 103559 | 335051 | 70156 | 40012 |
| noun-input | 1745 | 1021236 | 72676 | 8 | 25 | 4 |
| noun-padded-frame | 9374 | 2524270 | 101510 | 66 | 129 | 11 |
| noun-share | 1939 | 1750046 | 84648 | 10 | 28 | 4 |
| noun-helper | 3088 | 1978925 | 94778 | 18 | 40 | 6 |
| noun-loop | 16373 | 3676384 | 124207 | 260 | 228 | 36 |
| digest-identity | 8919 | 6728611 | 179617 | 352 | 117 | 11 |
| digest-helper | 10262 | 7593874 | 201398 | 362 | 134 | 12 |
| digest-loop | 21540 | 8696313 | 214340 | 596 | 289 | 36 |
| digest-u32-index | 6882 | 2757252 | 108664 | 167 | 89 | 8 |

Compilation overhead compared with descriptor delivery `adbb00e`:

- precedence: 1079313 → 1083860 reductions; 78484 → 80332 nodes.
- body-chunks: 4942607 → 4964610 reductions; 176500 → 178560 nodes.
- function-nested: 3454952 → 3461678 reductions; 138294 → 140213 nodes.
- loop-count-5000: 2357435 → 2361018 reductions; 101719 → 103559 nodes.
- noun-input: 1020765 → 1021236 reductions; 70867 → 72676 nodes.
- noun-padded-frame: 2521395 → 2524270 reductions; 99678 → 101510 nodes.
- noun-share: 1748499 → 1750046 reductions; 82823 → 84648 nodes.
- noun-helper: 1977681 → 1978925 reductions; 92947 → 94778 nodes.
- noun-loop: 3672695 → 3676384 reductions; 122354 → 124207 nodes.

Next: tuple source types/values and destructuring, then nominal structs/fixed
arrays, constants, attributes and real imports. Full compiler scale, C2/C3,
six-platform acceptance and native Zheng compiler proofs remain open. Noun stays
128K; the valid 4096-byte whitespace workload remains an SH4 issue.
