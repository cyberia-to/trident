# Full release preparation — 2026-09-16

Status: published — Trident 0.3.0, Trisha 0.3.0 and Joy 0.5.0.

All scoped release gates are closed. See the [published release report](release-2026-09-16.md) for exact source identities, six native targets, the 198-proof baseline gate, the 36-pair proof exchange, Neptune admission and artifact links. Broader nox and live database features remain roadmap work.

## Historical pre-release checkpoints

The following entries describe earlier candidates and do not represent current release blockers.

2026-09-16 continuation: FINAL6 remains blocked by the confirmed source-RAM
clobbers in [the scratch review](../../trisha/audit/ram-scratch-review.md).
Its proof/node/packaging gates were stopped before execution. Live-source return
cleanup, deep-stack access and inline assembly now preserve RAM in the regression
suite. All133 baseline execution fixtures pass; across99 positive cases51 use
fewer cycles and48 are unchanged. Three self-hash-bound inputs were independently
regenerated; the other96 inputs and all hand programs remain unchanged. Six fresh
CLI counterexample STARKs verify and six changed public outputs reject. The
exact new RAM smoke section adds two fresh proofs through Trident delegation
and two altered-output rejections. Exact evidence:
[RAM repair](../../trisha/audit/ram-scratch-repair.md).
Full workspace reruns pass: Trident896, Trisha460 (six explicit ignored proof
gates), Joy71; all24 release-tooling tests pass. A Keccak struct performance
regression found during the first full rerun was fixed without raising its
10-million-cycle limit; the complete rerun passes with both50-word vectors.
Fresh coordinated platform/packaging proof gates remain open. Earlier receipts
do not certify these edits. The prior
FINAL5 full-baseline receipt and its `/tmp` work directory are absent on this
continuation; that run's completion is unconfirmed.

Current authoritative checkpoint: frozen FINAL5 has native macOS/Linux builds,
complete installed proof smoke, byte-identical binary repacks, fresh-unpack
verification in both directions and actual isolated Neptune node admission.
The exact frozen macOS candidate had started the complete198-proof baseline gate;
its final receipt has not been recovered. Later live tooling/architecture work is absent
from FINAL5 and requires explicit source/artifact reconciliation. See
[FINAL5 evidence](../../trisha/audit/final5-release-candidate.md). Historical
198-proof results below retain their documented proof-time provenance limits.

The next exact source snapshot is
[FINAL6](../../trisha/audit/final6-release-candidate.md), SHA-256
`0219287c79902bad8b31433a065310766d5f8f3c9f0701f11422998e786ec0c3`.
Its11 repositories/2534 source entries and557 vendor entries verify after
extraction and reproduce byte-for-byte. Native rebuilds and installed checks
are tracked separately from FINAL5. The preserved Neptune intent has now aged
past its10-hour acceptance window; FINAL6 node admission requires a fresh
genuine SingleProof. No timestamp/expiry rule is relaxed to reuse it.

Post-FINAL5 live changes: the shared TIR now uses the selected XField width;
Trisha exclusively legalizes machine stack depths. All133 baseline executions
pass,48 positive fixtures use fewer cycles and none regresses, but40/84 native
program identities change. Three custom-token fixture statements were rebound
to the new actual self-hash through the independent codec generator. Exact
comparison: [owner legalization](owner-stack-legalization-receipt.json).
Changed native programs need fresh proofs; frozen FINAL5 evidence cannot be
relabeled. Ordinary Python baseline startup now preserves source inventory.
Joy/Trisha artifact readers reject blocking/special inputs and enforce bounded
reads; actual CLI regressions and old-proof compatibility pass. See
[file admission review](../../joy/audit/file-input-review.md).

The experimental tagged noun kernel has a separate draft contract and21 native/
constraint tests covering shape, hash, equality, word operations and quoted
composition. It is not selected by any production proof format. A separate
Trisha checker integration executes both branch shapes under the same program;
its genuine proof test now passes with two fresh Triton7 STARKs and complete
opposite-branch claim-transplant rejection. This remains experimental, with no
production protocol dispatch. [Actual proof evidence](../../trisha/audit/tagged-ccs-validation.md).
Pure scalar helper
formal analysis passes17 actual CLI/Z3 tests. The repeated48-module inventory
still has1,320 UNKNOWN and four UNSAFE guard verdicts out of1,324 functions;
no module is certified. See [formal evidence](formal-path-validation.md).
These are bounded steps toward the open general nox and whole-library formal
objectives, not completion claims.

The complete current Zheng workspace passes211 tests, including both exhaustive
wire-corruption tests, with zero failures/ignored/Rust warnings. Its release
workspace serde check also passes. Hemera's reviewed external changes retain
the existing constants/profile/framing and fixed vectors; eight actual vector/
profile tests pass. BBG and other sibling changes preserve the reviewed product
dependency identity set and public certificate encoding. Their exact compatibility
reviews remain distinct from source stability and cryptographic assurance.

Current complete Trident neural workspace:894 tests pass. Current Joy workspace:
71 tests pass, including real private proofs. All24 release Python tests pass,
covering exact source verification, snapshot fidelity, reproducible archives,
binary receipt binding and complete proof-event accounting. Exact downstream
workspace/check results are retained in their continuation receipts; the Trisha
ordinary suite passed450 tests before a missing-artifact doctest failure, whose
separate workspace documentation rerun passes after dependency rebuilding.
The original failure and actual rerun are both retained in
[workspace evidence](current-workspace-continuation.md). Joy's all-target/
all-feature check also passes. Exact24-test tooling receipt:
[release tooling](../../trisha/audit/current-release-tooling.json).
These live-source checks do not replace the frozen installed-platform gates.

Dependency migration is applied: the default backend and recursive verifier now
use pinned Triton VM/AIR/ISA/tasm-lib7.0.0, with native claim version5. The prior
Triton2 backend lacked the AIR soundness fixes shipped in Triton4–7, including
the Program Table fix in7. Historical Triton2 roundtrips below are regression
evidence only; their release proof gates must be rerun on7.
[Upstream changelog](https://github.com/TritonVM/triton-vm/blob/v7.0.0/CHANGELOG.md).

Trisha owns the pinned dependencies, GPU overlays and native/CCS formats. Joy's
JOYZK003 envelope requires `zheng-ccs-triton7-zk-v2` and rejects prior private
formats. Joy's current 62-test release suite passes on7, including genuine
private execution and state proofs. The canonical Neptune SingleProof gate passes,
including owner CLI submission and exact mempool admission on an isolated
pinned Neptune 0.15.1 Testnet(1) node. All three archive-built macOS binaries
pass complete installed smoke with recursive/public/private/state proofs.
All 43 baselines now pass 198 fresh source/hand proofs for their 99 positive
fixtures; all 34 rejection vectors pass execution. Later checkpoints supersede
historical tests.

## Current implementation

- Real Triton #[test] runner: preserves actual main/helpers/imports/cfg, compiles
  selected tests into typed TIR, executes bounded VM tests independently.
- Independent baseline vectors uncover and repair actual pow/hash/sponge/Merkle,
  RAM block, XField assignment and nested aggregate-field ABI defects. The suite
  keeps all 43 baseline files in the denominator and proves both candidates.
- Zheng derives a bounded global execution CCS; atom calls validate private
  witnesses, inactive arms do not require successful inverse/word/call operations,
  pair equality binds all structural digest limbs. Public format v2 is distinct
  from the earlier development certificate.
- Private execution: verifier-derived Zheng CCS checked by a deterministic
  Trisha-owned Triton7 program, native randomized STARK, Joy JOYZK003 artifact.
  Proof metadata cannot select another checker/program or replace public values.
- Private queries over public state now select namespace/index/value inside the
  proved CCS from authenticated complete tables. All10 public namespaces required;
  2048 total fields maximum. Actual CLI proof/fresh verification passed; private
  key absent from public statement, forged root/table/output rejected.
- Public state: injective versioned BBG dimension serialization, Lens systematic
  commitment v2, complete authenticated dimension certificates. Zheng pins actual
  namespace/key/value/activity and four root limbs to the same execution witness.
  Joy JOYST001 carries the certificate and independently checks native agreement.
- Nox compiler propagates roots through active local/imported state helpers and
  preserves stateless function ABI. Real public/private-state CLI proofs pass.
- Portable Poseidon2-HL matches pinned upstream Plonky3 on Triton and nox.
  Full-width bigint modular arithmetic and low-product Python integer vectors pass.
  Compiler fixes cover conditional aggregate return frames, mixed-width tuple
  wildcard cleanup, branch-local result widths and simultaneous tuple assignment.
  Early returns preserve lexical scope and stop loops. Runtime array indexing
  checks bounds before RHS effects and preserves aggregate layouts and neighbors.
- BBG public query verification authenticates root, exact entity key and cell
  context; private contextless payloads disclose no additional tables.
- Joy build uses the shared compile path with project/target/profile identity,
  atomic no-overwrite output and the specified JSON-v1 envelope.
- Generic owner-declared intrinsic signatures bind into target compilation
  identity and lower through typed `TargetCall`. Trisha's production recursive
  verifier binds the complete native claim and verifies the official STARK.
- Lens opening v3 authenticates each selected column once, while consuming every
  original transcript draw and enforcing the exact sorted unique set. Commitment
  roots remain v2. Old repeated-column openings require regeneration.
- Neptune offline deployment inspection returns the native program hash and
  verifies any attached execution proof. Node status rejects neptune-cli's
  exit-zero connection error and malformed height/mempool responses.
- Trisha's proof codec preserves canonical existing bytes while rejecting
  modular field aliases, inconsistent lengths and trailing bytes before proving
  APIs. Core run/prove forward warrior-owned file witnesses and digest queues.

## Remaining gates (keep open until evidence is recorded)

- Full baseline execution/proof coverage is closed for the declared 43-program
  inventory. The compiler pipeline remains a documented prototype and Trinity
  a finite-field arithmetic demonstration; those fixtures do not establish a
  complete self-hosted compiler or secure FHE. Broader promised implementations
  remain subject to their own acceptance requirements.
- Dynamic continuations/variable noun shapes and scalability beyond bounded
  public table selection. Recursive Triton SDK proofs and pinned Neptune
  transaction validation have separate completed receipts below.
- Hemera is explicitly novel and unaudited upstream. Correct execution of its
  constraints is not external cryptographic assurance for all roots/transcripts.
- Funded Neptune wallet construction, public-network operation and block
  confirmation. Canonical output/intent/SingleProof validation, authenticated
  adapter submission and actual isolated Testnet(1) mempool admission pass.
  The host's older node installation was not used for that gate; the pinned
  Neptune 0.15.1 node was built and run inside an isolated Linux namespace.
- Final supported-platform installed proof matrix and artifact checks after
  remaining source changes. Complete downstream suites and security/format
  regressions have current receipts below. Legacy contextless PCS APIs retain
  explicit limits.
- Coordinated breaking versions, migration notes, fresh reproducible source and
  binary archives, isolated installed CLI smoke with new private/state protocols,
  checksums and release notes tied to final commits. Fresh working-tree source
  and binary rehearsals exist; they are not final committed release artifacts.

Detailed test logs and exact results are recorded when each gate completes; this
file is a work ledger, not a release readiness assertion.

## Verified current checkpoints

- Joy complete workspace60tests pass on Triton7 with zero warnings, including
  build5tests and real JOYZK003 private execution/state proofs
  (`/tmp/joy-triton7-full-release.log`). Old envelope and backend formats reject.
- Zheng execution32tests pass, including malicious read-provider witnesses that
  fail the CCS itself, all four root limbs, private calls and pair equality.
  `/tmp/zheng-private-state-tests.log`. Default library172tests pass after legacy
  migration. Fresh full release/serde workspace185tests pass, including178
  library tests and exhaustive wire mutations (`/tmp/zheng-opening-v3-full.log`).
  A subsequent37-test execution run adds five exact-budget branch regressions:
  only the selected branch's bound cost consumes budget, with inactive-operation
  and forged-cost checks retained.
- BBG71tests pass against opening v3 (`/tmp/bbg-opening-v3-full.log`); full Lens
  workspace129tests pass with all features in release after independent review
  (`/tmp/lens-opening-v3-reviewed-full.log`). This includes canonical column
  bounds before hashing, independent full transcript draws and legacy-wire
  rejection. No sampled PCS128bit claim is made.
- Bigint full8 regression tests pass in release (7.52s), plus6 broader boundary
  tests under both debug/release source profiles. Logs:
  `/tmp/trisha-bigint-release.log`, `/tmp/trisha-bigint-boundaries.log`.
  Four frame and five control-flow real-VM regressions pass; early returns9 and
  runtime arrays9 pass in both source profiles (`/tmp/trisha-early-return-final.log`,
  `/tmp/trisha-array-final.log`). Trident763 default and807 with neural pass
  after the array edits (`/tmp/trident-post-array-full.log`,
  `/tmp/trident-neural-post-array.log`). The later CLI transport regression and
  five existing dispatch tests pass (`/tmp/trident-input-transport.log`).
- Standard Poseidon2-HL matches pinned Plonky3 on Triton and nox. The independent
  hand/source fixtures produce equal digests: source7105cycles, hand7027cycles.
- Historical Triton2 recursive SDK checkpoint (must regenerate on7): tests pass
  against actual official proofs, including
  changed claim program/version/input/output/length and malformed proof rejection
  (`/tmp/trisha-recursive-production-tests.log`). The explicit expensive outer
  proof test passed: 426962cycles, padded524288, 130402encoded proof fields,
  88.656s generation, peak19.23GiB, no swaps (`/tmp/trisha-recursive-outer-proof.log`).
  The test is deliberately ignored in ordinary suites and must be invoked with
  `--ignored` for the release gate. All four recursive baselines pass full
  proofs for both source and hand programs: eight generated and freshly verified
  STARKs (`/tmp/trisha-recursive-baselines-full.log`). Aggregation852915/852860
  cycles, padded1048576 each; two proofs598.64s, peak RSS34.00GiB, no swaps.
  OS peak memory footprint51.26GiB includes compressed memory; this differs from
  peak RSS and must remain visible in resource guidance.
- Right-shift lowering uses the correct numerator/divisor order and keeps the
  quotient. Two word-operation tests cover512 real VM vectors; two canonical
  proof-wire regressions pass (`/tmp/trisha-word-wire-final.log`).
- CLI file witnesses and explicit expected-claim preparation pass real process
  tests, including recursive execution, incorrect claims, private file mode and
  input bounds. Installed smoke now includes the full recursive outer proof;
  it has not yet been run against a fresh final archive.
- Neptune inspection native-hash/real-proof test and two CLI tests pass
  (`/tmp/trisha-deploy-rs-tests.log`, `/tmp/trisha-neptune-cli-tests.log`).
  These inspect programs and mocked/read-only status; they do not validate or
  submit Neptune transactions.
- Archive rehearsal built isolated Trident+Trisha+Joy without warnings, from a
  worktree snapshot. That rehearsal predates final private-stateformatv2 and
  current crypto edits; a fresh final archive/smoke is still required.

## Latest verification after the backend migration

- Trident release suite765tests passes without warnings after event and witness
  transport fixes (`/tmp/trident-release-final.log`); neural feature compilation
  passes (`/tmp/trident-neural-final.log`). Earlier807-test neural run predates
  the event changes and is not the final feature-suite receipt.
- Five actual-VM event tests pass on Triton7 for debug/release source profiles:
  declaration-order Reveal/Seal payloads, aggregate fields, preserved locals,
  once-only field effects and independent Tip5 commitments. Huge aggregate and
  duplicate fields reject; nox explicitly rejects unsupported event operations.
  Logs: `/tmp/events-triton7-final.log`, `/tmp/events-nox.log`.
- Four canonical native proof input tests pass on7, including exact proof-item
  consumption, hostile log-height values without panic, canonical field bytes,
  bounded lengths and trailing-byte rejection (`/tmp/trisha7-proof-wire-guards.log`).
- Quantum hand assembly has been replaced from independent complex arithmetic
  and RAM algorithms. Every21pure gate is exercised on four independent vectors;
  four complete baseline fixtures agree (source27388–27390cycles, hand739).
  RAM gate fixtures cover all targets, complex coefficients, state neighbors,
  highest valid U32 boundary and invalid addresses/dimensions before mutation.
  This is finite-field simulation; legacy measurement is an exact predicate,
  not probabilistic quantum measurement. Logs: `/tmp/quantum-pure-baselines.log`,
  `/tmp/quantum-all-gates-final.log`; full Triton7 proofs remain pending.
- Full SHA-256 compression and Keccak24rounds now have explicit bounded RAM
  interfaces. Independent hash/matrix vectors remain the oracle. RAM double-SHA
  uses188106cycles and Keccak826062cycles; these are execution observations,
  not proof receipts. All baseline43files stay in the denominator.

## 2026-09-12 continuation

- Canonical Neptune0.15.1/HardforkGamma SingleProof generated and independently
  verified on Triton7 for a deterministic empty transaction, after generating
  and verifying all five actual subproofs. Native trace1155385cycles,
  padded2097152; final proof518.771seconds, pipeline531.64seconds. Peak RSS
  25828950016bytes, OS peak footprint31741336592bytes, zero swaps. Four Rayon
  threads and `TVM_LDE_TRACE=no_cache`; no network submission or mock proofs.
  `/tmp/neptune-single-proof-full.log`; compiled policy wrapper/outer proof
  integration remains a separate gate owned by the Trisha fixture work.
- CPU and wgpu now share the exact native input/claim/proof codec and verifier.
  Both reject noncanonical input, oversized FRI index domains, appended proof
  items and binary aliases. Private execution errors suppress VM witness dumps.
  ProgramInput is capped at8Mi field words, counting every digest coordinate.
  JSON uses an eight-byte canonical Field representation and bounded sequence
  visitors, with tighter expected-claim limits. Six SDK/wire tests, two JSON
  tests and one real-proof GPU acceptance test pass without warnings:
  `/tmp/trisha7-input-guards.log`, `/tmp/trisha7-json-input-guards.log`,
  `/tmp/trisha7-gpu-proof-guards.log`. All-feature/all-target workspace check
  passed before the coordinated product-version bump.
- Full current Zheng release/serde workspace190tests passes, including183
  library tests and both exhaustive wire mutation scans; library612.90seconds.
  `/tmp/zheng-release-current-final.log`. These include the five exact-budget
  regressions added after the earlier185-test receipt.
- Coordinated versions are applied: Trident0.4, Trisha0.3, Joy0.5, Zheng0.4,
  BBG0.3, nox0.3 and affected Lens packages0.2. Compiler API2 rejects stale
  warriors; package JSON schema1 remains unchanged. All eight Cargo roots pass
  locked offline metadata checks, with no registry package changes. Complete
  map and32 focused test receipts: [version closure](release-version-closure.md).
  Individual registry crates still lack sibling resources/root patch propagation;
  the implemented distribution route is the full source closure plus binaries.
- Snapshot fidelity test passes; candidate-builder Nushell syntax passes.
  Final archives and installed binary smoke remain pending final source changes.

## Current coordinated suite and platform receipts

- All133 reference fixtures pass:99 positive and34 rejection vectors, covering
  all43 manual baselines. No H0003 hints or unused-import diagnostics remain in
  the latest complete execution run (`/tmp/h0003-bench.log`). This is execution
  coverage. Full source/hand proofs are a separate required gate:6 heavy
  baselines have6 positive/12 negative fixtures, and the other37 have93
  positive/22 negative fixtures. Their union is the exact original43/133 set.
- Trisha release workspace with all features passes428 tests, zero failures and
  zero compiler warnings, across55 test/doc-test binaries. Three explicit heavy
  outer/deployment proof tests are excluded from this ordinary suite and have
  separate required gates. The default CLI package passes33 tests across7
  binaries, with one intentional deployment-proof exclusion. The PLUMB compile
  regressions check actual old/new
  root and shared-path mutations. See `trisha/audit/final-workspace-validation.md`
  in the coordinated tree.
- Coordinated release suites pass: Joy60, Zheng190, Lens129, BBG71, nox169
  default/175 all-feature tests. Trident's full neural workspace now passes851
  tests across18 test/doc-test binaries with zero failures, ignored tests or
  warnings after formal-audit and registry state-selection fixes
  (`/tmp/formal-trident-workspace-neural-final.log`).
- Default CPU builds for Trident/Trisha/Joy pass Rust1.89 compilation. Linux
  x86_64 cross-checks pass for all three, with all features checked for Trisha
  and nox. Apple-only mining dependencies and dispatch are target-gated; the
  portable Tip5 path matches official hashes, including raw Montgomery words
  and padding. These are compile checks, not Linux binary execution receipts.
  Detailed commands: `trisha/audit/platform-validation.md`.
- nox jet admission now checks exponents and exact bounded shapes before shifts,
  allocation or jet charging. Declined acceleration preserves normal reduction,
  result/error, trace and budget. The existing NTT pure anchor is only a single
  butterfly; the repair no longer substitutes a full transform for that
  different formula. A full recursive NTT anchor remains unimplemented.

## Subsequent release preparation

- All six heavy baseline programs pass their twelve fresh source/hand outer
  STARKs on Triton 7. The separate compiled SDK outer-proof gate also passes:
  430989 cycles, padded 524288, 130.947 s proof generation. Exact paired cycles,
  wall times and distinct RSS/OS footprint observations are recorded in
  `trisha/audit/recursive-release.md`. The other 37 programs' 186 proofs also
  pass: full union43 baselines/99 positives/198 verified proofs, with all34
  rejection cases passing execution. Machine-readable exact fixture accounting
  is in `trisha/audit/full-baseline-proof-coverage.json`.
- Formal audit now uses independent function obligations and injective SSA
  symbols, honors scalar contracts and reports incomplete analysis as UNKNOWN.
  Z3 failures and counterexamples cannot become SAFE. Current full inventory:
  48 modules (34 core, 14 production SDK); 46 UNKNOWN files, 2 files with
  counterexamples, 1321 UNKNOWN functions and 3 SAT obligations. The latter
  exercise intentionally rejecting guards without sufficient caller assumptions.
  No module is claimed formally proved. Typed aggregates, interprocedural and
  recursive contracts, loops, RAM and intrinsic summaries remain open. See
  [formal validation](formal-release-validation.md).
- Trisha now owns a fifth, pinned Neptune consensus/RPC adapter crate. Its
  output constructor binds the compiled lock, canonical UTXO and addition
  record; complete transaction preparation fixes the caller-selected kernel
  and validates a real SingleProof and every output preimage. Submission uses
  an explicitly configured authenticated gateway with canonical bounded RPC.
  The generic runtime deployment capability remains false because that trait
  cannot carry the required transaction intent. Both genuine proof/CLI gates
  pass, followed by actual installed CLI submission and exact kernel admission
  on the pinned isolated Testnet(1) node. Changed kernel/original proof and
  wrong gateway credentials reject; node height remains 0 and cleanup is
  confirmed. See `trisha/audit/neptune-local-node-validation.md`. Caller-supplied
  transaction validation does not implement funded wallet coin selection or
  establish chain finality.
- Independent adapter review found a FIFO input-open hang; the loader now
  rejects nonregular files before opening and its regression passes. A distinct
  local-testnet1 state names chain 4/testnet-1. Upstream regtest admits mock
  proofs only, so it cannot establish real SingleProof acceptance. The completed
  real-proof node gate uses fresh genesis state/timestamp, isolated network
  access and exact kernel presence in the mempool after RPC acceptance.
- Mining now follows the pinned fork schedule, verifies independent PoW/MAST
  layouts, synchronizes CPU/GPU winner ownership and bounds nonce reservations
  across template updates. Eight mining tests and seven CLI lifecycle tests
  pass, including actual Metal oracle comparisons. Full legacy buffer
  construction and actual network mining remain distinct operational gates.
  See `trisha/audit/neptune-mining-compatibility.md`.
- Updated Trisha default CPU closure passes Rust 1.89, and its all-feature Linux
  x86_64 cross-check passes without warnings after the new adapter/mining
  changes. All three arm64 Linux binaries cross-link, with an explicit Zig
  linker deprecation warning in that earlier cross-build. Native Rust 1.89
  release builds now pass with zero warnings in the isolated Ubuntu 26.04 VM,
  together with installed execution checks. The full Linux proof smoke runs
  separately. See `trisha/audit/platform-validation.md`
  and `trisha/audit/linux-runtime-validation.md`.
- Source packaging now checks every dirty source input, creates deterministic
  archives with normalized metadata, and includes a separately locked state
  fixture helper. Both packaging regressions and the helper's Rust 1.89 run
  pass. New worktree snapshots remain rehearsals until final coordinated
  commits, binary builds and installed smoke are recorded.

## Final source continuation — 2026-09-12

- Both native archive-built platforms now pass complete installed smoke, including
  the real recursive outer proof, JOYEXEC2, JOYZK003 and JOYST001 positive and
  rejection paths. Darwin wall141.50s, monitor145.634s/peak9,351,577,600bytes;
  Linux monitor142.342s/peak7,818,088,448bytes. Actual binary archives reproduce
  byte-for-byte. They retain rehearsal1 source identity; subsequent source
  changes below require fresh candidates. See Trisha's installed/Linux audits.
- nox now lowers early returns in bounded loops, including nested branches,
  imported functions and aggregate results. Two associated scope defects are
  repaired: post-loop bindings cannot leak backwards into loop lowering, and
  sealed shadow frames cannot retain a different aggregate type layout. The
  original installed candidate rejects a valid reproducer; the corrected code
  agrees with eight actual Triton executions. Full Trident neural workspace:
  858 passed, zero failed/ignored/warnings. Current Joy workspace:62 passed,
  zero failed/ignored/warnings, including genuine public/private loop proofs.
  See [scope review](release-scope-review.md) and
  [independent review](nox-loop-return-review.md).
- Parallel BBG development introduced the dependency-free neuron-id crate.
  Preserve that work: minimal Joy and fixture-helper lock entries add the local
  package without changing registry versions. The 32-byte identity alias and
  public proof/state formats are unchanged; all three archived public fixtures
  verify and regenerate byte-for-byte. Current BBG default82 tests pass without
  warnings; all-features152 pass with three existing Fjall warnings, recorded
  separately. See `bbg/audit/release-dependency-continuation.md`. The next source
  closure includes Neuron; rehearsal1 still contains exactly10 repositories.
- Release tooling now verifies complete repository/vendor inventories before
  and after compilation, rejects escaping symlinks and unsafe destinations,
  and removes partial candidates on failure. Build, smoke and binary archive
  bind the exact source-verification receipt; smoke also binds the executed
  script, which is shipped from the verified source archive. Current verifier
  process/mutation tests8 and binary-packager tests3 pass; synthetic packaging
  tests are not proof evidence. Strict committed source manifests now include
  the same per-file inventories as snapshots.

## Typed-entry release continuation — 2026-09-12

FINAL3 now has native macOS and Linux full installed proof smoke, reproducible
binary archives, fresh extracted verification and actual Linux Neptune node
admission with its exact binary. Evidence: [candidate report](../../trisha/audit/final3-release-candidate.md).
A subsequent correctness review found uninitialized typed Triton entry parameters.
The live complete input adapter is implemented with actual VM ordering, aggregate,
narrow-type and genuine proof/claim-mutation tests. It requires compiler API3;
API2 descriptors are rejected before foreign compilation/dispatch. This fixes
source semantics in addition to the already sound assembly/claim binding.
FINAL3 remains unchanged and must not be described as ready after this finding.
The next installed smoke includes this bug and nox bounded loop/scope regressions.
Source/package helper regression suite:14 tests pass in16.575s on macOS
(`/tmp/cyber-final4-packaging-tests.log`). This is tooling evidence, not proofs.
The remaining formal, general nox, self-hosting, FHE, external assurance and
committed/publication gates above remain open until independently satisfied.

## nox source-entry and scalar formal continuation — 2026-09-12

Additional accepted-source defects were reproduced and fixed: nox entry arity,
U32/Bool range admission, flat aggregate marshalling, imported array dimensions,
and deep noun read/write paths. Native45-test source surface and3 Joy proof tests
pass. [Exact evidence](typed-nox-entry-validation.md).

Scalar formal audit now models bounded branches, early returns and each path's
postconditions with target-correct Bool/Field truth. Twelve real CLI/Z3/native
checks and83 verification unit tests pass. The complete library inventory still
reports46 UNKNOWN modules and2 counterexample modules; no whole-library SAFE
claim is made. [Formal evidence](formal-path-validation.md).

Current Triton compiler output matches all84 compared native program hashes for
both FINAL3 and the retained original proof-generation binary;266 baseline files
match FINAL3. Historical proof logs lack full proof-time input inventories for
all198 runs. Preserve that provenance limitation rather than retrospectively
claiming signed or cryptographically bound historical source identities.
[Continuity analysis](../../trisha/audit/typed-entry-baseline-continuity.md).

## Generic and terminal-return continuation — 2026-09-12

Further source review found defects outside entry marshalling: imported generic
bodies were not concretely checked, constant/layout ownership could be lost,
Triton struct literals could reverse named fields, and nox discarded terminal
conditional values. These are being repaired before the next source freeze;
the earlier84-program continuity comparison predates these changes.

The nox terminal fix passes the complete48-test native source surface and two
Joy proof tests, including a genuine private STARK and claim mutations.
[Evidence](terminal-return-validation.md). Ordinary return types and concrete
generic bodies now undergo checking; self-hosted compiler unit helpers explicitly
discard their intended unused Field results. Broad suites are rerunning against
the coherent changes. New installed smoke includes imported generics, named
struct layout, terminal branches and invalid return-type rejection.

Strict committed-source packaging was exercised with11 isolated fixture Git
repositories, retaining all2442 archived source entries and557 vendor entries.
[Packaging evidence](../../trisha/audit/strict-source-packaging-validation.md).
Fixture commits are not production release commits. A fresh full baseline gate
will bind actual proof verifications to exact candidate and source inventories.

The coherent compiler checkpoint passes884 Trident tests (49 actual nox surface),
448 all-feature Trisha workspace tests and67 Joy tests, with zero Rust warnings.
Five explicit Trisha proof/transaction tests remain excluded from that ordinary
suite and retain separate proof receipts; they are not counted as passing here.
All-target/all-feature checks pass in all three workspaces. Release tooling
passes23 Python tests. Logs: `/tmp/{trident,trisha}-final4-workspace-reviewed.log`,
`/tmp/joy-final4-workspace.log`, `/tmp/{trident,trisha,joy}-final4-check.log`,
`/tmp/trisha-final4-tooling-tests.log`.

The Trisha broad run first exposed four intermediate-branch width failures and
two stale generated-label assertions. The former are fixed by preserving value
context while discarding intermediate branch tails; the latter now execute
concrete/inferred generic calls on the actual VM. No diagnostic guard was
weakened. Field literals also now emit canonical target residues, matching the
native nox and corrected scalar formal semantics; U32/dimension bounds remain
integer checks. Two native Triton regressions pass for these literal cases.
The updated installed smoke includes the canonical Field constant case.

FINAL4 builds on both native platforms but fails the new installed generic loop
case: nox did not expose an unrolled immutable loop index to array access
lowering. FINAL4 is retained as failed; no binary package or full baseline proof
gate ran for it. Scoped bindings now retain each iteration's known index,
preserving nested/shadowed/sealed frames and U32 limits. The complete Trident
suite passes888 tests, including53 native nox source tests.

The exact failed fixture now passes both profiles and both inputs803/809 through
Joy, including genuine private proofs. New evidence totals8 public certificates
and4 private STARKs with input/output mutations rejected:
`joy/audit/loop-index-smoke-review.md`. A separate development CLI replay of the
remaining installed-smoke section passes. It also caught and fixed a smoke-script
field lookup (`claim.public_input`, not a top-level proof field); expected
claims and modified-proof rejection now execute. This replay is not a complete
installed release receipt. The next frozen candidate must repeat the full gate.
