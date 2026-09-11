# Warrior migration validation — 2026-09-11

Status: **unreleased development candidates; the requested fully working proof release is not complete**.
Trident 0.3.0, Trisha 0.2.0, Joy 0.4.0. All changes are on `fix/warrior-release` branches. No merge, tag or publication was performed.

## Public execution follow-up

A subsequent change adds actual bounded public execution verification to Zheng
and makes it the default in Joy. See [the implementation and validation report](../../zheng/audit/public-execution.md).
It authenticates the public program, input, output and reduction count, with full
witness disclosure and linear verification. It does not provide private/state
proofs or complete the full release gate. The measurements and archived
candidates below describe the earlier legacy path and are unchanged.

## Initial repair snapshot (before ownership follow-up)

- Core owns the frontend, typed IR, nox lowering, source metadata and an optional target-parametric neural harness. Trisha owns Triton instruction selection/emission, runtime/proofs, neural target adapters, Neptune source libraries and hand TASM baselines.
- nox honors cfg, lexical shadowing and qualified imports. Compilation, costs and bundle state metadata share lowering. Unsupported tree targets and unsupported transitive state calls fail explicitly; see [the supported nox surface](../reference/nox.md).
- Source programs now reach warrior-owned TASM through the shared typed IR. Bundle JSON preserves signatures/hashes/costs and rejects malformed identity fields. Compiler and Neptune resources are embedded.
- Explicit CLI targets override project targets; explicit source filenames remain selected inside projects. Missing warriors and malformed claims fail. `trident test` actually executes nox tests and propagates failures; Triton test execution is explicitly unsupported.
- Wide-stack spilling, aggregate/return layouts, tuple destructuring and cleanup optimizations were repaired. SHA-256 produces the FIPS empty-message digest on Triton VM; arithmetic helpers and the first round match independent Rust operations.
- Benchmark results require real execution against a declared expected output. Missing fixtures fail coverage; original assembly is never neutralized to obtain a measurement. Live deployment returns an unsupported error instead of reporting a transaction that never happened.
- TensorMerkle serialization preserves authenticated opening witnesses. Zheng rejects recursive opening forms whose authentication is not implemented. Joy refuses external IO claims and labels reported output/cycle metadata unverified.

## Initial repair validation

| Gate | Result and scope |
|---|---|
| Trident default suite | 718 passed, no ignored tests; full log `trident-release-final.log` |
| Final explicit-file CLI correction | 10 CLI execution/dispatch tests passed after the final source change |
| Core neural feature | 44 passed, including differentiable policy-gradient regression |
| Trisha default CPU + CLI suites | 216 passed, no failures/ignored; includes 4 real subprocess tests |
| Trisha neural adapter | 50 passed; used `CARGO_INCREMENTAL=0` after a Rust incremental compiler ICE |
| Trisha workspace all-features check | Passed; three pre-existing vendored Triton warnings |
| nox differential fixtures | 15 costable first-public-function entry paths covered by 9 tests; not whole-module equivalence |
| Standard library TASM | 42 tests parse actual emitted library code; only specifically executed cases establish output correctness |
| Final Trident registry package | `cargo package` built and verified 0.3.0 against registry dependencies |
| Separate source-tree builds | Locked optimized Trisha and Trident builds passed from archived repositories |
| Actual isolated installation | `cargo install --path ... --locked --root ...` passed for Trident, Trisha and Joy |
| Installed and archived binary smoke | Passed on Darwin arm64 with source trees moved out of their build paths, TRIDENT_* unset and PATH restricted to installed binaries |
| Reference benchmark | Both compiled and hand arithmetic executed/proved/verified: 11 and 7 cycles, expected output 38 |
| Complete historical benchmark gate | **Failed:** 1/43 baselines verified; 42 lack passing reference fixtures |
| Joy acceptance suite | 10 unit tests and 24 integration tests passed; **3 state-proof integration failures** at `UnsupportedRecursiveOpening` remain visible |
| Final Joy CLI artifact roundtrip | Arithmetic output 38, proof 64,715 bytes, statement-only verification passes; output metadata explicitly unverified |
| Compiler audit command | Arithmetic smoke reports zero constraints; its SAFE label is not evidence of execution or cryptographic soundness |

The portable smoke script is `trisha/scripts/smoke-release.nu`. It checks project-directed build/run/prove/verify through Trident, direct Trisha execution, public and secret input, absent secret failure, batch proofs, tampered-output rejection, unsupported deployment/target errors, nox execution and failing-test exit. It was run again after unpacking the exact binary archive.

## Earlier candidate artifacts

Local artifacts and retained command logs are in `trisha/target/release-candidate-20260911/`:

- `warrior-source.tar.gz`: pinned Trident/Trisha and local dependency repositories plus patched Triton vendor sources. `sources.json` records the commit closure.
- `warrior-darwin-arm64.tar.gz`: installed Trident, LSP, Trisha and Joy binaries, with an explicit development-candidate notice.
- `trident-lang-0.3.0.crate`: verified registry package; not published.
- `SHA256SUMS` and `logs/`: checksums and observed validation output.

| Artifact | SHA-256 |
|---|---|
| Source archive | `ff39dbd7ebfb9fb034af9e002cb8deeba699c9e707ed8d943e813cdb3e84294b` |
| Darwin arm64 binaries | `28eb613fad0a8b329bdff417c1219d21ea4987e6bf5fc11978394e1f4b0d8449` |
| Trident registry package | `95e0eec5d6f5065f4f72c30a98838641ad018811fe0edd75703f25fb8ca22083` |

The source archive pins Trident `ef6c420`, Trisha `7ff3c92`; the Joy binary is built from `0966981`. Zheng safety repairs are `0cbfe1f` and `45770c4`; Joy's claim restriction is `1b637e3`. Subsequent documentation commits do not change these binary/source artifacts. Only Darwin arm64 CPU execution/proving was exercised; no cross-platform or GPU performance release claim is made.

## Current release gates

The original public-output binding gap is repaired for the bounded public
execution certificate described above. The verifier now checks execution,
copy/control flow and public coordinates in an unfolded global CCS. This
certificate discloses the witness and has linear verification.

The requested full proof release remains incomplete:

1. Authenticated recursive state openings and private proofs in Zheng. The
   three positive Joy state-proof acceptance tests remain failing and visible.
2. Recursive STARK verification and Neptune transaction validation. The old
   SDK helper did not assert the values it computed; it is excluded from the
   production package and retained only as an experimental source.
3. Independent executable fixtures for all historical hand baselines: current
   verified coverage is 1/43.
4. Live deployment, Triton `#[test]` execution, transitive nox state calls and
   platform/backend validation for any claimed release surface.

The owner has authorized continued repairs. No unanswered scope question is
being used to stop implementation, and no narrower full-release claim is made.
See [the ownership implementation checkpoint](target-ownership.md)
for the current architecture and follow-up checks. The earlier binary archives
and hashes above do not contain this ownership migration.

## Ownership migration validation

The subsequent owner-approved migration has the following source validation:

| Check | Result |
|---|---|
| Trident default tests | 755 passed, no ignored tests |
| Core neural tests | 44 passed |
| Trisha CPU/CLI tests | 227 passed, including actual VM XField, shadowing and checked-cast regressions |
| Trisha neural adapter tests | 50 passed |
| Workspace checks | Trident all features/all targets, Trisha workspace all features/all targets, Joy workspace all targets passed; three vendored Triton warnings remain |
| Joy complete suite | 46 passed; three existing authenticated state-proof acceptance failures remain |
| nox regression file split | 28 tests passed after splitting the existing large test file; no semantic changes |

Raw logs for this checkpoint are `/tmp/trident-ownership-final.log`,
`/tmp/trident-ownership-neural-final.log`, `/tmp/trisha-ownership-final.log`,
`/tmp/trisha-ownership-neural-final.log`, and
`/tmp/joy-ownership-complete-suite.log`. The final installed and archived candidates below were rebuilt after the code
commits. Older archives above are not evidence for this migration.

### Final ownership candidates

Code commits: Trident `060494c`, Trisha `e97b543`, Joy `d065814`.
No merge, tag, registry publication or GitHub release was performed.

Local artifacts and retained logs:
`trisha/target/ownership-candidate-20260911/`.

| Artifact | SHA-256 |
|---|---|
| `warrior-source.tar.gz` | `c10c1e90ee09325a7b257eb575461c5f83ed8ddb713b79ac805c12dfdd0cd8e1` |
| `warrior-darwin-arm64.tar.gz` | `b83bb2322d2e083a66acd9e6e3b709aef8434544db3c174341c8113be98d5509` |
| `trident-lang-0.3.0.crate` | `acdc1af79374327b96156d5653ee073875eb92a31eec907d39188d63972d207b` |

- `cargo package --locked --offline` built and verified the compiler registry
  package: 483 files, 3.1 MiB uncompressed.
- The source archive includes ten repositories and 1,838 source files with
  SHA-256 inventory, plus the patched Triton vendor inventory. Working-tree
  snapshot mode preserves unrelated dirty BBG sources and records provenance;
  those dependency changes were neither committed nor reverted by this task.
- Every inventoried source/vendor hash was verified after extraction. All
  three CLI packages were rebuilt and installed from that extracted tree,
  with locked/offline resolution. Cargo dependency caches were reused.
- The unchanged full smoke passed on the development install, on the source-
  archive install after renaming its build-source directory, and again after
  unpacking the exact binary archive. Each run used an empty work directory,
  PATH limited to the installed binaries, and no resource/package overrides.
- Smoke covers project target selection, nox/Triton/Neptune check/build/package,
  owner-supplied state metadata, public/secret execution, supported proof and
  batch paths, altered result rejection, and unsupported state/recursive SDK
  requests. It certifies Darwin arm64 CPU behavior only.
- `trisha bench --full` returned nonzero as required: 1/1 reference fixture
  passed, 1/43 baselines verified, 42 unverified. Arithmetic input 5 produced
  38: classic 11 cycles, hand 7 cycles; both STARKs verified, padded height 256.

These candidates close the ownership/distribution migration gates. The full
production release gates listed above remain open. In particular, the three
Joy state-proof failures were retained in the complete-suite log.
