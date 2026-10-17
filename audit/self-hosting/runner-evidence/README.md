# Bootstrap orchestration preparation

This is a successful prepare-only measurement on macOS ARM64. It establishes
that the nine pinned origin repositories build the tools and reproduce the
frozen Rust seed and inventory. It does not establish a complete clean guest
bootstrap repetition or six-platform SH6 acceptance.

The exact invocation, working directory, source pins and tool versions are in
[manifest.json](manifest.json) and the compressed
[prepare receipt](prepare-receipt.json.gz). The receipt records 80 successful
commands. The fresh C1 is 9,706,396 bytes with SHA-256
`4b8276068704063eb13e5555bca872372d2975b4ac71cf93edc03273222327b6`.
The inventory is 356,739 bytes with SHA-256
`c121df82df67e1672e83cfc30b94a9161cde551b1be1890ff1e075c070ef2ea9`.
Both equal the frozen inputs of the successful whole-package C1 run at
Trident revision `0fe4054c7f9c9dc0f88e853a16289a3358ccb7ab`.

The measured helper was an uncommitted file outside the clean source clones;
its exact bytes are retained in [the source snapshot](bootstrap-runner.measured.py.gz).
Prepare-only deliberately permits this, because that pinned Trident revision
predates the new helper. A full run requires the executing helper to match its
copy in the pinned Trident checkout. Production compiler and runtime sources
were clean pinned origin checkouts. The manifest also records successful
post-build HEAD and clean-status checks for all nine source repositories.

[The command archive](prepare-commands.tar.gz) preserves all 160 stdout/stderr
streams without normalization. The
[member manifest](prepare-command-members.json.gz) records every raw stream's
size and SHA-256. The top-level manifest records compressed and decompressed
SHA-256 values. Gzip timestamps and tar member timestamps are zero.
[The inventory](prepare-inventory.json.gz) is retained byte for byte. The C1
file remains in the original measurement directory; its identity is retained
here and it can be regenerated with the recorded invocation and pins.

Subsequent helper changes add launch-failure receipt status, require the exact
`--artifact-profile raw` pair for labeled Rust reference oracles, and check all
source checkouts again before returning prepare-only success. They also enforce
distinct repetition directories, bind checker tools and exact resource flags,
hash every retained raw evidence file, and keep Python corpus assertions active.
The raw measured receipt remains unchanged. Final helper/workflow/test identities
and focused validation commands are recorded separately in
[final-validation.json](final-validation.json).
[The exact helper delta](measured-to-final.patch.gz) is retained without
normalizing patch bytes; its compressed and decompressed identities are in the
manifest.

[The real-input binding check](check-real-bindings.py) accepts the completed
C1 → C2 → C3 receipts and the six actual C2 corpus receipts. Its
[result](real-bindings.json) records all input identities and explicitly leaves
SH6 unaccepted. It executes the new runner's binding functions without executing
guest stages, fabricating repetitions, or substituting C2 tests for C3 tests.

To inspect the exact source and command evidence:

```sh
gzip -dc bootstrap-runner.measured.py.gz > /tmp/bootstrap-runner-measured.py
gzip -dc prepare-receipt.json.gz
tar -tzf prepare-commands.tar.gz
```

To repeat the preparation, set `BOOTSTRAP_PINS` to the receipt's complete `pins`
object and run the helper with fresh `--work` and `--output` directories,
`--target aarch64-apple-darwin --rust-version 1.95.0 --prepare-only`.
The runner uses a fresh Cargo home and separate build targets. The full mode
performs two clean source checkouts/builds, C1(S) → C2, C2(S) → C3 and all six
provided-compiler corpora for both C2 and C3 in each repetition.
It retains at most 50,000 files and 4 GiB of evidence. The final file manifest
binds all raw source snapshots, jobs, outputs and command logs; the aggregate
rejects missing, added or changed files.

The [new workflow](../../../.github/workflows/selfhost-bootstrap.yml) uses
the exact PR head and explicitly pinned siblings. It preserves tracked bytes
before checkout, requires native OS/architecture and Rust host/version,
uploads failure evidence, and compares retained compiler bytes across all six
platforms. Its runner labels follow the
[GitHub hosted-runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
Any missing, failed or incomplete platform is a failing aggregate gate.
The preparation reported here does not execute that workflow.
