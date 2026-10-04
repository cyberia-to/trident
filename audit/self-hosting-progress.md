# Self-hosting on soft3 — progress ledger

Updated: 2026-10-03. Working contract:
[reference/self-hosting.md](../reference/self-hosting.md).
This ledger tracks acceptance. Linked receipts retain exact commands, revisions,
source/artifact identities, failed experiments and historical resource limits.
Component results close only their named substeps.

## Current position

Follow-on work: [VB — Verified Bootstrap](../roadmap/verified-bootstrap.md) is
open. Its next slice is VB0: exact delivery inventory, claim/assumption matrix
and own nox seed/Eidos kernel experiment contract. The accepted SH results below prove
their declared execution claims; source/binary correspondence, semantic
preservation and full canonical nox/Zheng/Joy verification have separate VB gates.

The current S1 compiler has compiled its complete source through Joy/nox,
published usable C2 and reproduced it byte for byte as C3. SH0–SH6 are closed
for this frozen subset and measured workload. Twelve independent native
bootstrap repetitions and all twenty-four actual C2/C3 corpus jobs passed on macOS, Linux
and Windows, each ARM64 and x64. The original CI aggregate and independent
byte-exact replay both pass. The earlier
S0 snapshot also reached exact C2/C3 equality and passed both supplied-compiler
corpora. SH7 now accepts the production native public compiler profile through
[actual C2 pilots and independent review](self-hosting/native-proof-pilots/README.md).
The complete SH7 archive has [verified durable retention](self-hosting/native-proof-pilots/durable-retention/README.md), including independently downloaded parts, full archive reconstruction and both acceptance-commit install attempts.
SH8 is [accepted for frozen S1](self-hosting/bootstrap-results/whole-proof-final/acceptance/README.md) under `joy-nox-disclosed-compiler-v1`. Both complete self-build proofs pass fresh-process verification; the extracted C2 and C3 match byte for byte and each passes all 547 corpus observations. Final checker F5 accepts 23 distinct rejections and two original controls per generation, combining explicitly replayed prior results with fresh sequential completion. The [public summary](self-hosting/bootstrap-results/whole-proof-final/acceptance/packet/summary.json) binds those results to the original commands and source. Earlier V2 and V3 attempts remain failed; V4 retains its `input-changed` outcome. The accepted profile discloses the complete public witness; physical resources remain unattested host observations.

The [proof-extracted compiler corpora](self-hosting/whole-proof-corpus-v2-result/README.md) retain their original execution evidence. Complete byte-equivalence retention is accepted separately in Trisha [PR25](https://github.com/cyberia-to/trisha/pull/25), merged as `fbea3cef9a4139075e529c319ff75488ed5df625`. The [retained audit](https://github.com/cyberia-to/trisha/blob/fbea3cef9a4139075e529c319ff75488ed5df625/audit/whole-retention-byte-closure/README.md) binds all 22 ordered parts, both complete digests, independent replay and the original failed local attempt.

S1 is Trident `77213171d39b88c5f41221912251cc4813ac2b11`: 94 modules,
484 functions and 370544 source bytes. The [reviewed feature map](self-hosting/compiler-feature-coverage.md)
covers the unchanged 51 feature kinds, with 65 named Rust references and 145
historical installed observations. The [split Rust test gate](self-hosting/lexer-frame-chunks/full-gate.json)
passed 1197 distinct tests, with five existing ignores and zero Rust warnings.
A reviewed two-test rerun is counted separately from those unique tests.
All 21 examples also [built with zero warnings](self-hosting/lexer-bootstrap/example-build/README.md);
that build adds no test passes.

The [current complete build and source-scale evidence](self-hosting/lexer-bootstrap/README.md)
retain S1's actual C2 SHA256
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
C1(S1) charged 10378203737 reductions, allocated 190817237 nodes cumulatively,
peaked at 3145728 resident nodes, completed 66 collections and took 1425188177
worker microseconds. Actual C2 passed all six source-scale cases: the valid
65536-byte comment uses 4324 frames and emits the same independently executed
program as the short case. Negative cases preserve the prior published program.
Full S1 package relocation/reversal preserves exact JOB1 bytes; a dependency
change changes package identity. These results close the measured SH4 barrier.
The separate identifier/decimal/name scanners retain their documented bounds;
this trivia repair does not claim universal success for every maximal token.

Actual S1 C2 passed [all six unchanged semantic corpora](self-hosting/lexer-bootstrap/c2-corpus/README.md):
547 observations and 1816 commands, including 119 explicitly identified raw
Rust reference builds and no compiler replacements. Its [second complete self-build](self-hosting/lexer-bootstrap/fixed-point/README.md)
charged 9777538159 reductions, allocated 162296944 nodes and took 1251723053
worker microseconds. Strong source/JOB1/producer checks establish exact C2/C3
byte and particle equality. The [actual C3 corpus](self-hosting/lexer-bootstrap/c3-corpus/README.md)
also passed all 547 observations and 1816 commands using the true second
producer output. The [local continuation](self-hosting/lexer-bootstrap/orchestration/README.md)
completed with every stage successful.

The prior split-repetition bootstrap used implementation
`c17bd0371c11746f46e20222c48cae2ab08be79d` and Joy
`ec83bd8d85b20a8bd20d2d14b0f25aab0f75e9fe` ([Joy PR22](https://github.com/cyberia-to/joy/pull/22)).
[The original CI run](https://github.com/cyberia-to/trident/actions/runs/36353842247)
at `23691cd2c6885bf25bfc023799552559724dbc2b` hit the explicit 3600000 ms
deadline on Intel macOS during repetition 1's `C1(S1) -> C2`, emitting no C2.
Its C1, JOB1 and inventory exactly match the successful local run; the
[original failed artifact and log](self-hosting/bootstrap-results/run-36353842247/intel-deadline/README.md)
remain byte-exact. The [complete original-run archive](self-hosting/bootstrap-results/run-36353842247/README.md)
retains the final failed matrix: Linux ARM64 completed both repetitions;
the other four platforms reached repetition 2 and then hit the 330-minute
GitHub step limit. Their uploaded outer receipts remain `running`, and the
original aggregate fails with `bootstrap not passed`. Exact restoration and
local replay preserve those outcomes. These historical results do not replace
any repetition of the new run.

The reviewed [split-repetition delivery](self-hosting/bootstrap-results/split-repetitions/README.md)
starts two independent jobs per native platform with an explicit 7200000 ms
whole-compiler deadline. Guest computational/memory limits and all corpus
quotas are unchanged. Its v2 aggregate requires all twelve distinct results
from the same CI run/attempt/head and exact C2/C3 bytes. The orchestration and
fixed-point guards pass 46 distinct tests; Joy passes 172 tests. These checks
establish no full native acceptance. [The new twelve-job CI run](https://github.com/cyberia-to/trident/actions/runs/36359020560)
closed with failure at `c17bd03`. [Both repetitions on macOS ARM64 and Linux/Windows x64/ARM64](self-hosting/bootstrap-results/run-36359020560/README.md) pass, including exact restored artifacts and both actual compiler corpora. Both Intel macOS repetitions completed both self-builds, exact C2/C3 equality and all six C2 corpora, then reached the 330-minute outer step limit during C3 corpora. The original aggregate rejected those partial reports and left SH6 open at that point. The following orchestration delivery separated each fresh producer from its two native corpus jobs while preserving exact producer tools, all test cases and execution limits.

[PR112](https://github.com/cyberia-to/trident/pull/112) carries the reviewed phase implementation at `57491633fbccb58ae44dca2da438ee31430be1bc`. The [original native run](https://github.com/cyberia-to/trident/actions/runs/36382085561), attempt 1, completed successfully on 2026-09-28. All twelve fresh producers and all twenty-four native C2/C3 corpus jobs pass. Each generation executes the same six corpora, 547 observations and 1816 commands, with exact producer, selected compiler, Joy, source, profile and run/attempt/head bindings. Both native Cargo builds per producer have zero warnings. The original aggregate compares every phase and yields the same C2/C3 bytes. The [accepted retained matrix](self-hosting/bootstrap-results/run-36382085561/README.md) contains 88488 expanded files and 1621286190 bytes, counting shared copies separately. A fresh restore of all three archive stores, byte comparison against every original ZIP and independent frozen matrix replay passed; the verification SHA256 is `be6d76f37c0daf9694e75dad83e200adc25a32b05e593fdbeec11f4d67a69f81`. Every non-time self-build field matches the original S1 reference. Earlier failed runs supply no phase to this matrix. The [source-bound preparation audit](https://github.com/cyberia-to/trident/blob/57491633fbccb58ae44dca2da438ee31430be1bc/audit/self-hosting/bootstrap-results/phase-jobs/README.md) separately retains orchestration guards and their original diagnostics.
Receipts were developed separately on `test/0.4-selfhost-acceptance` to preserve
the tested CI head; they now integrate through their own PR into `release/0.4`.
The reviewed [artifact replay tool](self-hosting/bootstrap-results/archive-tool/README.md)
preserves every raw platform file with a shared byte store and exact path
manifests. Its real Intel roundtrip and 19 corruption/boundary guards pass;
storage validation supplies no additional platform acceptance.

The later Windows audit-path correction in [PR117](https://github.com/cyberia-to/trident/pull/117)
passed all 37 jobs of [original run37002840888, attempt1](https://github.com/cyberia-to/trident/actions/runs/37002840888)
at `10772124836b68a2df011c912b5776b2454a511f`. The [retained collection and two offline replays](self-hosting/bootstrap-results/whole-proof-final/pr117-ci-collection/README.md)
bind all 37 original artifact ZIPs and attempt logs, twelve producers, twenty-four
corpus jobs with 547 observations each, and the same `76a07c08` compiler. All
replay fields agree except timestamps and the replay output path. Earlier failed downloads, both recovered
curl failures and the first peer launch's pre-spawn interpreter-path rejection
remain retained. Exact-tree review accepted merge
`8da6f8f6efb3eafb4697f3ce7da74e059d18e02e` into `release/0.4`.
This matrix retains its original Rust1.95 profile; the accepted Rust1.89 package
rehearsal and accepted frozen-S1 SH8 proofs keep their own source-bound gates.

Historical S0 is `b991d901`: 94 modules, 484 functions and 369820 source bytes.
Its [complete build](self-hosting/body-scale/README.md) published C2 SHA256
`fe0390b92257edf58686e50571160fc7985b0883ede116c6dcfd0a9f7820b5d0`.
The [actual C2 corpus](self-hosting/c2-corpus/README.md) and independent
[actual C3 corpus](self-hosting/c3-corpus/README.md) each passed 547 observations
and 1816 commands with unchanged cases and limits. The [fixed-point receipt](self-hosting/fixed-point/README.md)
binds both actual source packages and producer steps to exact C2/C3 equality.
Those immutable measurements remain distinct from the current S1 acceptance.

Bounded NoTrace compaction is merged into `release/0.4`: nox
[PR23](https://github.com/cyberia-to/nox/pull/23) and Joy
[PR20](https://github.com/cyberia-to/joy/pull/20). Joy's explicit worker ceiling
is extended by [PR21](https://github.com/cyberia-to/joy/pull/21), with default
limits unchanged; its [receipt](https://github.com/cyberia-to/joy/blob/15202f42240ba923398908ee0db62d32f1d655f8/audit/compiler-work-budget/README.md)
records the boundary checks. [Nox phase localization](https://github.com/cyberia-to/nox/blob/2f09ca3c3f18ae470365310cca8db5208eda75c6/audit/prefix-frontier/README.md)
is retained in [PR24](https://github.com/cyberia-to/nox/pull/24).
Earlier Trident scaling merged in [PR109](https://github.com/cyberia-to/trident/pull/109).
Alias/export lookup changes, complete C1(S) and audit tooling merged in
[PR110](https://github.com/cyberia-to/trident/pull/110) at `2184197`. Current
bootstrap implementation is merged through [PR112](https://github.com/cyberia-to/trident/pull/112). The scanner repair
merged through [PR111](https://github.com/cyberia-to/trident/pull/111) at
`c54446a`; integration targets `release/0.4`. Master remains untouched.

The earlier proof foundation integrated separately: nox [PR25](https://github.com/cyberia-to/nox/pull/25) merged as `172811b` into `release/0.4`, supplying bounded logical observation. Its [clean Joy integration and complete ordinary C2(S1) → C3 check](https://github.com/cyberia-to/nox/blob/84e35daab2a9af066c5d8ec85f73837083fbc1df/audit/semantic-observer/whole-compiler/README.md) preserve exact compiler bytes and every non-time execution field; capture was disabled for this compatibility check. Zheng [PR37](https://github.com/cyberia-to/zheng/pull/37) merged as `c753f5a`, adding only the draft noun/Cost/read-port relation proposal. [Integration identities](self-hosting/bootstrap-results/semantic-observer-integration/integration.json) bind both reviewed trees to their merge commits. Frozen SH6 source pins remain unchanged. Authenticated memory, compiler proof constraints and production proof dispatch remained open at that revision.

Zheng [PR39](https://github.com/cyberia-to/zheng/pull/39) implemented the first internal noun/Cost component and merged as `633e5ba` into `release/0.4`. Its fixed schema constrains native Hemera headers, all Cost cases, full-u64 saturation and six explicit read premises; unresolved reads prevent ordinary relation finalization. [The retained audit](https://github.com/cyberia-to/zheng/blob/2f79869a81a28daff3d700863ec593339ba4d08f/audit/noun-cost-component/README.md) records 229 default and 236 all-feature test passes, each with one existing ignored test and zero warnings, plus fourteen component tests. These totals overlap and are not added. The [integration receipt](self-hosting/bootstrap-results/noun-cost-integration/integration.json) binds all fourteen measured source identities to the committed delivery and exact merge tree. The separate example-feature repair is [PR38](https://github.com/cyberia-to/zheng/pull/38). At that revision, authenticated memory, transitions and production proof dispatch remained open; SH6 pins and SH7/SH8 acceptance were unchanged.

The integrated nox/Zheng revisions also pass the [clean downstream regression](self-hosting/bootstrap-results/noun-cost-joy-integration/README.md): Joy has 172 passing tests; Trident has 1197 passing tests and five existing ignores, and all 21 examples build. The real Z3 follow-up checks safe/unsafe outcomes with exit codes 0/1. Its first tool-path identity failure and corrected fresh-target replay remain retained. These local component checks leave the frozen native SH6 matrix unchanged.

Production structured public certificates are now integrated through Joy
[PR26](https://github.com/cyberia-to/joy/pull/26), with transport from
[PR25](https://github.com/cyberia-to/joy/pull/25), authenticated noun memory and
bounded derivations from Zheng PR40–43, and complete collection snapshots from
nox [PR26](https://github.com/cyberia-to/nox/pull/26). `prove-artifact` and
`verify-artifact` bind complete ART1/JOB1/RES1 inputs, results, computed
continuations, exact charge and expanded logical work. The verifier checks
Zheng derivations without executing nox or a compiler. Physical resource
observations remain explicitly unattested. Joy's
[source-bound integration audit](https://github.com/cyberia-to/joy/blob/6e0ec4d8440e2521df08f442d64f54e667044716/audit/structured-certificates/README.md)
records 215 passing workspace tests, one existing ignored census diagnostic,
zero failures/warnings, independently constructed semantic attacks and
fresh-process CLI checks. The separately reviewed
[actual compiler pilot gate](self-hosting/native-proof-pilots/README.md) now closes
SH7: five complete-C2 proofs, three byte-identical controls and twenty-four
rejected mutations, including canonical wrong output DAGs and rebound terminals.
The exact initial dynamic-apply probe proves and freshly verifies output 42
at charge 6 through the production structured route. The subsequent
[SH8 acceptance](self-hosting/bootstrap-results/whole-proof-final/acceptance/README.md)
covers both complete frozen-S1 self-builds.

The separate [six-platform production-profile gate](https://github.com/cyberia-to/trisha/blob/79f5ba880ddf1a7699aee60a316fe4e23ca27ca2/audit/native-proof-profile/hosted-36958147193/README.md)
passes on native macOS/Linux/Windows ARM64 and x64 through Trisha
[PR21](https://github.com/cyberia-to/trisha/pull/21). Its complete logs and
source inventories are retained; the frozen distribution archive remains a
separate gate with its original pins.

Coordinated distribution preparation is tracked in Trisha [PR18](https://github.com/cyberia-to/trisha/pull/18), with Joy's independent native smoke in [PR23](https://github.com/cyberia-to/joy/pull/23). The [source-bound Trisha audit](https://github.com/cyberia-to/trisha/blob/f512e87e197df477eb4b8d952886f5d3a38a1d2b/audit/native-soft3-release/README.md) retains 429 CPU test passes, six existing ignores and sixteen packaging guards. The first complete archive attempt failed on the standalone fixture helper's stale lockfile; its original failure is retained and the repair is committed. The next archive built successfully and exposed a stale smoke fixture that read private imported fields; the fixture now declares those fields public, with separate positive and privacy-rejection checks retained. The [third installed rehearsal](https://github.com/cyberia-to/trisha/blob/f512e87e197df477eb4b8d952886f5d3a38a1d2b/audit/native-soft3-release/installed-rehearsal/README.md) now passes on macOS ARM64 with actual Rust 1.89: four archive-built binaries, Joy’s 49 commands, complete coordinated smoke, all 47 proof-corpus cases, 28 process/file probes, all 133 baseline executions, deterministic repack and unpacked LSP. Its 570 retained raw files include the actual proof payloads and independent source/archive review. The [archived CPU and full baseline-proof gate](https://github.com/cyberia-to/trisha/blob/fef81df38e1e22fd0b20b2135408f2333c512f08/audit/native-soft3-release/cpu-proof-rehearsal/README.md) also passed on the same closure: Trident workspace 1231 passing tests with five existing ignores, Trisha 429 with six, and Joy 172 with none; all commands have zero failures and warnings. The separate 198-proof gate covers 99 positive and 34 rejection fixtures and all 43 baselines. It verifies proof payloads in process and retains their exact events and claims; the payloads themselves are transient. The [root integration checks](self-hosting/bootstrap-results/cpu-proof-integration/root-review.json) compare original logs, committed source and tool/binary identities. This records the earlier macOS ARM64 checkpoint; the final coordinated distribution gate is recorded below.

Portable C2 delivery is accepted in Trisha [PR19](https://github.com/cyberia-to/trisha/pull/19). The [actual kit evidence](https://github.com/cyberia-to/trisha/blob/c66c2da3da0d5b1da55f09be533a4d664d585bb2/audit/selfhost-kit/accepted/README.md) binds the real assembler to a fresh replay of the successful original 36-phase authority. Installed and unpacked Joy each compile the supplied example with the original native C2, execute atom 13 and preserve an existing output on guest rejection. All twelve outer commands pass, including production unpack/check and byte-identical package/repack. The retained kit is 4377450 bytes, SHA256 `a3052d95c3de6d622157988a8e74826b2f0140724a634298458c3d75f6b508bd`; its manifest is `4096a513d439adda55e62731f461ff0a7a72fa0c85d79292090be048b7f55ae8`. Default accepted-source preparation reproduces all 94 sources and exact C2/JOB/package/runtime/profile inputs of the retained complete C2(S1) → C3 run. That byte-bound compatibility check performs no second heavy self-build. Independent actual-delivery and retention reviews pass. The earlier historical-C2 rehearsal remains unchanged. This delivery validates the existing macOS ARM64 distribution binaries with the accepted kit; the final wider native distribution evidence is recorded below.


The final distribution gate is accepted in Trisha [PR24](https://github.com/cyberia-to/trisha/pull/24), merged into `release/0.4` as `95899e8f4fe32b5d7269d92b5e63ef429fbfafac`. The [source-bound package audit](https://github.com/cyberia-to/trisha/blob/95899e8f4fe32b5d7269d92b5e63ef429fbfafac/audit/final-host-ceiling-package/README.md) retains exact source archive `73b50ebd`, six native producers, complete package readback, and the actual six-by-six consumer matrix: 36 pairs, 72 corpus sets, 2664 cases and 138 installed deadline commands. The original producer/consumer invocations and revisions are retained with the audit. Its final Trisha executable received a fresh 198-proof gate. The separately bounded macOS 14 ARM consumer passed the exercised Joy/Trisha and compiler-kit paths; Intel macOS 14 and Windows 11 x64 remain unmeasured projections. Original failed transport attempts remain retained. This closes the distribution rehearsal gate for the frozen package inputs; SH8 and owner release promotion remain separate.

The [supplied-compiler delivery](self-hosting/bootstrap-results/supplied-compiler-integration/README.md) in Trident [PR113](https://github.com/cyberia-to/trident/pull/113) now completes the full C2(S1) → C3 through installed Joy with an empty toolchain command path. The byte-preparation helper verifies and copies all 94 frozen modules; the guest compiler performs all language work. Actual C3 bytes and every non-time execution field match the original reference. Its historical rehearsal-kit qualification is preserved. Both independent reviews, raw execution evidence and the clean isolated postcommit install are retained. The [six-platform transport validation](https://github.com/cyberia-to/trident/blob/70ca2ec038305f2518e771a4f3bdbef872268a5d/audit/self-hosting/bootstrap-results/source-transport-platforms/README.md) in [PR114](https://github.com/cyberia-to/trident/pull/114) subsequently passed on Python 3.13.15: the same 21 cases run in two modes on six native targets, with four explicit Linux filesystem-capability skips. Original CI bytes and independent source/receipt review are retained separately from SH6.

The [earlier full failure](self-hosting/full-bootstrap-compacting/README.md)
remains identified as a different source/runtime attempt. The
[fixed-point checker](self-hosting/fixed-point-job-binding/README.md) now binds
verified source copies to the actual executed JOB1 through canonical repacking.
[Extended supplied-compiler routing](self-hosting/extended-compiler-routing/README.md)
keeps independent Rust oracles separate and executes the selected C2 unchanged.
Tooling preparation alone closes neither corpus nor fixed-point acceptance.

| Gate | Status | Acceptance evidence / remaining work |
|---|---|---|
| [SH0](../reference/self-hosting.md#sh0-contract-and-compiler-subset) | Closed — contract gate | [Owner review and runtime evidence](self-hosting/native-runtime.md) |
| [SH1](../reference/self-hosting.md#sh1-native-bootstrap-foundation) | Closed — native bootstrap foundation | [Combined acceptance](self-hosting/native-compiler-profile.md) |
| [SH2](../reference/self-hosting.md#sh2-first-native-compiler) | Closed — bounded native arithmetic compiler | [Source, executed corpus and installed CLI acceptance](self-hosting/native-source-compiler.md) |
| [SH3](../reference/self-hosting.md#sh3-compiler-language-coverage) | Closed — reviewed frozen subset | [Complete construct map and executed positive/rejection evidence](self-hosting/compiler-feature-coverage.md) |
| [SH4](../reference/self-hosting.md#sh4-complete-project-and-runtime-scale) | Closed — current S1 measured scale | [Whole compiler, actual C2 source boundaries and complete-package invariance](self-hosting/lexer-bootstrap/README.md); original fixture limits retained |
| [SH5](../reference/self-hosting.md#sh5-first-self-compilation) | Closed — complete usable S1 C2 | [Actual supplied-C2 corpus](self-hosting/lexer-bootstrap/c2-corpus/README.md), all 547 observations and emitted-program checks |
| [SH6](../reference/self-hosting.md#sh6-reproducible-bootstrap) | Closed — frozen S1 reproducible native bootstrap | [Original CI and independently replayed artifacts](self-hosting/bootstrap-results/run-36382085561/README.md): twelve bootstrap repetitions, twenty-four actual corpora, six platforms; original aggregate passed |
| [SH7](../reference/self-hosting.md#sh7-native-proof-relation) | Closed — native public compiler profile on actual SH3/SH4 pilots | [Complete retained proofs, independent review and dynamic-apply check](self-hosting/native-proof-pilots/README.md); full witness and unattested host resources declared |
| [SH8](../reference/self-hosting.md#sh8-proved-self-compilation) | Closed — complete public-profile proofs for frozen S1 | [Final F5 acceptance](self-hosting/bootstrap-results/whole-proof-final/acceptance/README.md): both proofs and fresh verifiers, exact C2/C3, 547 corpus observations each, 23 distinct rejections and two original controls per generation; separate Trisha [PR25 retention](https://github.com/cyberia-to/trisha/pull/25), with earlier failed attempts preserved |

The [0.4 integration receipts](self-hosting/bootstrap-results/native-acceptance-integration/README.md) bind the reviewed source trees to the actual merge commits for Joy PR22/23, Trident PR112/113/114 and Trisha PR18/19. All seven are merged into `release/0.4`. The accepted kit and direct self-build instructions are available there; master and release publication remain outside this delivery.

## Next work, in order

- [ ] VB0 + RS0: freeze the complete Trident/Rs/Eidos/nox/Zheng prover/verifier/Joy
  surfaces and critical dependencies, required Rust/Trident implementation pairs,
  claims and trust assumptions; specify Eidos E0–E4 and the bounded VB1
  own nox seed/Eidos kernel experiment. Follow the
  [VB0–VB8 plan](../roadmap/verified-bootstrap.md) for dependent implementation.
  The [soft3 development and ceremony order](https://github.com/cyberia-to/soft3/blob/docs/verified-bootstrap-ceremony/docs/verified-bootstrap.md)
  retains the reviewed nox seed and adds the own Rs frontend/bootstrap route;
  all implementation acceptance remains open.

The completed SH work below retains its original evidence and scope.

- [x] Localize the body-stage budget barrier and reduce repeated imported-owner
  suffix/export-row work. Preserve historical failures and original fixture
  quotas; retain isolated measurements in the owning audit reports.
- [x] Complete SH3's construct inventory and review executed positive/rejection
  evidence against the actual frozen compiler closure.
- [x] Complete SH4's data-intensive stages within predeclared guest, cumulative
  allocation, resident, collection-work, frame and host-time limits. Retain
  4 KiB/64 KiB/full-closure and exact/one-below boundaries, dependency-identity
  changes, package reorder and checkout-directory reproducibility. Current S1
  succeeds under the declared profile; earlier failures remain in their receipts.
- [x] Freeze S, options/ABI and seed identity; obtain complete `C1(S) -> C2`
  through Joy. Save its actual independently loadable compiler artifact and
  complete measurements; no host-built replacement may complete this step.
- [x] Complete the unchanged independent positive/rejection corpus through
  actual S1 C2 and execute its emitted programs. Historical S0 completion is
  retained separately; the current compiler is supplied with `--compiler PATH`.
- [x] Complete current S1 `C2(S) -> C3`, exact canonical byte/particle and
  source/JOB1 checks.
- [x] Complete the independent actual S1 C3 regression corpus.
- [x] Repeat clean bootstrap with the [documented runner](../reference/self-hosting.md#sh6-reproducible-bootstrap) and execute the SH6
  six-target CI matrix: macOS, Linux glibc and Windows MSVC, each ARM64/x64.
  Retain source/seed/artifact identities and downloadable CI evidence.
- [x] Package the accepted actual C2 with its source/provenance manifest and
  direct Joy commands. Verify source compilation, execution and failed-output
  preservation using the installed and unpacked binaries.
- [x] Complete the native distribution rehearsal gates for frozen source archive
  `73b50ebd`: six native producers and consumers, fresh final proof gate, complete
  retained package readback and the bounded macOS 14 ARM observation in Trisha PR24.
- [x] Integrate the SH7 production public compiler profile, authenticated noun
  memory, bounded dynamic derivations, collection snapshots and Joy dispatch.
- [x] Accept SH7 using actual frozen compiler SH3/SH4 workloads, independently
  verified positive/adversarial certificates and measured resource limits.
- [x] Prove both complete frozen self-builds under SH8, with fresh-process
  verification, exact C2/C3 output comparison and adversarial binding checks;
  retain the [final acceptance](self-hosting/bootstrap-results/whole-proof-final/acceptance/README.md).

Resolve language/protocol choices in their owner contracts before dependent
implementation. PRs target `release/0.4`; master remains outside this delivery.

## Delivered increments

- [x] SH0.1 Inventory the actual compiler/library closure and map constructs to
  existing support, seed extensions and native rewrites. Record original modules
  and replacement owners, including the missing `.tri` nox generator.
  [Inventory and disposition](self-hosting/compiler-subset.md),
  [machine-readable closure](self-hosting/compiler-subset.json),
  [validation receipt](self-hosting/sh0-inventory-validation.json); regenerate/check
  with `cargo run --locked --example selfhost_inventory -- --root . --output audit/self-hosting/compiler-subset.json --check`.
- [x] SH0.2 Specify typed native collections, byte encoding, identity/equality,
  persistence, bounds and the minimal source API. Update `reference/language.md`,
  `reference/grammar.md` and `reference/nox.md` together with the inventory.
  [Data contract](../reference/self-hosting-data.md),
  [conformance evidence](self-hosting/native-data.md),
  [vectors](self-hosting/native-data-vectors.json). Source support is SH1 work.
- [x] SH0.3 Specify job/result and complete artifact codecs, canonical package
  order, module resolution and diagnostic/error behavior; add golden vectors.
  [Job contract](../reference/self-hosting-jobs.md),
  [protocol evidence](self-hosting/native-jobs.md),
  [complete vectors](self-hosting/job-vectors.json); nox codec merged in
  [PR16](https://github.com/cyberia-to/nox/pull/16) to its `release/0.4`.
- [x] SH0.4 Specify bounded loop/function execution and runtime resource policy
  jointly with nox/Joy; record the proof-relation consequences for Zheng.
- [x] SH0.5 Close the contract review against all four owners and choose the
  first SH1 implementation slice with exact commands and acceptance fixtures.
  [Runtime contract and owner review](self-hosting/native-runtime.md),
  [pinned validation](self-hosting/sh0-runtime-validation.json).
- [x] SH1 runtime slice: lifetime node allowance and sequential heap frames in
  nox [PR18](https://github.com/cyberia-to/nox/pull/18)/
  [PR19](https://github.com/cyberia-to/nox/pull/19); the same compact formula runs
  4097/5000 iterations with traced/run-only agreement. Raw source lowering is now
  covered by the separate reusable-control receipt below.
- [x] SH1 Joy raw transport slice: [PR5](https://github.com/cyberia-to/joy/pull/5),
  complete noun execution/publication with NoTrace;85 workspace tests passed.
- [x] SH1 Joy compiler-job slice: production JOB1/RES1 admission and binding,
  [PR9](https://github.com/cyberia-to/joy/pull/9), [receipt](https://github.com/cyberia-to/joy/blob/15202f42240ba923398908ee0db62d32f1d655f8/audit/self-hosting/compiler-jobs.md).
- [x] SH1 explicit compiler-profile seed export and source-guest JOB1/RES1 execution:
  [combined SH1 receipt](self-hosting/native-compiler-profile.md).
- [x] SH2 source-package driver: exact files to JOB1, with no host language stages;
  Joy [PR11](https://github.com/cyberia-to/joy/pull/11).
- [x] SH2 native arithmetic compiler: shared guest validation budget, UTF-8/lexer,
  iterative expression parser, native ART1/RES1 generation and executed corpus.
  [Acceptance](self-hosting/native-source-compiler.md).
- [x] SH3 seed parser: explicit expression-before-block parsing for if/for/match;
  [AST and cross-target execution evidence](self-hosting/block-expressions.md).
- [x] SH3 seed values: preserve zero-width structures and aggregates in shared
  TIR without renaming or consuming adjacent live values;
  [execution and boundary evidence](self-hosting/zero-width-values.md).
- [x] SH3 seed frontend: resolved halting-call semantics through return checking
  and both backends, with scoped callable/constant bindings;
  [execution and validation](self-hosting/resolved-halting.md).
- [x] SH3 native compiler locals: Field declarations, mutable assignments,
  stable runtime slots and lexical shadowing;
  [source/JOB execution evidence](self-hosting/native-compiler-locals.md).
- [x] SH3 native compiler control: Bool, equality, scoped if/else and early return;
  [source/JOB and installed CLI evidence](self-hosting/native-compiler-control.md).
- [x] SH3 native compiler functions: typed reusable calls, forward signatures and
  deterministic reachable code tables;
  [source/JOB and installed CLI evidence](self-hosting/native-compiler-functions.md).
- [x] SH4 bounded heap arena: nox allocation in place and explicit larger Joy
  pack/run allowance, preserving the current default and canonical output.
  [Execution and resource evidence](self-hosting/native-compiler-arena.md).
- [x] SH3 native scalars: U32, checked conversions, comparison and bit masking;
  [source/JOB and installed CLI evidence](self-hosting/native-compiler-scalars.md).
- [x] SH3 native literal-range loops: reusable bodies, scoped index and return flow;
  [execution evidence](self-hosting/native-compiler-loops.md).
- [x] SH3 native Noun values and structured raw entry;
  [execution evidence](self-hosting/native-compiler-nouns.md).
- [x] SH3 canonical type descriptors through AST, bindings, signatures and bodies;
  [execution and boundary evidence](self-hosting/native-compiler-types.md).
- [x] SH3 native Digest identity, whole-value transport and checked read indexing;
  [source/JOB and installed evidence](self-hosting/native-compiler-digest.md).
- [x] SH3 source tuples and ordered destructuring;
  [native/installed execution evidence](self-hosting/native-compiler-tuples.md).
- [x] SH3 module-owned nominal structs, constructors and field reads;
  [source and execution evidence](self-hosting/native-compiler-records.md).
- [x] SH3 persistent nested field writes and whole-value snapshots;
  [execution and boundary evidence](self-hosting/native-compiler-record-writes.md).
- [x] SH3 fixed Field arrays, exact types and checked read indexing;
  [source and resource evidence](self-hosting/native-compiler-arrays.md).
- [x] SH3 shared typed seed constants and final owner bindings;
  [native/Triton validation](self-hosting/typed-constants.md).
- [x] SH3 guest constants, final typed aliases and raw literal provenance;
  [source/JOB and installed evidence](self-hosting/native-compiler-constants.md).
- [x] SH3 assertions and resolved halting;
  [source/JOB and installed evidence](self-hosting/native-compiler-assertions.md).
- [x] SH3 function attributes, declaration-owned purity and contract metadata;
  [source/JOB and installed evidence](self-hosting/native-compiler-attributes.md).
- [x] SH3 seed final-callable ownership, exact generic call sites and shared ABI;
  [cross-backend and installed evidence](self-hosting/final-callable-exports.md).
- [x] SH3 seed explicit imports, canonical owners and opaque nominal layouts;
  [cross-backend and installed evidence](self-hosting/explicit-imports.md).
- [x] SH3 guest package handles, bounded lookup and cached validated sources;
  [component and complete installed corpus](self-hosting/guest-package.md).
- [x] SH3 seed complete qualified paths and visible check diagnostics;
  [cross-warrior rejection and execution](self-hosting/strict-module-paths.md).
- [x] SH3 guest graph components: cached reached sources, repeated direct uses,
  bounded discovery and seed ordering; [component evidence](self-hosting/guest-module-graph.md).
- [x] SH3 qualified-name and frozen constant-binding components;
  [owner/provenance and installed component evidence](self-hosting/constant-bindings.md).
- [x] SH3 guest constant imports through real C1: final public values, direct
  aliases and original owner diagnostics; [execution evidence](self-hosting/guest-constant-linking.md).
- [x] SH3 direct ordinary function imports through C1, with private helpers,
  final visibility and owner-specific diagnostics; [execution evidence](self-hosting/guest-function-imports.md).
- [x] SH3 seed stable nominal layouts across repeated declarations;
  [isolated and integrated evidence](self-hosting/nominal-bindings-combined.md).
- [x] SH3 guest nominal imports: ordered public type/constructor aliases, owner-preserving
  opaque returns and aggregate dependency signatures; [execution evidence](self-hosting/guest-type-imports.md).
  The delivery consumes deterministic codec prerequisites in Trisha/Joy and targets `release/0.4`.
- [x] Explicit generated 1/1 compiler profiles: exact requested metadata, final
  structured entry validation, complete JOB1-bound RES1 and executed generated programs;
  [combined acceptance](self-hosting/generated-compiler-profiles.md).
- [x] Joy explicit compiler arena and deadline: accepted [PR13](https://github.com/cyberia-to/joy/pull/13),
  [combined boundary evidence](https://github.com/cyberia-to/joy/blob/15202f42240ba923398908ee0db62d32f1d655f8/audit/explicit-compiler-arena-combined.md).
  That delivery preserved defaults and its reduction ceiling; complete source
  scale was still open at that checkpoint and is now accepted for frozen S1.
- [x] SH3 exact intrinsic declarations, ABI validation and final callable identity;
  [installed acceptance and preserved boundaries](self-hosting/guest-intrinsics.md).
- [x] SH4 indexed-read increment: original 61–64-bit record writes fit the same
  786432-node arena; all prior successful ART1 identities are preserved.
  [Stable installed acceptance](self-hosting/native-indexed-reads.md).
- [x] SH4 reproducible complete native source inventory: parser-owned entry/import
  closure with stale-receipt rejection; [acceptance](self-hosting/native-source-inventory.md).
- [x] SH4 independent source-byte capacity and exact full-closure probe:
  complete scans and original spans beyond 4096 bytes, preserved internal IDs;
  [acceptance and runtime boundary](self-hosting/source-capacity.md).
- [x] SH1 native Noun/raw source slice: [implementation and execution evidence](self-hosting/native-noun.md).
- [x] SH1 reusable raw calls/loops and dynamic indexing: [execution receipt](self-hosting/native-control.md),
  [design](self-hosting/native-control-design.md). Flat bundle lowering remains legacy.
- [x] SH1 native Seq/Bytes source libraries: [validation and installed CLI evidence](self-hosting/native-collections.md).
- [x] Native wrapper prerequisite: nominal module ownership, enforced private
  fields and duplicate-owner rejection. [Validation](self-hosting/native-wrapper-privacy.md).

- [x] SH4 packed byte/path traversal and stable bounded function ordering;
  [byte validation](self-hosting/byte-discard.md),
  [job path reads](self-hosting/job-word.md),
  [ordering measurements](self-hosting/function-sort-scale.md) and
  [frozen complete-source frontier](self-hosting/full-bootstrap-frontier.md).
- [x] SH4 bounded nox compaction and explicit Joy worker policy;
  [runtime accounting/root review](https://github.com/cyberia-to/nox/blob/2f09ca3c3f18ae470365310cca8db5208eda75c6/audit/sequential-compaction/README.md)
  and [installed Joy boundaries](https://github.com/cyberia-to/joy/blob/15202f42240ba923398908ee0db62d32f1d655f8/audit/self-hosting-compaction/README.md).
- [x] SH4 exact discovery component with canonical output and successful gas
  preserved under compaction. Complete compilation was still open at that
  checkpoint; the S1 SH4 acceptance above supplies the later complete result.
- [x] Supplied compiler routing (`8467b2c`) and retained fixed-point checks
  (`e306dc4`); [routing evidence](self-hosting/compiler-routing/README.md) and
  [checker evidence](self-hosting/fixed-point-checker/README.md). Partial real
  probes and synthetic checker tests do not close SH5/SH6.

## Known blockers and ownership

| Blocker | Owner / first gate | Current evidence |
|---|---|---|
| Verified bootstrap trust and dual implementation closure | soft3 + Trident/Eidos/nox/Zheng/Joy / VB0 | Open — [separate implementation plan](../roadmap/verified-bootstrap.md); no accepted soft3 root, DDC or complete canonical-stack claim |
| Bounded long-trivia scanning | Trident / SH4 — resolved in S1 | Actual supplied C2 accepts the valid 64 KiB comment at 4324 frames; all six source-boundary cases and complete-package invariance pass. Original S0 failure remains retained |
| Cross-platform bootstrap | Trident + Joy / SH6 — resolved for frozen S1 | All twelve fresh bootstrap repetitions, twenty-four actual corpus jobs, original aggregate and independent byte-exact replay pass. Original one-hour worker and 330-minute CI failures remain historical evidence |
| Complete self-build proof acceptance | Zheng + Joy / SH8 — resolved for frozen S1 | [Final F5 acceptance](self-hosting/bootstrap-results/whole-proof-final/acceptance/README.md) binds both complete proofs, fresh verification, exact C2/C3 and corpus results, and all 23 distinct rejections plus two original controls per generation; complete byte retention is accepted in Trisha PR25 |

## Baseline evidence

The original RAM prototype and its AST-to-nox gaps remain historical findings
in the [prototype probes](self-hosting-2026-09-23/probes.json). The active native
closure uses the delivered Noun collections and native code generator; its
accepted frozen-S1 execution and proof gates are listed above. Further language
coverage requires its own declared workloads.

- [2026-09-23 soft3 assessment](soft3-self-compilation-readiness-2026-09-23.md):
  native target, runtime/backend/proof distinctions and inspected source paths.
- [2026-09-23 prototype assessment](self-hosting-readiness-2026-09-23.md): seven
  modules check with the Rust compiler; narrow Triton prototype observations.
  Its historical Triton-first recommendation is superseded.
- Verified release bundle: Trident0.3.0 / Joy0.5.0. Its identity is retained
  in [the initial probe receipt](self-hosting-2026-09-23/probes.json).
  Compiler-scale native runs and full bootstrap were not performed.

The delivered helpers and accepted six-platform bootstrap are linked above.
The following original estimate is retained as historical planning context.

## Planning estimate

The initial native self-hosting envelope is 40–70 three-hour sessions
(120–210 focused hours), with low confidence. It includes the native contract,
Rust seed extensions, nox/Joy runtime transport, compiler port and bootstrap
hardening. It is not measured remaining work or a promise. The concrete
SH6 six-target matrix is required regardless of this initial estimate.

The historical S0 `b991d901` C1(S) build measured 1471089704 worker microseconds
on the reference host, with bounded resident memory; the exact command and
revisions are retained in the [body-scale receipt](self-hosting/body-scale/README.md).
The current S1 build is measured separately above. These observations supply
baselines for subsequent full-build runs, not wall-time estimates for corpus,
clean builds or CI. SH6, SH7, frozen-S1 SH8 and the frozen distribution rehearsal
are accepted; the original broad estimate remains historical.

The [original local public-profile proof runs](self-hosting/bootstrap-results/whole-proof-final/acceptance/README.md)
took 6678.02309654 seconds for C1(S1) → C2 and 5955.15907412 seconds for
C2(S1) → C3. Their separate fresh-process verifiers took 1874.25285621 and
1646.03263471 seconds, respectively. The [summary](self-hosting/bootstrap-results/whole-proof-final/acceptance/packet/summary.json)
binds these measured command durations to the original proofs and final F5
acceptance. The adversarial completion and transport histories are retained
separately. Elapsed time and sampled memory remain host observations.
Private/succinct compilation and semantic preservation remain separate claims.

## Delivery history and retained receipts

This index condenses repeated delivered-history prose. Each linked receipt keeps
its original commands, revisions, counts, failed runs, artifact comparisons and
quota changes. Acceptance is limited to that receipt's source and binary. Later
component or local gates do not silently rerun earlier installed corpora. Formal
audit UNKNOWN verdicts remain UNKNOWN. SH6 acceptance moves the Noun layer
to 64K in the current roadmap; historical receipt temperatures remain unchanged.

| Date | Delivery / revision | Retained evidence and qualification |
|---|---|---|
| 2026-09-23 | Starting prototype/native assessments | [Prototype](self-hosting-readiness-2026-09-23.md), [soft3](soft3-self-compilation-readiness-2026-09-23.md), [raw probes](self-hosting-2026-09-23/probes.json); no full bootstrap |
| 2026-09-23 | SH0 inventory/data/jobs/runtime | [Inventory](self-hosting/sh0-inventory-validation.json), [data](self-hosting/native-data.md), [jobs](self-hosting/native-jobs.md), [runtime review](self-hosting/sh0-runtime-validation.json); contracts closed |
| 2026-09-24 | SH1 reusable raw control and native collections | [Control](self-hosting/sh1-control-validation.json), [collections](self-hosting/sh1-collections-validation.json); exact output and allowance boundaries |
| 2026-09-24 | SH1 compiler profile | [Combined acceptance](self-hosting/native-compiler-profile.md); source-guest JOB1/RES1 execution closes SH1 |
| 2026-09-24 | Shared validation budget `4a9a283`; Joy exact files `a3dd4c5` | [Combined commands](self-hosting/shared-validation-budget.json); formal collection analysis UNKNOWN |
| 2026-09-24 | SH2 `7684fd7` | [Native compiler](self-hosting/sh2-native-compiler-validation.json); fresh packages compile inside nox and their emitted programs execute |
| 2026-09-24 | Seed expression-before-block `fd64b73` | [Cross-target acceptance](self-hosting/sh3-block-expressions-validation.json) |
| 2026-09-24 | Zero-width/array extent repair `f11a432` | [Execution and bounds](self-hosting/sh3-zero-width-validation.json) |
| 2026-09-24 | Resolved halting and scoped final bindings | [Acceptance](self-hosting/resolved-halting.md) |
| 2026-09-24 | Native locals/control/functions `5339030` | [Locals](self-hosting/sh3-native-locals-validation.json), [control](self-hosting/native-compiler-control.md), [functions](self-hosting/native-compiler-functions.md); cost and deep-call/default-arena failures retained |
| 2026-09-24 | Heap arena: nox `568ac16`, Joy `820041b`, runner `1e08ded` | [Acceptance](self-hosting/native-compiler-arena.md); original whitespace boundary retained |
| 2026-09-24 | U32/scalars `fb2bcd9`, diagnostic refinement `6a1abc2` | [Acceptance](self-hosting/native-compiler-scalars.md); runtime conversion traps distinct from compile diagnostics |
| 2026-09-24 | Literal-range loops `7e5a274` | [Acceptance](self-hosting/native-compiler-loops.md); scope/early-return/exact limits |
| 2026-09-24 | Noun/structured entry `4dbd03e`, harness `c5d1901` | [Acceptance](self-hosting/native-compiler-nouns.md); complete output and runtime projection traps |
| 2026-09-24 | Aggregate descriptors `adbb00e` | [Acceptance](self-hosting/native-compiler-types.md); sharing/depth/arity bounds |
| 2026-09-24 | Digest `dbf3c13` | [Acceptance](self-hosting/native-compiler-digest.md); identity words and checked reads |
| 2026-09-24 | Tuples `850525c` | [Acceptance](self-hosting/sh3-native-tuples-validation.json); old default arena boundary and earlier artifacts preserved |
| 2026-09-24 | Exactly-once seed field initialization `ac2481d` | [Acceptance](self-hosting/unique-struct-initializers-validation.json); duplicate fields reject |
| 2026-09-24 | Nominal identity/layout `1e36ceb` | [Acceptance](self-hosting/native-compiler-nominal-validation.json); owner-based visibility |
| 2026-09-24 | Source records `60ea10e` | [Acceptance](self-hosting/native-compiler-records-validation.json); original long-name arena failure retained |
| 2026-09-24 | Persistent record writes `324a015` | [Acceptance](self-hosting/native-compiler-record-writes-validation.json); original wide-source generation failures retained |
| 2026-09-25 | Fixed Field arrays `a427531` | [Acceptance](self-hosting/native-compiler-arrays-validation.json); separate planner allowance never replaces source acceptance |
| 2026-09-25 | Typed seed constants `2f6b0ef` | [Acceptance](self-hosting/typed-constants-validation.json); unchanged C1, earlier guest corpus reused explicitly |
| 2026-09-25 | Retained packages `473d20c` | [Acceptance](self-hosting/guest-package-validation.json); record widths 61/62 recover, wider failures retained; host deadline selection explicit |
| 2026-09-25 | Constant imports `17685e1` / harness `655ac69` | [Acceptance](self-hosting/guest-constant-linking-validation.json); long-name and widths 61–64 recover within original caps, width 65 still fails at this revision |
| 2026-09-25 | Direct function imports `4acc730` | [Acceptance](self-hosting/guest-function-imports-validation.json); final visibility/owners, previous artifacts preserved |
| 2026-09-25 | Seed nominal repair `5d06645`, combined `285681d` | [Combined acceptance](self-hosting/nominal-bindings-combined-validation.json); unchanged guest/runtime corpus reused explicitly |
| 2026-09-26 | Guest nominal imports `7c1701c`, harness `a166c8d` | [Acceptance](self-hosting/guest-type-imports-validation.json); deterministic codec prerequisite, clean rebuilds and installed corpora rerun; earlier derive drift retained |
| 2026-09-27 | Generated profiles `3cfaf0c`, Joy `a15adb7` | [Acceptance](self-hosting/generated-compiler-profiles.md); literal compiler emitted by C1 compiles fresh jobs, full self-source still open |
| 2026-09-27 | Exact intrinsics `be9676d` | [Acceptance](self-hosting/guest-intrinsics.md); original width-65 case recovers, exact resource recalibration and earlier failures retained; full closure returns capacity diagnostic |
| 2026-09-27 | Independent source bytes `40a86de` | [Acceptance](self-hosting/source-capacity.md); exact earlier closure admitted, 100M reduction failure before RES1; 64 KiB fixed-arena boundary retained |
| 2026-09-27 | Byte/path/sort scaling through `713f457` | [Frontier](self-hosting/full-bootstrap-frontier.md); exact discovery succeeds at larger explicit limits, append-only all-body arena failure and rejected lexer regression retained |
| 2026-09-27 | Opt-in nox/Joy compaction | [Nox](https://github.com/cyberia-to/nox/blob/2f09ca3c3f18ae470365310cca8db5208eda75c6/audit/sequential-compaction/README.md), [Joy](https://github.com/cyberia-to/joy/blob/15202f42240ba923398908ee0db62d32f1d655f8/audit/self-hosting-compaction/README.md); merged components, whole self-build remains open |
| 2026-09-27 | Supplied-compiler runner `8467b2c`, checker `e306dc4` | [Routing](self-hosting/compiler-routing/README.md), [checker](self-hosting/fixed-point-checker/README.md); partial real probes, no C2/C3 corpus or fixed-point acceptance |
| 2026-09-27 | Full compacting C1(S), retained at `90ac882` | [Failure](self-hosting/full-bootstrap-compacting/README.md); execution-budget rejection, bounded resident storage, no C2 |
| 2026-09-27 | Alias/export lookup through `b991d901`, explicit Joy `2878f4b` | [Complete C1(S)](self-hosting/body-scale/README.md) produces actual C2; [SH3 feature map](self-hosting/compiler-feature-coverage.md) reviewed, corpus/fixed point pending |

Historical scalar/loop implementation plans remain linked for continuity:
[scalars](../.claude/plans/native-compiler-scalars.md),
[loops](../.claude/plans/native-compiler-loops.md). The delivered increments and
current pending list supersede their old sequencing. Initial integration began
from `360b737e073ca2f969ab0c78460b4228bcac7b78`; accepted nominal-layout/import
worktrees and sibling pins remain identified in their receipts.

### Soft3 runtime ownership correction

Joy `06aac01` removed its private Trisha/Triton adapter and dependency closure.
Trident `7b1d4c0` isolated foreign target resources behind `external-targets`;
Joy disables that feature and Trisha `aa25e32` enables it explicitly. Native
compiler source and profiles stayed unchanged. The
[owner receipt](https://github.com/cyberia-to/joy/blob/15202f42240ba923398908ee0db62d32f1d655f8/audit/soft3-only/README.md) preserves compatibility,
installed profile acceptance and unchanged C1/artifact identities. At that
revision secret execution and public Zheng certificates remained supported,
while private/zero-knowledge proving was unavailable. Native compiler proof
acceptance remains governed by SH7/SH8 and the current owner contracts.

For each gate update, retain exact source/binary identities, passed and failed
conditions, next action and any changed estimate. Preserve failed receipts;
close a gate only when its complete reference acceptance is satisfied.
