# Bounded lexer trivia scans

This local candidate was measured from base revision
`0fe4054` with the exact source snapshots retained in
`source-snapshots.tar.gz`. The 94-module baseline was captured once from the
successful full-compiler package. Only `std.compiler.nox.lexer` changes in
the final package: 369820 source bytes become 370544. The final lexer
SHA-256 is `8c21e20b82854e7f82023c275ded0802ab345fc6155ea144f1d8ae48c0bf58e2`.
Execution revisions remain the base revision plus these explicit source
hashes; subsequent commits do not relabel when the tests ran.

`lexer.next` scans trivia in 256 chunks of 256 bytes. Returning from each
inner loop releases its pending continuations while keeping the source
offset and comment state. Its existing source/start preconditions make
complete loop exhaustion possible only at exact EOF. Token construction,
dot lookahead and residual unsupported punctuation checks avoid redundant
calls and branches so the original small-program node ceiling still holds.
The public `ascii.punctuation` helper remains unchanged; ten of its twelve
bytes already return in earlier lexer branches, leaving only `>` and `^`
at the final check.

Two Rust seeds were built by `selfhost_seed_snapshot` from explicit frozen
manifests. The initial baseline seed is exactly the earlier full-bootstrap
seed (`4b8276068704063eb13e5555bca872372d2975b4ac71cf93edc03273222327b6`).
The final candidate seed is
`5728e07a37e111f88166c471ad338f8947b74dfdc842ed1ffa9fa0886c005fda`.
Its artifact has 100857 DAG entries and 9715917 bytes, compared with
100760 entries and 9706396 bytes before. These are C0-built C1 artifacts.

The paired source-size commands use the same six source byte sequences,
options and explicit limits. The pinned Joy binary SHA-256 is
`506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8`.
Limits remain 20 billion reductions, one billion cumulative allocations,
3145728 resident nodes, ten billion units of collection work, 65536 frames,
16777216 validation visits and a one-hour deadline. Short ordinary
program execution retains its separate one-million-reduction/196608-node
limits.

| Same source case | Before reductions | Final reductions | Before peak frames | Final peak frames |
| --- | ---: | ---: | ---: | ---: |
| Short program | 421452 | 423571 | 465 | 465 |
| Valid 4096-byte comment program | 23039150 | 23076041 | 32777 | 2385 |
| Valid 65536-byte comment program | unknown; frame failure | 502000635 | 65536 | 4329 |
| Diagnostic after a long comment | 24732947 | 24770393 | 33081 | 2393 |
| Invalid UTF-8 at exactly 65536 bytes | 6236588 | 6236588 | 1577 | 1577 |
| Source exceeding the ceiling by one byte | 107881 | 107881 | 316 | 316 |

The final 65536-byte case allocates 8740499 nodes cumulatively and performs
two collections within the unchanged resident ceiling. Every positive
candidate output is byte-identical to its short-program output and executes
independently to 13. The negative cases retain diagnostic codes 5, 1 and 7;
the long-comment diagnostic retains the original `missing` byte span.
Failed execution does not expose charged gas; its evaluator checkpoint count
is not substituted for that value.

The old default tests also pass without changing their code or limits:

| Case | Reductions | Allocated nodes | Peak frames |
| --- | ---: | ---: | ---: |
| Nine assignments/body chunks | 2817597 | 196590 | 629 |
| 64 nested groups | 2328546 | 194969 | 737 |
| Seven assignments | 2313369 | 177175 | 629 |
| Eight assignments | 2574366 | 187902 | 629 |

The tight case has 18 nodes of headroom under its original 196608-node
ceiling. Earlier candidates failed this gate; their exact sources and logs
are retained. The reported excess-node counts came from the pre-existing
786432-node diagnostic path after the original test failed. They are
diagnostics, not acceptance under an expanded quota. The direct literal
`kind` experiment also failed type checking before explicit U32 conversion
was added.

The component tests compare the live lexer with the exact baseline lexer
for every byte class, token/lookahead boundaries, comment transitions,
arbitrary admitted starting offsets and exact token spans/values. Full
65536-byte whitespace and comment scans use the existing source component
limits, including the unchanged 65536-frame ceiling.

This unit bounds trivia/comment scanning in `lexer.next`. The separate flat
identifier, decimal, line and name scans remain outside this measured scope.
The C2/C3 bootstrap and supplied-compiler semantic corpora have separate
receipts.

The complete Rust gate is recorded in `full-gate.json`: 1197 distinct tests
passed, five existing tests remained ignored, and no Rust warnings were
emitted. All 46 Cargo test targets are covered by three disjoint primary
commands: the native compiler/component targets, every remaining library,
binary and integration target, and doctests. The review added newline-at-byte
255/511 vectors; a supplemental run passed both component tests again.
There are 1199 observed passing test executions including that rerun.
`test-before-review.rs.gz` retains the exact earlier test source. The final
production source is unchanged across these runs.

Exact commands, working directories, environments, source hashes and raw
log hashes are in the gate. `cargo-metadata.json.gz` and the Cargo manifest
hash bind the complete target list. Individual logs are retained along
with `full-rust.log.gz`, an exact concatenation without filtering. Expected
negative tests print language/CLI errors; every Cargo command exits zero
and every suite reports zero failed tests.

The paired measurements were launched from the isolated Trident worktree:

```sh
python3 ../measurements/lexer-frame/check-pair.py > ../measurements/lexer-frame/check-pair.log 2>&1
python3 ../measurements/lexer-frame/check-pair-v9.py > ../measurements/lexer-frame/check-pair-v9.log 2>&1
CARGO_TARGET_DIR=../target-lexer cargo test --release --locked --offline --test native_compiler native_constants_keep_previous_default_arena_and_artifacts -- --nocapture > ../measurements/lexer-frame/default-constants-v9.log 2>&1
CARGO_TARGET_DIR=../target-lexer cargo test --release --locked --offline --test native_compiler locals_cross_parser_and_emitter_chunks -- --nocapture > ../measurements/lexer-frame/default-locals-v9.log 2>&1
```

`seed-builds.json.gz` and `seed-build-v9.json.gz` retain the exact seed-builder
commands and binary identity. `pair.json.gz` contains the baseline and the
initial chunking experiment; `pair-v9.json.gz` binds that baseline receipt
and contains the final candidate's six cases. Both scripts and all discarded
variant logs are preserved by `measurement-evidence.json`. Intermediate
variants are evidence of rejected implementations; the final source alone
is selected for acceptance.
