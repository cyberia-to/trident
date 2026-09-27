# Complete C1(S) produces C2 through Joy/nox

The complete compiler source at Trident
`b991d901e6585a40bedd0e0a3d4382c2ad3d89c1` compiled successfully inside
the installed Joy worker on macOS ARM64. The actual output is retained as
[c2.dag.gz](c2.dag.gz), an independently loadable compiler-profile ART1.
This receipt establishes full source compilation. SH5 additionally requires
the independent corpus to pass through this C2; SH6 requires the next build,
exact fixed point, C3 corpus and six-platform reproduction.

## Inputs and command

The frozen [inventory](inventory.json) contains 94 modules, 484 functions,
369,820 source bytes and 8,722 lines. Its SHA256 is
`c121df82df67e1672e83cfc30b94a9161cde551b1be1890ff1e075c070ef2ea9`.
Every captured source SHA256 matches the committed module. The
[seed identity](seed-identity.json) records the explicit frozen source manifest
and Rust seed-builder command. No host compiler stage participates after C1.

[Installed Joy](joy-compiler-work-budget-install.json) is
`2878f4b17dfedf237c6110d7d411bb4824e65103`, with nox
`1eaa8a494f994e4da6b20509469c1e62624f3f2d` and the above Trident revision.
Binary SHA256 is
`506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8`.
The run uses the explicit bounded compaction tier; ordinary fixture limits
remain unchanged. The [complete receipt](c1-to-c2.json) retains all commands,
stdout/stderr, original paths, source hashes, options, limits and execution.
From the Trident worktree the command was:

```sh
CARGO_TARGET_DIR=../target-probe python3 audit/self-hosting/probe-native-closure.py --joy ../install-compiler-work-budget/bin/joy --compiler ../measurements/body-scale-c1.dag --inventory ../measurements/body-scale-inventory.json --output ../measurements/body-scale-closure-20b.json --budget 20000000000 --arena-nodes 1000000000 --time-ms 3600000 --validation-visits 16777216 --resident-nodes 3145728 --collection-work 10000000000 --emit program
```

The two Cargo commands in the probe check metadata inventories before
execution; they neither produce C2 nor compile a guest language stage.
Compiler input/output profiles are (1,1), target and optimization are zero,
and cfg flags are empty. JOB1 also caps frames at 65,536, artifact bytes at
16,777,216, artifact nodes at 196,608 and depth at 4,096.

## Observed result

Every number below comes from the successful `run-artifact` command in the
receipt above, with those exact inputs and revisions.

| Measurement | Value |
|---|---:|
| charged reductions | 10,357,536,443 |
| worker elapsed microseconds | 1,471,089,704 |
| cumulative fresh allocations | 190,792,140 |
| pinned nodes | 177,106 |
| peak resident nodes | 3,145,728 |
| final resident nodes | 2,370,575 |
| reclaimed nodes | 188,421,565 |
| completed collections | 66 |
| collection work | 1,616,181,373 |
| collection scratch bytes | 29,360,128 |
| peak evaluator frames | 9,267 |
| C1 canonical artifact bytes | 9,706,396 |
| C2 canonical artifact bytes | 9,681,676 |

Joy reports `compiler_job.status=success`, no diagnostics and `trace_mode=none`.
C2 SHA256 is
`fe0390b92257edf58686e50571160fc7985b0883ede116c6dcfd0a9f7820b5d0`;
its particle is
`84710d31df5911098b614ee0455d7a199bfe5349e57a0f97e031d52dd533eeb3`.
C1 and C2 differ, which is allowed because the Rust seed has different
generation choices. C2/C3 equality remains a separate measurement.
The elapsed time is worker wall time, not a portable performance guarantee.
Resident nodes are an enforced bound; they are not a process-RSS measurement.

## Retained bytes and regressions

[Artifact hashes](artifacts.json) bind the original C1, executed JOB1 and
published C2 to deterministic gzip archives. Decompression was checked
byte-for-byte against the originals. [sources.tar.gz](sources.tar.gz) retains
all original source files and manifest, with only tar metadata normalized.
The archived manifest refers to its sibling numbered `.tri` files. Historical
receipt paths remain untouched; new runs must write fresh output paths.

The [full Rust gate](gates.json) used
`CARGO_TARGET_DIR=../target-root cargo test --release --locked --offline -- --test-threads=4`
on the same complete source: 1,195 passed, five ignored, zero failed and zero
Rust warnings. [Raw output](full-tests.log.gz) is preserved without text
normalization. The ignores are separately invoked footprint and compiler-scale ordering
diagnostics; their measurements remain in the owning byte-discard, function-sort,
alias and export-row receipts. A wording-only coverage-checker test failure
was corrected before commit; its raw output is retained in
`coverage-status-wording-failure.log.gz`.
The [feature coverage map](../compiler-feature-coverage.md) binds all used
constructs to positive/rejection evidence and this completed Rust gate.

Earlier [10-billion-budget failure](../full-bootstrap-compacting/README.md)
and [body-stage localization](../../../../nox/audit/prefix-frontier/README.md)
remain historical evidence. The source and runtime changed between those
attempts and this success; this is not an isolated speedup comparison.
Native compilation proofs remain SH7/SH8 work.
