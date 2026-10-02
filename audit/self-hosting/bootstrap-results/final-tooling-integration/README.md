# Combined self-hosting acceptance tooling

Integration commit `dc6700ce3552e9f5e0f27d81600162f62b26382c`, tree
`27228fb1b10997fecffc34c037ab63db42b08a29`, combines the reviewed SH7
retention, complete-proof validation and corpus metadata Git branches. Its
parents are `87e2909f6ab2dfc01a97add0fe0402cbd9cd838c`,
`01e45e55ef9f68eaeccca270dd0bb00bc58a4bba` and
`2f1a575a6fbc0704c268f1ae21667830a3c996df`.

The invoked generated-compiler corpus can select an absolute Git executable
for source metadata while guest commands retain an empty PATH. Compiler
sources, corpus cases, native workflow definitions and computational limits
retain their reviewed bytes. Hash-bound corpus evidence also survives Git
checkout with Windows line-ending conversion enabled.

[Combined checks](originals/precommit/receipt.json) record the actual commands
against that tree: seven metadata tests, eighteen bootstrap phase tests,
twenty-eight bootstrap runner tests, sixteen corpus delivery tests, delivery
replays, actionlint and the whitespace check passed. The separately retained
[compiler routing command](originals/compiler-routing-check/receipt.json)
passed fourteen tests against the committed integration revision.

[Rust validation](originals/rust-source-and-check/receipt.json) distinguishes
two kinds of evidence. The complete prior workspace result, 1231 passed,
zero failed and five ignored, is reused only after comparing the protected
source tree with the original tested revision. The pinned Rust 1.89.0
`cargo check --release --workspace --locked --offline` was executed afresh
and passed without warnings. This delivery makes no fresh Rust test-count
claim. [Postcommit installation](originals/postinstall-merge/receipt.json)
uses a new installation prefix and the recorded warm compilation cache;
both installed binaries and actual compiler identities are retained there.

[Independent review](independent-review/review.json) replays the merged source
inventory, command receipts, focused tests and real Git checkout filters.
It also retains the initial review attempt stopped when the staged tree was
committed during inspection; the final review binds the resulting commit and
its unchanged tree explicitly.

[files.json](files.json) binds every original receipt and raw command stream
to its copied bytes. Streams are stored with deterministic gzip compression;
the manifest includes both stored and decoded identities. The original
successful corpus and earlier failed attempts remain in their existing audit
directories unchanged.

This is a tooling integration result. The complete native matrix must run on
the final PR revision. SH8 remains open until the full adversarial checks,
final acceptance replay and durable complete-certificate retention pass.
Later completed evidence will be delivered in a separate PR after this
tooling PR is integrated into `release/0.4`.

The subsequent complete-certificate diagnostic run stopped at
`rebound-job-limit`. The [original failure observation](negative-v2-stop/observation.json)
records the exact helper revision, verifier command receipt, actual rejection
and preserved temporary identity. The constructor reused the original reduction
budget instead of the admitted alternate JOB1 limit, causing transport rejection
before the intended semantic check. The coordinator stopped both suites and the
final acceptance watcher refused to launch. Their original failed receipts remain
failed; this delivery does not count the unfinished case as accepted.
