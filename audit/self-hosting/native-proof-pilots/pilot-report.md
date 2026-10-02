# Actual complete C2 proof pilots

The five bounded pilots passed with the complete accepted C2 compiler. Fresh
verifier processes accepted each certificate against explicit expected compiler
and JOB1 inputs. Three byte-identical transport controls passed; twenty-four
altered certificates or bindings were rejected. Full SH8 self-build proofs are
separate work. Acceptance of this SH7 evidence remains subject to independent
review.

All proof, fresh verification and generated-program execution measurements use
Joy `6e0ec4d8440e2521df08f442d64f54e667044716`, installed with Rust 1.89.0, binary
SHA256 `8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9`.
The complete build closure and toolchain are in `production-source/inputs.json`
and `production-source/receipt.json`. The accepted C2 artifact is 9,691,488 bytes,
SHA256 `76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
Its frozen source provenance includes 94 modules and 370,544 source bytes.

Each row below names the attempt whose `receipt.json` records the exact command,
arguments, binary and input identities, search PATH, resource bounds and sampled
host measurements. `stdout` is the unmodified Joy report. Time is outer wall time
from that attempt. RSS is sampled process-tree KiB and remains a host observation.

| Prove attempt under `attempts/` | Charged reductions | Expanded steps | Certificate bytes | Prove seconds | Fresh verify seconds | Prove / verify sampled RSS KiB |
|---|---:|---:|---:|---:|---:|---:|
| `prove-loop-1` | 1,339,805 | 1,055,826 | 2,077,545 | 2.317 | 1.813 | 275,792 / 201,728 |
| `prove-aggregate-1` | 6,034,212 | 5,050,100 | 8,662,132 | 5.074 | 2.655 | 285,808 / 231,440 |
| `prove-compile-error-1` | 915,380 | 720,262 | 1,707,931 | 2.187 | 1.756 | 275,712 / 203,280 |
| `prove-scale-baseline-1` | 366,863 | 310,258 | 957,305 | 1.811 | 1.587 | 202,896 / 202,896 |
| `prove-scale-valid65536-1` | 488,109,499 | 437,670,002 | 417,642,285 | 255.282 | 71.322 | 828,624 / 561,184 |

The corresponding fresh result verifiers are `verify-<case>-result-1`; successful
cases also have `verify-<case>-program-1` and `execute-<case>-1`. Commands use an
absolute installed Joy executable and an empty search directory as PATH. The
verifier checks the public semantic certificate without executing the compiler.
Its report contains no prover host observations. Complete semantic roots, cost,
depth, expanded steps, record/transport counts and compiler response match the
prover report. Final machine-readable comparisons and hashes are in
`summary.json` and each `*-comparison.json`.

The loop source is the accepted SH3 arithmetic/loop fixture, with output 234.
The aggregate case is the accepted two-module imports/functions/branch/struct
fixture, with output 79. Exact accepted source strings and their fixture revision
are recorded in `inputs.json`. These two compiled outputs equal the raw executed
output bytes of separately built Rust references. The Rust references adapt the
entry ABI; their ART1 bytes differ, as recorded explicitly in the comparisons.
The three-module compile-error case proves a `compile_error` RES1 with diagnostic
code 5, module index 2, bytes 32..39, `invalid binding`. Fresh result verification
accepts that response; requesting a program refuses and preserves the existing
destination.

The SH4 cases copy the exact previously accepted 35-byte and 65,536-byte sources
and use the same complete C2. Their extracted ART1 bytes equal each other and
the accepted historical artifact; the executed result is 13 with identical
historical output bytes. The long run matches historical charge, depth 4,324 and
437,670,002 expanded steps. Its host observer reports 3 resets (initial snapshot
plus 2 successful collections), 301,983 snapshot nodes and 8,468,935 fresh nodes.
Those labels are distinct from cumulative allocations. These are measured in
`attempts/prove-scale-valid65536-1/stdout`; historical commands and quantities are
retained in `scale-historical-receipt.json`.

The baseline profile bounds guest charge/allocations at 100 million, frames at
65,536, resident nouns at 3,145,728, collection work at 1 billion and host time at
300 seconds; proof caps are 1 GiB wire, 2 GiB decoded, 100 million records,
200 million observed steps and 65,536 cache slots. Its outer guards are 330
seconds, 3 GiB sampled RSS and 2 GiB attempt disk. The separately declared SH4
profile uses 600 million charge, 20 million allocations, 200 million collection
work, 900 seconds host time, 4 GiB wire, 8 GiB decoded, 500 million records,
600 million steps and the same cache/resident/frame bounds. Outer SH4 guards are
930 seconds, 3 GiB sampled RSS and 8 GiB attempt disk. The exact guest source
limits differ from the historical JOB1; source bytes and compiler are unchanged.
These profiles are recorded in raw commands and `scale-inputs.json`.

`adversarial-results.json` retains twenty rejects: changed expected source,
dependency, option, JOB limit and compiler; the same five changes with completely
rebuilt matching transport contexts; continuation and cache generation changes;
terminal cost; damaged output payload and topology; omitted semantic terminal;
dropped/swapped frames; trailing bytes; and truncation. Every rejected case has a
fresh installed verifier command, empty successful stdout and unchanged protected
destination. The two initial controls reconstruct byte-identical certificates.

`adversarial-valid-output-results.json` adds a byte-identical control and four
stronger coordinated attacks. A separate Rust helper constructs altered payload
and topology nouns, re-encodes all identities, then fresh-decodes and canonically
re-encodes to identical bytes. With the checked terminal particle unchanged,
fresh verification passes noun decoding and rejects `certificate result identity
mismatch`. Rebinding the semantic terminal to each new valid root rejects
`semantic terminal: Claim`. The helper calls noun construction and codecs only;
it never evaluates a formula. Sources, Rust 1.89 test/build commands and logs are
under `valid-output-mutator/`. The earlier helper and all prior attempts remain
unchanged. Helper locks are retained separately from the installed production
CLI lock; acceptance always uses the exact installed production binary.

One original positive-control harness attempt is retained as
`attempts/attack-control-copy`: the verifier succeeded and replaced its intended
output, but the generic wrapper wrongly classified that preexisting destination
as immutable input. No compiler, JOB or proof changed. Positive controls were
rerun with fresh destinations. `driver-failures.json` records this correction.
The distinct compiler negative uses historical seed SHA256
`4b8276068704063eb13e5555bca872372d2975b4ac71cf93edc03273222327b6`; it is not the
accepted SH6 C1.

The profile is `joy-nox-disclosed-compiler-v1`, a complete public semantic witness.
It carries no CCS, succinctness or zero-knowledge claim. Physical resource use is
unattested. The initial preparation receipts retain their original chronological
pending wording; `summary.json` records the completed pilot results.

`manifest.json` hashes the raw evidence. Small files are copied with original
relative names; large proofs and compiler artifacts remain at the explicit
external paths in that manifest, with exact sizes and SHA256 values. The complete
experiment directory is preserved. `package_evidence.py` records the assembly
and rechecks every positive proof identity and every expected rejection. The
first package snapshot is also retained outside this package.
