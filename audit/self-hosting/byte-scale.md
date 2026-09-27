# Sequential packed-byte scale

The native library now shares tree traversal and processes complete packed-word
subtrees. Canonical BYT1 nouns and validated Bytes handle fields are unchanged.
Source comments and UTF8 error offsets remain part of the input.

All observations below use revision `b453e3444bc5a8ac9c4e7f78910aeab6c59e0581` plus
these working-tree changes. The [receipt](byte-scale.json) records exact source
SHA256 values, commands, allowances and logs. Its baseline command selects the
three committed library modules through `CompileOptions.module_sources`; it never
replaces files in the shared worktree. Measurements execute emitted native
formulas through the nox reducer; the host constructs only the input fixture.

The component allowances are copied from
`tests/native_compiler/source_capacity.rs` and remain identical before and after.
They are explicit component allowances, not a complete JOB1 self-hosting verdict.
The fixture uses the stated number of ASCII space bytes. Mode 0 returns the exact
canonical input noun; mode 1 checks the complete UTF8 success span.

| Bytes | Operation | Before reductions | After reductions | Before nodes | After nodes | Before frames | After frames |
|---|---|---|---|---|---|---|---|
| 128 | admission | 118011 | 14124 | 11840 | 7712 | 410 | 248 |
| 128 | admission + UTF8 | 533942 | 26766 | 18385 | 8697 | 643 | 248 |
| 4096 | admission | 6008331 | 392583 | 341649 | 30153 | 1258 | 1216 |
| 4096 | admission + UTF8 | 28591382 | 718021 | 551194 | 59532 | 1258 | 1216 |
| 16384 | admission | 28360365 | 1563357 | 1532390 | 92376 | 1458 | 1264 |
| 16384 | admission + UTF8 | 136042424 | 2857933 | 2370732 | 209905 | 2707 | 1264 |
| 65536 | admission | 130778415 | 6244671 | 6859493 | 335001 | 2237 | 1456 |
| 65536 | admission + UTF8 | 630908474 | 11413207 | 10213028 | 805402 | 8862 | 1456 |

## Implementation contracts

`tree.Cursor` stores a pending stack of right subtrees with private fields.
`cursor_at` seeks once, and `read` visits each branch once across a complete
sequential traversal. Each call descends at most the existing U32 tree-height
bound. Occupied pair leaves stay opaque and the count prevents padding reads.

`bytes.words`, `words_from`, `word_at`, and `word_byte` expose packed reads without
expanding the Bytes handle. Both offset-taking functions address byte offsets;
`words_from` starts with the containing word and is empty at the byte length.
The original random `get` path remains direct. `word_groups` treats canonical
height-three subtrees as opaque groups of packed words when the source tree is
large enough.

Byte admission now checks shape and payload during one traversal. A full group
retains every shape visit and every historical height-plus-one payload charge,
charged before the group is examined. Partial subtrees descend normally;
wholly unused subtrees must equal canonical padding. Every occupied word receives
a checked U32 conversion, including values whose result is otherwise unused.
The final partial word must have zero unused high lanes. Successful remaining
allowances exactly match the independent host model.

UTF8 uses the grouped cursor. Summing the masked high bits of a group certifies
ASCII only when no continuation is pending. The sum fits in Field. Non-ASCII or
pending continuation states use the original scalar state machine and return
immediately after the same offending byte. Final canonical padding never changes
the source span.

## Verification

The commands and revision for each result are in the receipt. `cargo check --tests`
passes with zero warnings. The final extra padding cases were verified by the
scoped admission rerun after the complete suite. Existing collection
tests pass, including exact allowance boundaries, shared allowances, malformed
nouns, persistent roots, packed lanes and U32 height boundaries. New tests pass
for suffix cursors through the maximum tree height, all byte values in every
packed lane, UTF8 boundary/truncation cases across words, groups and chunks,
and the final source byte. Corruption cases reject a wide atom or pair in every
position of full groups, nonzero unused final byte lanes, and nonzero padding
leaves.

The [formal audit](byte-scale-audit.json) returns `unknown` with exit status 2:
the symbolic executor does not model these aggregate/Noun contracts. It reports
no compiler warnings after removing the unused UTF8 import. This is an executable
conformance and resource improvement, with no symbolic-proof claim.
