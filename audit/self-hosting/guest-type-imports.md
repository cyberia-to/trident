# Guest nominal type imports through C1

Compiler source `7c1701c4c0f7a6f3862fc858ca80a794a999f96e`; acceptance harness
`a166c8de6fb39e992a8dbc2bd6fe0466e1629ad7`. The [validation receipt](guest-type-imports-validation.json)
records frozen source hashes, embedded compiler/harness snapshots, source ancestry,
all commands and sibling pins. This is local macOS ARM64 development evidence.

C1 resolves direct public nominal types and constructors using ordered full/short
aliases. Descriptors retain their defining owner, ordered layout and field visibility.
Private types may cross function boundaries as inferred opaque values. Dependency
parameters/results now preserve complete Noun, Digest, tuple, fixed Field-array and
nominal values. Qualified types/constructors use their own namespace; projections
retain lexical shadowing. Every private/replaced declaration consumes the shared cap.
The seed prerequisite gives each owner/name one resolved layout; repeated declarations
may change only struct export visibility. Forward type references remain rejected.

| Installed corpus | Commands | Observations |
| --- | ---: | ---: |
| [guest-type-imports-full-cli.json](guest-type-imports-full-cli.json) | 1198 | 402 |
| [guest-type-imports-types-cli.json](guest-type-imports-types-cli.json) | 111 | 24 |
| [guest-type-imports-callable-cli.json](guest-type-imports-callable-cli.json) | 149 | 32 |
| [guest-type-imports-constants-cli.json](guest-type-imports-constants-cli.json) | 141 | 31 |
| [guest-type-imports-graph-cli.json](guest-type-imports-graph-cli.json) | 39 | 12 |

The full corpus preserves all 235 H successful observations and 203 distinct ART1
particles: 227 exact source_hex values match; the remaining eight boundary observations
retain module/source identities and lengths. Package identities, entry, options and
all 233 non-recalibrated limit sets match. Two exact resource boundary tests use the
new measured reduction/arena cost and still fail one below; their old/new caps are
recorded separately. Complete JOB1 identities change because expected_compiler is now C1
`497d6a47860300831becf06e3afc7282f023f2de16e43fa310e26e7915882a3f`; both bindings are recorded. Same-layout duplicate
structs now execute, changed layouts reject, and six formerly unsupported import
observations now execute against the seed. All 12 graph results and exact allowances
are unchanged. CLI receipts preserve original source bytes and actual command output.
Harnesses compared complete output bytes while temporary files existed; the receipts
retain those executed-comparison flags and hashes, without claiming retained artifacts.

All seven owner gates completed: 1175 Trident,
122 Joy and 380 Trisha tests passed,
with zero warnings and four existing Trisha ignored tests. All 133 baseline rows
and 43 manual baselines match H. All 113 formal audits report UNKNOWN.
The first owner test run failed on one stale expectation: qualified constant-as-
constructor now reports invalid binding 5 instead of unsupported construct 6.
Its original frozen snapshot, failed gate/logs and exact one-line test correction
are retained; all seven final gates were rerun after the correction. Production
compiler code and installed CLI inputs stayed unchanged.
The first rebuild exposed randomized error-enum generation in upstream
`bfieldcodec_derive`. Trisha owns the checksum-pinned deterministic overlay; Joy
selects its existing locked version. The prerequisite receipt retains the failing
macro regression and verifies two fresh builds of all three binaries byte for byte.
Every installed CLI corpus above was rerun with the corrected warrior binaries:
source/artifact fields, execution metadata and exact resource costs match the
original run, excluding elapsed time. The Trident executable and formal-audit
sources are unchanged. All three installed binaries and C1/graph artifacts reproduce
after committed, clean installation. C1 SHA256 `abb8d805f0020779695c805c806ec8cd7c8e04f9454ef9a3ccaf1e76970f7dba` has
91942 entries.

The original 61–64-bit record-write sources return 3199 with the same 786432-node
allowance. Wide64 uses 773424 nodes, 36981916
reductions and 1346 frames. The 2158-byte long-record source returns
79 using 782938 nodes, 74160420 reductions and
2641 frames under that allowance. Wide65 still exhausts the arena
and preserves the old program. These costs come from the named full-corpus commands.

Imported intrinsics, legacy remaps, generated compiler profiles, SH4 scale, full
C2/C3, six CPU platforms and native Zheng proofs remain open; Noun stays 128K.
