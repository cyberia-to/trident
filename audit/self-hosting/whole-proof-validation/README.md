# Complete self-build certificate validation tools

Status: prepared and reviewed. This delivery records the validation tools and
their existing-pilot checks. Complete C1/C2 self-build proof acceptance remains
open; no complete self-build certificate has been consumed by these tools yet.

The source inputs are the unchanged accepted SH6 compiler package from Trident
`77213171d39b88c5f41221912251cc4813ac2b11`. The tested production Joy source is
`6e0ec4d8440e2521df08f442d64f54e667044716`, integrated into `release/0.4` by
PR26. The command receipts bind its installed executable SHA256
`8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9`.

## Recorded checks

The exact commands, source identities, native Rust version and raw output are
retained with each receipt:

- [Alternate request preparation](tools/preparation.json): production `pack-job`
  admits five changed complete requests for each accepted compiler. Each request
  retains the complete 94-module, 370544-byte source graph; the changes cover the
  compiler, entry source, dependency source, configuration and JOB reduction limit.
  [Input identities](prepared-inputs.json) record the external files, which remain
  in the named isolated measurement directory.
- [Corrected helper build](tools/helper-build-2/receipt.json): Rust 1.89 release
  tests and build pass without warnings. The five helper tests cover framing and
  canonical output reconstruction. The original build and source are preserved;
  [review](tools/root-helper-review.json) found unchecked local index arithmetic,
  and [validation](tools/root-helper-review-validation.json) closes that finding.
- [Existing-pilot integration](tools/fixture-check/receipt.json):
  `python3 fixture_check.py` checks the actual SH7 aggregate certificate, including
  a byte-identical rebuilt-chain control, thirteen rejected mutations, and a
  separate malformed-index overflow rejection. These are pilot results, not full
  self-build results. The receipt binds the actual helper and certificate bytes.
- [Independent source review](independent-receipt.json): the complete-generation
  runner admits only a successful producer followed by actual fresh verification.
  The review caught a collision between generated output sidecars; unique names
  fix it, while all sidecars and failed attempts remain retained.

## Full-proof execution contract

[The plan](tools/plan.md) and [runner](tools/whole_suite.py) require two controls
and 23 rejected changes for each complete certificate. The original control
explicitly reuses the root runner's actual fresh verification; the rebuilt-chain
control invokes the production verifier again and requires byte-identical
certificate and extracted C2/C3 program bytes. Every negative invokes the actual
production verifier, requires the intended error, empty successful stdout, and
preservation of a pre-existing output file.

A global lock permits one complete temporary mutation across both generations.
The original certificate is read-only. Each temporary is removed only after its
recipe, identity, command output and classified verification result are retained;
an unexpected result preserves the payload and stops the suite. The bounded
canonical output sidecars remain separate. The complete-proof resource profile
and original compiler limits remain fixed before execution.

These are exact archived measurement drivers. Their relative paths resolve in
the original isolated `selfhost-0.4-finalization-20261002` family recorded by
[retained-files.json](retained-files.json); this directory is an evidence copy,
not a second running suite. The executable and large inputs remain identified
external files. Source copies, logs, successful checks and initial failures are
retained without rewriting their chronology.
