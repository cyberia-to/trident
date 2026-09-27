# Local S1 first build and source/package checks

The full compiler source at `77213171d39b88c5f41221912251cc4813ac2b11`
(94 modules, 370,544 bytes) compiled itself successfully. C1 emitted actual C2,
SHA256 `76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`,
9,691,488 bytes, particle
`2eea2ac5f611012877b4e7291a3a6f534aee7281bb358a0b8e5fabe2ac1f9fbe`.
The launch preceded the source commit: `launch.json` preserves the actual
base revision and dirty source hashes. `validation.json` independently verifies
all 94 captured source files against that later commit, their producer copies,
and the exact package manifest. No launch history has been relabeled.

`c1-to-c2.json` retains every actual command, working directory, exit code,
stdout and stderr. Its final `joy run-artifact --compiler-job` used C1 SHA256
`5728e07a37e111f88166c471ad338f8947b74dfdc842ed1ffa9fa0886c005fda` and
pinned Joy SHA256
`506f665b0567cf8d7d669f152153b72dbbbd4520e926a4f47955d2f0bef487b8`.
Measured work was 10,378,203,737 charged reductions, 190,817,237 cumulative
allocations, 66 collections and 9,259 peak frames. The explicit host profile
was 20 billion reductions, one billion cumulative allocations, 3,145,728
resident nodes, 10 billion collection work, 65,536 frames, 3,600,000 ms and
16,777,216 validation visits. Existing ordinary/default limits were unchanged.
`artifacts.json` binds the retained C1, C2, JOB1, inventory, launch and source
archive; the collector preserves those files byte for byte.

The actual emitted C2 passed all six source-size cases in
`source-scale/receipt.json`, produced by the exact invocation recorded there
of `audit/self-hosting/source-scale-compacting/check.py`. Every command uses
the supplied C2 and the pinned runtime. The same explicit compiler profile
applies. The successful emitted ordinary programs execute to 13.

| Case | Charged reductions | Peak frames | Result |
| --- | ---: | ---: | --- |
| 35-byte baseline | 366,863 | 403 | program |
| 4,096-byte comment | 22,156,845 | 2,380 | same program bytes |
| 65,536-byte comment | 488,109,499 | 4,324 | same program bytes |
| Binding error after comment | 23,860,286 | 2,388 | code 5, bytes 4131–4138 |
| Invalid encoding at 65,536 bytes | 6,147,232 | 1,619 | code 1, bytes 0–1 |
| Excess 65,537 bytes | 100,737 | 312 | code 7, bytes 0–0 |

`source-scale/artifacts.tar.gz` retains the entire generated tree: 31 exact
input, manifest, JOB1, result and output files. `artifacts.json` in that
subdirectory records every file hash. Raw command output remains in the
unaltered receipt. These cases establish the bounded comment/trivia path;
they do not establish every possible maximum-length token workload.

`package-invariance/receipt.json` records the actual invocation of
`audit/self-hosting/full-package-determinism/check.py`. Relocating all source
files and reversing manifest order reproduced the exact original JOB1 bytes.
Changing one dependency byte changed the package/job identities while
preserving source length and limits. The retained 97-file input archive
contains the 94 relocated sources, the modified dependency and both manifests.
This check repacks transport inputs and binds the existing successful build;
it does not claim a second full compiler execution.

`install/receipt.json` and its exact compressed logs retain the separate local
candidate installation at Trident `4bbe399c85a5c5ef50e40791cc5f8da79e929071`
and Joy `2878f4b17dfedf237c6110d7d411bb4824e65103`. Installed Joy reproduced
C1 byte for byte. The initial missing relative `lens` dependency failure is
retained alongside the successful installs. Cargo PATH reminders are preserved;
there were no Rust compiler warnings. This installation includes unrelated dirty
`lens` files, with post-build dependency status/diffs retained. It is a local
installation, not a clean release input, and it was not used for the retained
C1(S) or source-size runs: those pin the earlier immutable Joy binary.

Reproduce the archival checks from the Trident repository with:

```sh
python3 audit/self-hosting/lexer-bootstrap/collect.py
```

The collector verifies existing hashes and copies measurement files; it performs
no language compilation. Exact original execution argv/cwd and outputs are in
the build, source-size, package-invariance and install receipts. The two retained
`check.py` copies preserve measurement-tool provenance; replay uses their original
repository paths so their shared helper imports resolve normally.

At this evidence cutoff, S1 C2 semantic corpus acceptance and C2→C3/fixed-point
validation are ongoing. This directory claims the completed local first build
and SH4 source/package checks only. Clean bootstrap and platform acceptance
remain separate.

Subsequent evidence retains the [completed actual C2 semantic corpus](c2-corpus/README.md),
the [exact C2/C3 fixed point](fixed-point/README.md), and the
[separate example build check](example-build/README.md). The original first-build
and source-scale receipts above remain unchanged.

The [actual C3 corpus](c3-corpus/README.md) subsequently passed as well.
The [completed local continuation](orchestration/README.md) retains all stage
commands and final exit statuses; clean native-platform repetition remains open.
