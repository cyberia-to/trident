# Full native compiler frontier before compaction

Local development evidence on macOS ARM64. Full self-compilation remains open
at this frontier: no successful C2 is claimed here.

[Identities and revisions](full-bootstrap-frontier/identities.json) bind each
source variant, compiler and diagnostic prefix. Frozen source patches against
Trident `ca9d926` preserve the intermediate inputs. Apply with
`git apply --unidiff-zero`; both [patches reproduce all recorded source hashes](full-bootstrap-frontier/patch-reproduction.json).
The final `scaled` source
is committed as `713f457`; its unchanged closure contains 94 modules, 486
functions and 369707 bytes. `selfhost_inventory --check` is recorded in each
full-closure receipt. Nox is `13b4c2e`; Joy's cached runner is `4e814a9`.
Reports include original local paths; measured diagnostic artifacts remain in
that worktree. The packed prefix predates the current diagnostic fixture.

## Measured barriers

| Exact input and command receipt | Successful gas | Allocated nodes | Peak frames | Result |
|---|---:|---:|---:|---|
| [packed, discovery only](full-bootstrap-frontier/prefix-discovery-large.json) | 684646281 | 15458181 | 3259 | all 94 modules discovered |
| [packed, all bodies](full-bootstrap-frontier/prefix-bodies.json) | — | 25165824 | 3591 | resident arena exhausted |
| [scaled, discovery only](full-bootstrap-frontier/scaled-discovery.json) | 627648529 | 14596780 | 3245 | all 94 modules discovered |

These are direct nox component measurements with explicit larger limits, not
Joy worker acceptance. The packed and scaled sources differ; this table does
not establish an isolated speedup. Failed gas is unavailable: evaluator
checkpoints and a propagated child Halt budget are not root charged reductions.
The discovery receipts' `prefix_words` are `[2,94,0,0,0]`, not a compiler artifact.

`selfhost_runtime_probe` takes complete ART1/JOB1, a selected diagnostic stage
and explicit gas/node/time ceilings. The latter receipts record its exact argv.
The earlier packed discovery command uses the same packed ART1 and JOB1 as the
all-bodies command, with `--stop-stage 2 --budget 1000000000 --nodes 25165824
--seconds 1200 --cached`. The source body remains unchanged.

The [baseline](full-bootstrap-frontier/baseline-closure.json),
[packed](full-bootstrap-frontier/packed-closure.json) and
[scaled](full-bootstrap-frontier/scaled-closure.json) full-compiler attempts
preserve their host/runtime rejections. The original one-million guest
validation allowance is insufficient for the packed source: decoding plus
module lookup already exceeds it. An explicit 16777216-visit run passes that
boundary; it still reaches the append-only arena barrier. No old fixture quota
was increased to hide a regression.

The exact all-body frontier was independently inspected by nox's isolated
liveness census: see `nox/audit/compiler-liveness/` in the sibling worktree.
That census motivates bounded compaction; it is not a whole-run live-memory
bound or a completed self-build.

## Reproduction and regressions

An installed Joy embeds its standard library at build time. An early attempted
rebuild with the old installed binary reproduced the old compiler; it was not
used as an optimized measurement. `selfhost_seed_snapshot` instead requires a
complete explicit frozen module map and rejects missing imports before seed
compilation. The [guard and reproduction receipt](full-bootstrap-frontier/frozen-seed-guard.json)
reproduces packed C1 byte-for-byte, with SHA256
`e2dcc0e2b844bb667d30f03e0a4536069d8093cb5ff851a12fa8590f472bb61f`.
This helper is a host seed tool, never a guest compilation stage.
It uses the resolver's public canonical-owner function to reject alias names
and rejects the three generated namespaces that override supplied sources.
[Final helper guards](full-bootstrap-frontier/final-helper-guards.json) cover
those substitutions, distinct diagnostic result/receipt paths, preserved
existing output and the same frozen C1 reproduction. Both helper writers use
exclusive creation. The existing canonical-owner unit test passes after the
function is exposed without changing resolution behavior.
The closure probe also reconstructs the exact frozen source tree and runs the
existing inventory checker before packing. This binds each snapshot back to
the inventory's BLAKE3 and declaration/import rows. The
[one-reduction negative run](full-bootstrap-frontier/snapshot-guard-closure.json)
passes that guard and admission, then preserves the expected budget rejection.

The [rejected lexer patch](full-bootstrap-frontier/lexer-word-cache-rejected.patch)
reduced repeated packed reads but regressed small-program allocation limits;
it was removed. Stable function ordering, packed path comparison and discarded
validation bindings restore the original limits. Their isolated evidence is
in `function-sort-scale.md`, `job-word.md` and `byte-discard.md`.

The complete Trident gate on these frozen production sources passed 1190 tests
with two existing ignores and zero warnings. Exact command, revision and raw
output are preserved in `job-word.json` and `job-word-full-gate.log`.
`native_compiler/support.rs` uses the same cached evaluator as Joy; the quotas
are unchanged and separate traced/uncached oracles remain available.

Next acceptance is full C1(S) → C2 through explicitly bounded compacting Joy,
followed by independent programs through the supplied C2 artifact and C2(S) →
C3. Canonical C2/C3 equality and repeat execution are still required. No trace
or compilation-proof milestone follows from these run-only measurements.
