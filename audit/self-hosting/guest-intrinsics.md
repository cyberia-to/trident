# Declared intrinsics through the native compiler

Source `be9676d4848dea5464ad7381da9aa65be1f4f8ce`; Joy `a15adb7310db5b159b4e72ce0dfd0fea2ff01c7d` and Trisha `f5c94f5ec5d6d52943c8f416f343513b9d33c85e`.
The [validation receipt](guest-intrinsics-validation.json) pins commands, source hashes,
sibling revisions, output identities and complete gate logs. These are local macOS
ARM64 development results. Acceptance began before the source commit; the frozen
source map matches that commit, and its clean reinstall reproduces all three binaries
and the compiler/graph artifacts. It uses the same build cache; fresh-build codec
reproducibility belongs to the accepted prerequisite.

C1 admits exact bodyless intrinsic declarations, checks namespaces and complete ABIs,
and resolves identity from the final declaration ID after name/import resolution.
Direct aliases retain visibility and source-member purity checks. Ordinary wrappers
and replacements keep their bodies. Twelve existing builtin operations lower; five
additional known ABI identities remain unavailable in reachable ordinary bodies and
report code 6 at the original call/module. Private and replaced declarations are checked.
The [source review](guest-intrinsics-review.md) records the implementation invariants.

| Installed corpus | Commands | Observations |
| --- | ---: | ---: |
| [full](guest-intrinsics-full-cli.json) | 1199 | 402 |
| [profile](guest-intrinsics-profile-cli.json) | 65 | 21 |
| [types](guest-intrinsics-types-cli.json) | 111 | 24 |
| [callable](guest-intrinsics-callable-cli.json) | 149 | 32 |
| [constants](guest-intrinsics-constants-cli.json) | 141 | 31 |
| [graph](guest-intrinsics-graph-cli.json) | 39 | 12 |
| [intrinsic](guest-intrinsics-intrinsic-cli.json) | 158 | 37 |

The full corpus retains all 236 previously successful observations and 203 distinct
ART1 identities, with unchanged input sources and public quotas. Exact reduction, arena and
evaluator-frame boundary probes recalibrate to measured costs and still reject one below.
The ordinary-program intrinsic fixture now reports namespace error 5 instead of the
former unsupported-syntax error 6. Its complete old/new diagnostics are retained.
The original 65-bit record-write source now also completes and returns 3199 under the
same 786432-node limit; its earlier failure remains in the preceding receipt.

Indexed reads traverse two levels per continuation and finish directly at heights 1–3.
Packed-byte reads select exact constant lanes. Collection encoding, padding, validation
charges and public resource ceilings stay fixed. All 256 byte values are checked in
all four lanes; indexed reads retain opaque pair leaves through every U32 tree height.
The unchanged long-record CLI source uses 742497 nodes /
68586501 reductions / 2643 frames;
wide65 uses 764327 / 35319883 /
1326. Each comes from its named full-corpus command.
C1 SHA256 `1acb0acf2950528c366db85e30e33ac0cc4450ba7d538d517664492bd153be1c` has 95315 DAG entries.

All seven owner gates pass: 1197 Trident, 123 Joy and 380 Trisha tests; zero Rust
warnings and four existing ignored Trisha tests. All 133 baseline rows and 43 manual
baselines match the preceding delivery. All 119 formal verdicts are UNKNOWN.

Failed runs remain in the receipt: the first census lacked executed coverage for two
new public functions; the next full run exposed four old 196608-node regressions;
a later run caught the stale expectation that wide65 must exhaust its arena.
Coverage and implementation were corrected, then the full owner gates reran.
A CLI invocation accidentally used the harness default 30000ms and cancelled on the
long record; the complete corpus was repeated at the original 60000ms. Intermediate
[resource experiments](guest-intrinsics-resource-experiments.json) retain the local
source fingerprints, failed quotas and separate generous diagnostic measurements.

The parser-owned closure contains 94 modules, 455 functions and 345639 source bytes.
The recorded full-closure JOB1 uses the explicit 3145728-node/100M-reduction tier and
returns capacity error 7 at `std.compiler.nox.job`; it produces no C2. Full source
admission, compiler-scale runtime, C2/C3 fixed point, six CPU platforms and native
Zheng proofs remain open. Noun remains 128K.
