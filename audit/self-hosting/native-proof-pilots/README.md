# SH7 native compiler certificate acceptance

SH7's production public profile passes the actual accepted C2 pilot gate.
Joy proves complete compiler workloads through nox and Zheng, and a fresh
process independently verifies their semantic records. The exact original
dynamic-apply probe also succeeds through this route. SH8 separately requires
certificates for both complete frozen self-builds.

The profile is `joy-nox-disclosed-compiler-v1`. Its witness is public. It
authenticates the expected compiler, input, complete result, computed
continuations and semantic charge. Succinctness, zero knowledge and language
semantics preservation remain separate claims. Physical time, allocations,
collection work and RSS are host observations.

## Actual commands and results

All production commands use the clean Rust 1.89 installation of Joy
`6e0ec4d8440e2521df08f442d64f54e667044716`, binary SHA256
`8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9`.
The closure includes nox `2f09ca3c3f18ae470365310cca8db5208eda75c6` and Zheng
`0d7ba6d422d9b825f9685e903e55252f88ebecd9`. The complete source inventory,
tool versions and command records are retained in the evidence archive.

[The original pilot report](pilot-report.md) and
[machine-readable summary](pilot-summary.json) name each measured command and
its source revisions. Five complete-C2 proofs pass fresh result verification:
loop arithmetic, imported aggregates, a guest compile error, and the exact
accepted short and 64 KiB source fixtures. Successful compilations also pass
fresh ART1 extraction and generated-program execution. Loop and aggregate
outputs match separately built Rust references; both scale outputs match the
historical accepted artifact and output bytes.

The 64 KiB case performs two real collections. Its certificate is 417,642,285
bytes, production takes 255.282 seconds and fresh result verification takes
71.322 seconds. These outer times and sampled RSS are recorded under
`attempts/prove-scale-valid65536-1/receipt.json` and
`attempts/verify-scale-valid65536-result-1/receipt.json`. The pilot retains its own declared resource profile; the
historical source and compiler bytes are unchanged.

Three reconstructed byte-identical controls pass. Twenty-four altered
certificates or expected bindings reject in fresh processes with protected
destinations preserved. These cover compiler/source/dependency/options/JOB
limits, completely rebuilt transport contexts, continuation and cache
generation, cost, missing/reordered/truncated completion, and output changes.
Four stronger output cases regenerate complete valid canonical noun DAGs;
verification rejects both a wrong result under the original terminal and a
terminal rebound to that wrong result. Original payloads, generator sources,
commands and errors are retained.

The [historical dynamic probe](historical-dynamic/receipts/execution/receipt.json)
uses the exact formula and public input from the
[starting audit](../../self-hosting-2026-09-23/soft3-runtime-probes.json).
The fixture encoder uses Joy's existing parser and subject constructor with
the canonical nox codec. Installed `prove-artifact`, fresh `verify-artifact`
and `run-artifact` all pass with output atom 42 and charge 6. The certificate
is 406 bytes. Those commands and the successful fixture build are retained
alongside two fixture-authoring failures: missing standalone workspace
declaration, then an attempted call to a private transport-limit method.

## Independent review and retention

[Root byte/receipt review](root-review.json), executed by [retain.py](retain.py),
checks all 665 original manifest files, all 637 packaged file identities,
production binary continuity, immutable command inputs, raw logs, verified
fields, byte-identical controls and rejection output preservation. It also
re-reads every member of the complete archive and verifies its identity.
[A separate read-only review](independent-review.json) independently checks all
original, packaged and archived file identities. Source review covered the
capture/derivation implementation, pilot runner,
mutation helpers and valid-output reconstruction; component reviews remain
in the owning Joy/nox/Zheng audits.

[pilot-receipts.tar.gz](pilot-receipts.tar.gz) preserves the original small
package exactly, SHA256
`325c98956a448bfd29115f2760794897e2457783cb1312b0561db58882fb249b`.
Its chronological preparation and review-pending wording remains unchanged;
this acceptance records the later review. The original successful verifier
misclassified by a positive-control wrapper also remains in that package.

The complete archive additionally includes every large indexed proof,
compiler and helper, plus the exact macOS ARM64 production Joy binary:

```text
sh7-native-c2-complete-evidence-20261002.tar.gz
602191451 bytes
sha256 fa0f99785b4f499067683949a4126a1e4837558fbd421c0c4d61fedc39000d61
```

[archive-files.json](archive-files.json) names all 670 archive files and their
hashes. Original relative paths resolve below `evidence/`, including paths
previously marked external in the small package. `tools/joy-macos-arm64`
is the measured production binary. The historical dynamic probe is separately
small enough to retain completely in this directory.

[Independent acceptance review](acceptance-review.json) checks the milestone
criteria, original small-package bytes, positive/adversarial reports and exact
historical dynamic probe. It reports no findings.

Remote archive retention is in progress through the existing unpublished
rehearsal draft. The [initial single-file transfer](initial-transport/receipt.json)
was stopped after slow upload made its declared transfer deadline impractical;
its failure remains recorded. Ordered chunk transport is separate follow-up.
Large payloads remain complete in the locally reviewed archive.
Final transport evidence must bind server asset digests and independently
downloaded archive bytes; tag creation and release promotion belong to the owner.
