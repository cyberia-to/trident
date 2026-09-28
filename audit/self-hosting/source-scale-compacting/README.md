# Full source-size cases under explicit compaction

The valid 65,536-byte comment-padded source failed at the declared 65,536-frame
limit under actual C2. Baseline and exact 4 KiB sources emitted byte-identical
programs that executed to 13; the three source-rejection cases passed their
diagnostic and publication-preservation checks. Five of six distinct cases pass;
SH4 remains open. The original failed receipt is retained unchanged.

The runner uses only C2 published by the successful full-source C1(S) receipt,
binding its exact SHA256 and particle, the producer receipt and immutable Joy
binary. Neither measured run built or replaced a compiler.

Fix the compiler profile before running: 20 billion reductions, one billion
cumulative allocations, 3,145,728 resident nodes, 10 billion collection work,
65,536 evaluator frames, 16,777,216 validation visits and one hour host time.
LIM1 otherwise matches the completed full project profile, with each fixture's
exact source-byte allowance. This is new evidence; historical fixtures and
failure receipts keep their original quotas.

The cases retain exact historical invalid UTF-8 65,536-byte and excess
65,537-byte sources, plus the missing-name diagnostic beyond byte 4,096.
Every source rejection also forces publication over a saved valid program and
requires guest failure with the previous bytes unchanged. Commands, source
bytes, manifests and outputs remain available. This narrow corpus does not
close the full SH5 corpus or any fixed-point/platform/proof gate.

## Measured result: frame frontier remains open

The provided compiler is actual C2 SHA256
`fe0390b92257edf58686e50571160fc7985b0883ede116c6dcfd0a9f7820b5d0`,
particle `84710d31df5911098b614ee0455d7a199bfe5349e57a0f97e031d52dd533eeb3`,
from [the successful C1(S) run](../body-scale/c1-to-c2.json) on frozen source
`b991d901e6585a40bedd0e0a3d4382c2ad3d89c1`. Joy SHA256 is
`506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8`.
The initial [receipt](receipt.json) failed at valid 65,536-byte source. It is
unchanged, including its pending-case marker at the point of failure. The
separate [negative-case receipt](negative-receipt.json) passed its four selected
cases. All original and final compiler/Joy hashes match. Neither run built or
replaced a compiler, and neither raised the declared limits.

| Case | Source bytes | Result | Guest reductions | Peak frames |
|---|---:|---|---:|---:|
| Baseline | 35 | Emits and executes 13 | 366,104 | 397 |
| Valid comment-padded source | 4,096 | Same program bytes; executes 13 | 22,122,424 | 32,772 |
| Valid comment-padded source | 65,536 | `Execution(Frames)`; no program | Not reported on failure | 65,536 |
| Missing name after old comment prefix | 4,139 | Code 5, original span `[4131..4138)` | 23,825,396 | 33,076 |
| Historical invalid UTF-8 source | 65,536 | Code 1, span `[0..1)` | 6,147,232 | 1,619 |
| Historical excess source | 65,537 | Code 7, span `[0..0)` | 100,737 | 312 |

The valid 64 KiB failure occurred after 11.447040292 host seconds, with 1,815,995
resident/total allocations, zero collections and 64,082,200 evaluator
checkpoints. Those checkpoints are not charged reductions. Resident memory was
below the declared cap; increasing gas or collection work would not resolve the
frame limit. All three negatives also re-executed with `--emit program --force`,
returned exit 1 with an explicit guest-compilation diagnostic, and preserved
previous valid program bytes. The valid 64 KiB case remains a genuine SH4 gap.

Exact argv, source files, manifests, JOB1 and output files are named in each
receipt. To reproduce the initial full sequence with a fresh receipt:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -W error audit/self-hosting/source-scale-compacting/check.py --joy ../install-compiler-work-budget/bin/joy --joy-sha256 506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8 --compiler ../measurements/body-scale-closure-20b-files-wnfaub83/result.dag --compiler-sha256 fe0390b92257edf58686e50571160fc7985b0883ede116c6dcfd0a9f7820b5d0 --producer-receipt ../measurements/body-scale-closure-20b.json --producer-receipt-sha256 354d7be504a511dfaf11303e218487d3dcbb8d11d85948af506f5fb6795e6c2f --output /tmp/source-scale-compacting-new.json
```

The second measured command appended
`--case baseline --case diagnostic-after-comment --case exact65536 --case excess65537`
and used a fresh output. Its `complete_corpus=false` flag prevents a subset pass
from being mistaken for the full six-case gate.

## Runner changes after the failure

The first run's exact source is retained as [measured-check.py](measured-check.py),
SHA256 `98191b5f5e874de3cc4efb0a8a2e0839008d2f0b248c2909227d2e90c9310d36`.
The final runner, SHA256
`03a99dd789a54a4bdfc616adfcc121b1d7a1cafcf300f3668b800213b673addb`, adds
explicit guest-error checking after forced publication, preserves the receipt
with null end hashes if a binary disappears, and permits named subsets only
when baseline is included. It does not change sources, program expectations or
any quota. The negative subset used this final runner. Six focused guards pass
under `PYTHONDONTWRITEBYTECODE=1 python3 -W error audit/self-hosting/source-scale-compacting/test_check.py`;
[the final log](tests-final.log) and [initial four-test log](tests.log) remain.

## Read-only localization and next bounded change

The comment lengths in the valid 4 KiB and long-diagnostic cases differ by 38
bytes; observed peak depth differs by 304 frames, exactly eight per comment
byte. The 64 KiB invalid UTF-8 source completes at only 1,619 frames, separating
source admission and the already-chunked UTF-8 scan from the long-trivia failure.
This is strong localization evidence, not an opcode-level failure trace.

The source path is `package_compile.compile` → `module_graph.entry_header` →
`utf8.validate` → `module_header.open` → `path` → `lexer.next` after the program
name. `lexer.next` scans the comment in a single loop of up to 65,537 iterations.
`loop_gen.iteration` invokes itself through `flow.then`/`compose`; Rust native
loop lowering uses the corresponding recursive continuation layout. Nox counts
active parents until postorder completion, so generic tail-frame elimination
would alter the resource contract. Balanced byte-tree lookup depth alone does
not explain the measured linear comment-depth growth.

A bounded chunk helper for long trivia can unwind traversal frames without
changing evaluator semantics. Keep normal short tokens on their existing path;
measure any added C1 code footprint and per-token cost against the unchanged
196,608-node default fixtures (last locals measurement 195,882 nodes). A candidate
is pending on `feat/0.4-source-frame-chunking`, based on `0fe4054`, in the separate
`trident-lexer` worktree; it has no committed fix or accepted result yet. The
frozen 94 production sources remain intact.
After the exact trivia case is repaired, separately examine long identifier and
decimal scanners, `ascii.same_line`, `symbols.same`, name copying and the raw
array/loop-bound decimal scans. Their 65,536 bounds alone do not prove they fit
65,536 active frames. No source optimization is claimed by this audit.

## Exact artifact retention

[artifacts.tar.gz](artifacts.tar.gz) retains every generated source, package,
JOB1, diagnostic result, program and output; [artifacts.json](artifacts.json)
binds every file's bytes/SHA256 and both unchanged receipts. It was produced and
byte-compared with all original files by
`PYTHONDONTWRITEBYTECODE=1 python3 -W error audit/self-hosting/source-scale-compacting/archive.py`.
After the exact 64 KiB failure fixture was independently copied and checked,
the two generated directories moved to the locations in
[retained-directory-moves.json](retained-directory-moves.json). Raw padding and
invalid UTF-8 files are intentionally excluded from the Git patch.

Restore the historical receipt paths without replacing any differing file:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -W error audit/self-hosting/source-scale-compacting/archive.py --restore
```

The final helper also rejects resolved paths that escape the audit directory
through a parent symlink. Three [restore-path tests](archive-tests.log) pass with
`PYTHONDONTWRITEBYTECODE=1 python3 -W error audit/self-hosting/source-scale-compacting/test_archive.py`.
The exact helper used to create and verify the archive is preserved in
[measured-archive.py.gz](measured-archive.py.gz); its decompressed SHA256 is
`45b0c011a487286a622c7f29472c3ffd0f41d9583511d06d70d80942e56a48e2`,
matching `artifacts.json.script_sha256`. This guard-only change did not rerun,
move or modify measurement artifacts.

`--verify` checks archive and restored file hashes. Receipt paths are historical
host metadata; the archive stores relative paths. Restore under the recorded
checkout to reuse those absolute paths, or generate fresh receipts for another
checkout. Source, JOB1 and executable bytes are never normalized.
