# Bind diagnostic headers to the admitted compiler job

The complete-proof `rebound-job-limit` constructor used the original reduction
budget, 20000000000, while its alternate JOB1 admitted 19999999999. Joy therefore
rejected the certificate header before reaching the intended semantic record.
The original failed v2 attempt and its unexpected diagnostic remain preserved in
[the integration evidence](../../bootstrap-results/final-tooling-integration/negative-v2-stop/observation.json).

[The corrected helper](originals/whole-binding-context-review/binding_context.py)
derives reductions and evaluator frames from the actual admitted JOB1. Compiler,
program, formula and job identities remain bound to the prepared canonical bytes.
It preserves the expected `semantic record: Key` rejection and all existing
verifier limits. Production Joy and the native mutation helper are unchanged.

[The actual bounded run](originals/whole-binding-context-review/actual/receipt.json)
uses the accepted SH7 aggregate certificate and production Joy source
`6e0ec4d8440e2521df08f442d64f54e667044716`, executable SHA256
`8f42591ece35f192ff6f2328a8360fe0f0959f48a173248b211cd0d8f4d984f9`.
Its exact command and source identities are in
[the invocation receipt](originals/whole-binding-context-review/actual-command.json).
The run records 58 native command receipts, including 29 fresh verifier commands:
two controls, all 23 original negative cases, two evaluator-frame cases and two
reproductions of the original incorrect header. Eight focused context tests pass.
[Independent review](originals/whole-binding-context-review/review.json) and
[root readback](originals/whole-proof-attacks-completion-v3/binding-fixture-acceptance.json)
replay the raw results. The separate outer telemetry wrapper's ResourceWarning
observation is retained; the successful child commands have no warnings.

[files.json](files.json) binds 309 retained source, receipt and raw stream files,
with complete decoded-byte readback. It identifies 22 temporary fixture
certificate payloads by size and SHA256 at their original measurement paths;
those payloads are external local files. Their complete construction recipes and
verification receipts are retained here. No durable payload-retention claim is
made for these temporary negative fixtures.

These measurements establish the bounded regression only. Full SH8 acceptance
requires the separately reviewed complete-proof continuation, final acceptance
check and durable retention of both original full certificates. Completed cases
from the failed v2 suite must be explicitly replayed with their original
provenance; the original failed suite never becomes a successful run.
