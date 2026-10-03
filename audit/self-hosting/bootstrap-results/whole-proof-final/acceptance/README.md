# SH8: accepted complete native self-build proofs

Accepted on 2026-10-03 for frozen S1
`77213171d39b88c5f41221912251cc4813ac2b11`. The actual F5 checker passed both
complete self-builds, fresh independent verification, equal extracted C2/C3,
their regression corpora and the complete adversarial matrix. The
[public summary](packet/summary.json) projects original F5 receipt
`46e7171ede86ce006afcb850b610590aa1bbcbd7bcf5ee3a37ec3f59ca617d66`.
This closes the [SH8 contract](../../../../../reference/self-hosting.md#sh8-proved-self-compilation)
for `joy-nox-disclosed-compiler-v1`.

The original commands use Joy revision
`6e0ec4d8440e2521df08f442d64f54e667044716`, binary SHA256
`8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9`.
[commands.json](commands.json) retains the exact `joy prove-artifact` and
`joy verify-artifact` argument vectors, working directories, binary/profile
identities, measured fields and original receipt identities. It also records
the actual checker and collector commands. Each row below refers to its
corresponding producer and fresh verifier in that file; seconds are rounded
to three decimal places, with exact values in the summary.

| Actual self-build | Certificate bytes | Prove seconds | Verify seconds | Sampled prove / verify RSS bytes |
|---|---:|---:|---:|---:|
| C1(S1) → C2 | 11977015727 | 6678.023 | 1874.253 | 1146552320 / 1053179904 |
| C2(S1) → C3 | 10569174820 | 5955.159 | 1646.033 | 1089650688 / 1087750144 |

Both extracted compiler artifacts are 9691488 bytes with SHA256
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
Each passed the unchanged six-corpus suite: 547 observations and 1816 commands,
retained in the [proof-extracted compiler audit](../../../whole-proof-corpus-v2-result/README.md).
Fresh verification checks Zheng derivations independently of compiler or nox
execution. It authenticates the compiler, complete source/options package,
output, machine/profile and execution cost against the actual relation.

The profile discloses the complete witness and has linear verification work.
Private or succinct compiler proofs and source-language semantic preservation
remain separately specified claims. Sampled physical resources above are
observations; the certificate does not attest host resources.

## Adversarial acceptance

F5 accepted 23 distinct rejections and two original positive controls for each
generation. Each set includes nine V2 cases and three V3 cases. C1 adds eleven
fresh V5 cases; C2 adds ten fresh V5 cases and the independently authenticated
original V4 cost rejection. The [summary](packet/summary.json) names every case
and preserves the original failed V2/V3 and `input-changed` V4 statuses.

The completed V5 controller receipt has SHA256
`86e068b8c8b59567580be56fa1f82db4af8e72b18b5494f89c55f0503fc2a008`.
Its exact native commands, unchanged inputs, resource/process closure,
declared diagnostic retirements and historical failures were checked by F5.
The successful watcher receipt is
`e7cc4efc78edc333df6334fd910822c03ef5912cce0a09b2745573dee49aed41`.
Both original launch processes exited zero. No native proof or negative case
was regenerated during final collection.

## Retained evidence and delivery scope

The frozen collector ran once after the actual watcher passed. Its
[receipt](packet/receipt.json) binds 94 original public command streams in 33
distinct objects and a catalog of 16703 dependency identities. The exact
packet has 41 files and 7683029 bytes; the
[root copy validation](root-validation/copy.json) binds every file to its
original. Source/review identities and the unchanged collection limits are in
the receipt. The [root review](root-validation/review.json) checks the exact
file inventory, strict public-data rules and all retained originals.

The [independent actual review](independent-review/peer-review-v2.json)
separately checked the terminal evidence and exact packet against the originals.
Its initial schema/path comparison failures and first summary's command
transcription error remain in that directory; the corrected summary copies
the commands from the original review receipts. No native checks were rerun.
The [documentation validation](documentation-validation/receipt.json) records
the measured-status update and preserved contracts, versions and history.

Validate the retained public streams with:

```sh
python3 -B packet/verify-retained.py
```

Append `--originals` only where the recorded original paths are available.
The packet retains public metadata and command streams. Full private receipts,
host process tables and large proof/binary bodies remain at their original
locations, referenced by byte identity. This metadata validation is separate
from replaying a complete native proof.

Complete proof-body retention is independently accepted in Trisha
[PR25](https://github.com/cyberia-to/trisha/pull/25), with
[ordered-part and complete-byte readback evidence](https://github.com/cyberia-to/trisha/blob/fbea3cef9a4139075e529c319ff75488ed5df625/audit/whole-retention-byte-closure/README.md).
The [Joy source bridge](../joy-source-bridge/README.md) distinguishes the
original measured binary from the later packaged binary and records their
unchanged execution-relation sources. The
[native CI replay](../pr117-ci-collection/README.md) and
[coordinated distribution rehearsal](https://github.com/cyberia-to/trisha/blob/95899e8f4fe32b5d7269d92b5e63ef429fbfafac/audit/final-host-ceiling-package/README.md)
retain their own source revisions and acceptance scopes.

This delivery targets `release/0.4`. Tested package versions, default branches,
tags, registry publication and public release promotion retain their separate
owner-controlled policy.
