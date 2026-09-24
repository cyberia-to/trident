# Native compiler: bounded heap arenas

[Pinned validation](sh4-native-arena-validation.json), [installed arena receipt](native-arena-cli.json), [regression receipt](native-arena-regression-cli.json).

Source: Trident `1e08dedbb0ec7b2dd25fc54ecc16c0587f8fc1ab`, Joy `820041bed4a7d649721a2d25b7ac92d162860854`, nox `c9f7486a74fe81bfc194b598da40f6343ecb2ef1`. Trisha and all remaining sibling pins are in the validation JSON. All results are local development evidence on macOS ARM64.

The installed acceptance command is:

```sh
python3 audit/self-hosting/run-native-arena.py \
  --joy ../install/bin/joy --output /tmp/native-arena-cli.json
```

C1 is built once before case sources exist. Joy packs each exact source into JOB1,
executes C1 inside nox, extracts returned ART1 and executes it independently.
C1 particle stays `a2d472f1c8bc2c2016c613b36765d7d3f533c9f1053d704ef4ff269f3e270b30`. The runner records 41 commands
and 10 observations. The 31-assignment program returns 31; 64 nested identity calls
return 1. Both exhausted the previous default allowance.

| Workload | Charged reductions | Allocated nodes | Peak frames |
|---|---:|---:|---:|
| assignments31 | 15238660 | 451540 | 1275 |
| calls64 | 15012164 | 497992 | 1322 |
| source4096-invalid | 8836242 | 534594 | 1412 |

The same arithmetic JOB on both physical tiers preserves exact JOB/ART1 bytes,
RES1 identity, reductions, allocated nodes and peak frames. Changing LIM1's
allowance changes JOB1/RES1 identities while retaining the emitted program.
The small and large arenas reserve 26214416 and 104857616 bytes respectively,
reported from `size_of::<Reduction<N>>()`; these are reserved representations,
not measured RSS. Default logical allowance remains 196608; 786432 is explicit.

On the large tier, arithmetic succeeds at 71369 nodes and fails at 71368.
The 31-assignment job succeeds at 451540 and fails at 451539. All failed executions
and rejected program publications preserve the previous destination with force.

The 4096-byte invalid-UTF8 source now reaches guest diagnostic 1. A valid arithmetic
source padded to 4096 bytes still exhausts 786432 nodes. This is a retained open
SH4 boundary, not evidence that arbitrary 4096-byte sources fit. No failed-run
node/reduction count is invented. Full closure and compiler-sized allocation
remain open.

Workspace checks have zero Rust warnings. Trident 970, Joy 122 and Trisha 380 CPU
tests pass, with four existing Trisha ignores. Trisha 133/133 fixtures and 43/43
manual baselines pass; all result/cycle rows match the prior functions delivery.
The existing 242-command/84-observation installed corpus passes with every
previous positive ART1 identity retained. All 37 formal compiler audits remain
UNKNOWN; execution acceptance does not establish a proof.

No source-language coverage is added here. U32/loops, Noun/aggregates, constants,
attributes, imports, full closure, C2/C3, six-platform release and native Zheng
compiler proof gates remain open. Noun stays 128K.
