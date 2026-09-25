# Self-hosting on soft3 — progress ledger

Updated: 2026-09-25. Working contract:
[reference/self-hosting.md](../reference/self-hosting.md).
This ledger is the current execution checklist. Dated assessments and receipts
retain their original observations; they are not substituted for gate evidence.

## Current position

**Next: SH3 compiler language coverage and SH4 compiler-scale resources.** SH0's compiler
inventory, data/job formats and runtime/control-flow contract are reviewed and
specified. Nox's complete codec, lifetime arena allowance and sequential heap
executor and Joy's structured raw run are in `release/0.4`.
Native source Noun and source→ART1→Joy execution are implemented and tested
in the current delivery branches; see [evidence](self-hosting/native-noun.md).
Reusable raw source calls/loops and checked dynamic array indexing now have
[execution acceptance](self-hosting/native-control.md). Native Seq/Bytes have
[execution acceptance](self-hosting/native-collections.md). Production compiler
JOB1/RES1 admission is delivered in Joy [PR9](https://github.com/cyberia-to/joy/pull/9);
[receipt](../../joy/audit/self-hosting/compiler-jobs.md). Explicit compiler-profile
seed export and source-guest execution have [acceptance](self-hosting/native-compiler-profile.md).
SH1 and SH2 are closed. The native source compiler has
[SH2 acceptance](self-hosting/native-source-compiler.md): fresh source packages
become separately executed nox programs through Joy.
No self-compilation or compiler execution proof is claimed.
The [bounded heap-arena increment](self-hosting/native-compiler-arena.md) admits
larger native compiler jobs through explicit Joy quotas. Complete source-scale
allocation remains open, including a valid 4096-byte whitespace workload.

Integration: `release/0.4`. First delivery: `feat/0.4-sh0-inventory`, based on
`360b737e073ca2f969ab0c78460b4228bcac7b78`, with pinned release sibling checkouts.
Active isolated checkout: `~/cyber/.worktrees/selfhost-0.4-constants/trident`.
Dependencies use clean worktrees at the pinned receipt revisions.
PRs target the integration branch; master stays unchanged until 0.4 acceptance.

| Gate | Status | Missing acceptance evidence |
|---|---|---|
| [SH0](../reference/self-hosting.md#sh0-contract-and-compiler-subset) | Closed — contract gate | [Owner review and runtime evidence](self-hosting/native-runtime.md) |
| [SH1](../reference/self-hosting.md#sh1-native-bootstrap-foundation) | Closed — native bootstrap foundation | [Combined acceptance](self-hosting/native-compiler-profile.md) |
| [SH2](../reference/self-hosting.md#sh2-first-native-compiler) | Closed — bounded native arithmetic compiler | [Source, executable corpus and installed CLI acceptance](self-hosting/native-source-compiler.md) |
| [SH3](../reference/self-hosting.md#sh3-compiler-language-coverage) | Open — foundation accepted | Whole compiler subset and executed differential/rejection corpus |
| [SH4](../reference/self-hosting.md#sh4-complete-project-and-runtime-scale) | Open — bounded heap-arena increment accepted | Real closure, complete compiler-scale memory/runtime and boundary receipts |
| [SH5](../reference/self-hosting.md#sh5-first-self-compilation) | Open — needs SH3/SH4 | C1 compiles all of S into usable C2 on nox |
| [SH6](../reference/self-hosting.md#sh6-reproducible-bootstrap) | Open — needs SH5 | C2/C3 fixed point, regression corpus and six-platform CI |
| [SH7](../reference/self-hosting.md#sh7-native-proof-relation) | Open — design after SH0 | Production native relation and compiler pilot proofs; final gate uses SH3/SH4 |
| [SH8](../reference/self-hosting.md#sh8-proved-self-compilation) | Open — needs SH6/SH7 | Native proofs of both complete self-builds and adversarial verification |

## Next work, in order

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
  [PR9](https://github.com/cyberia-to/joy/pull/9), [receipt](../../joy/audit/self-hosting/compiler-jobs.md).
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
- [ ] SH3 true guest imports and complete compiler closure coverage;
  [implementation order](../.claude/plans/native-imports.md).
- [ ] SH4 source closure and allocation scale: repair the valid 4096-byte
  whitespace workload's lifetime arena boundary and measure full compiler closure.
- [x] SH1 native Noun/raw source slice: [implementation and execution evidence](self-hosting/native-noun.md).
- [x] SH1 reusable raw calls/loops and dynamic indexing: [execution receipt](self-hosting/native-control.md),
  [design](self-hosting/native-control-design.md). Flat bundle lowering remains legacy.
- [x] SH1 native Seq/Bytes source libraries: [validation and installed CLI evidence](self-hosting/native-collections.md).
- [x] Native wrapper prerequisite: nominal module ownership, enforced private
  fields and duplicate-owner rejection. [Validation](self-hosting/native-wrapper-privacy.md).

Resolve protocol or language choices explicitly in their owner contracts before
dependent implementation. Keep the next executable task and its dependencies
visible here as those decisions land.

## Known blockers and ownership

| Blocker | Owner / first gate | Baseline |
|---|---|---|
| RAM-based compiler structures; migrate onto delivered native collections | Trident / SH0–SH1 | `.tri` compiler modules and current AST/type system |
| Full compiler arena/memory scale still unmeasured | nox + Joy + Trident / SH1, SH4 | Explicit heap arena admits larger real compiler jobs; valid 4096-byte whitespace workload and full closure remain open |
| Full native compiler language coverage | Trident / SH3 | Scalar/control/call/loop/Noun foundations, Digest, tuples, nominal records/writes, arrays and [typed constants](self-hosting/native-compiler-constants.md) are accepted. [Assertions](self-hosting/native-compiler-assertions.md) and [attributes](self-hosting/native-compiler-attributes.md) are accepted; [final callable ownership](self-hosting/final-callable-exports.md) is accepted; [seed explicit imports](self-hosting/explicit-imports.md) are accepted; true guest imports remain open |
| Prototype semantic errors and missing `.tri` AST-to-nox generator | Trident / SH2–SH3 | [prototype probes](self-hosting-2026-09-23/probes.json) |
| No complete source build or fixed-point runner | Trident + Joy / SH4–SH6 | [starting assessment](soft3-self-compilation-readiness-2026-09-23.md) |
| Native dynamic apply runs but production proof rejects it | Zheng + Joy / SH7–SH8 | [run/prove receipt](self-hosting-2026-09-23/soft3-runtime-probes.json) |

## Baseline evidence

- [2026-09-23 soft3 assessment](soft3-self-compilation-readiness-2026-09-23.md):
  native target, runtime/backend/proof distinctions and inspected source paths.
- [2026-09-23 prototype assessment](self-hosting-readiness-2026-09-23.md): seven
  modules check with the Rust compiler; narrow Triton prototype observations.
  Its historical Triton-first recommendation is superseded.
- Verified release bundle: Trident0.3.0 / Joy0.5.0. Its identity is retained
  in [the initial probe receipt](self-hosting-2026-09-23/probes.json).
  Compiler-scale native runs and full bootstrap were not performed.

No future runner names or successful CI runs are claimed here. Add actual
commands, artifact locations and gate receipts as implementations land.

## Planning estimate

The initial native self-hosting envelope is **40–70 three-hour sessions**
(120–210 focused hours), with low confidence. It includes the native contract,
Rust seed extensions, nox/Joy runtime transport, compiler port and bootstrap
hardening. It is not measured remaining work or a promise. The concrete
SH6 six-target matrix is required regardless of this initial estimate.

SH2 now removes uncertainty about the complete native compilation pipeline.
Its lifetime-arena boundary confirms that compiler-scale data handling remains
substantial work; the original broad estimate is not a measured remaining-work
estimate. Re-estimate from the SH3 feature inventory and SH4 complete workload.
Zheng compiler-scale proving (SH7/SH8) needs a relation design and measurements
before a credible effort bound. A public native profile may close those gates;
private/succinct compilation and semantic preservation remain separate claims.

## Session log

| Date | Work | Result / next action |
|---|---|---|
| 2026-09-23 | Compiler prototype and native soft3 assessment | Baseline probes recorded; self-compilation remains open |
| 2026-09-23 | Working SH0–SH8 contract, dependencies, owners and acceptance | Documentation only; start SH0.1 inventory |
| 2026-09-23 | SH0.1 AST inventory tool and migration disposition on the 0.4 delivery branch | 10 modules, 985 functions, 321415 bytes; proceed to SH0.2 native data contract |
| 2026-09-23 | SH0.2 native data contract and conformance on `feat/0.4-sh0-native-data` | 18 tests, including six actual nox probes; proceed to SH0.3 job/result and artifact transport |
| 2026-09-23 | SH0.3 job/result contract, bounded reference admission and complete canonical transport | Nine protocol tests +18 data tests; nox's codec passed178 workspace tests; proceed to SH0.4 runtime/control flow |

For a gate update, record its receipt link, exact owner commits/patches, passed
and failed conditions, next action and any changed estimate. Preserve previous
failed receipts. Mark completed substeps independently; close the gate only
when the complete reference acceptance is satisfied.

2026-09-23 continuation: SH0.4/SH0.5 contract review closed; nox runtime
PR18/PR19 merged into release/0.4. Joy raw transport PR5 also merged; native source data is next.

2026-09-24 continuation: reusable raw-native calls/loops, checked dynamic array
indexing and balanced frames passed Trident, Joy CLI and Trisha compatibility
gates. [Pinned commands and observations](self-hosting/sh1-control-validation.json).
Continue with canonical source Seq/Bytes, then production compiler-job admission.

2026-09-24 continuation: native source Seq/Bytes passed exact canonical output,
validation allowance and installed Joy runtime quota boundaries.
[Collections receipt](self-hosting/sh1-collections-validation.json).
Continue with production compiler JOB1/RES1 admission and binding in Joy.

2026-09-24 continuation: explicit compiler-profile source export and real source-guest
JOB1/RES1 execution accepted. SH1 closed by the [combined receipt](self-hosting/native-compiler-profile.md).
Proceed with SH2 source packages and arithmetic compilation inside nox.

2026-09-24 continuation: Joy [PR11](https://github.com/cyberia-to/joy/pull/11)
landed exact-file source packaging on `release/0.4` at
`a3dd4c5c7c2f870f6632deace5b137a141796173`. The native collection APIs now
return remaining validation visits for a shared guest pass at Trident
`4a9a2838b6336fdc0ba9126b12b88e1a8ee43340`. The chained Seq/Bytes execution
matches the independent model at exact and insufficient allowances, including
chunk boundaries. [Commands and revisions](self-hosting/shared-validation-budget.json)
record 876 Trident package tests plus 34 silicon tests, 120 Joy tests and
133/43 Trisha fixture/baseline checks with unchanged result/cycle rows.
Formal collection analysis remains UNKNOWN. The SH2 lexical, grammar and
diagnostic contract is now explicit; actual guest source compilation is next.

2026-09-24 continuation: SH2 accepted at source
`7684fd7e67d3d6610c42553088767844af379f36`. The fixed native compiler compiles
fresh source packages and Joy executes its emitted programs. Exact output,
diagnostic, shared-validation and execution-limit cases passed; the source
admission arena limit remains explicit. [Pinned evidence](self-hosting/sh2-native-compiler-validation.json).
Continue with SH3 frontend repairs and whole-compiler language coverage, then
SH4 resource work before attempting C2.

2026-09-24 continuation: expression-before-block parsing accepted at Trident
`fd64b73f094ff6a611f8473364f7758484c02fd9`, with Trisha companion
`03b6f9dda71f08000265c929ea55dc32642b050f`.
[Commands and evidence](self-hosting/sh3-block-expressions-validation.json).
Continue with the observed zero-width TIR value defect and resolved halting
semantics, then complete native compiler coverage. SH3 remains open.

2026-09-24 continuation: zero-width value and checked array-extent repair accepted
at Trident `f11a4320ed2b42be5f4a32789c06c5c5fe4fa1bd`, with Trisha
`3140d30ff2a9d4ba57641148e3f227148577cc0f`.
[Exact commands and evidence](self-hosting/sh3-zero-width-validation.json).
Continue with resolved halting-call semantics through type checking and lowering,
then whole native compiler coverage. SH3 remains open.

2026-09-24 continuation: resolved assertion failure, continuing branch values,
per-owner callable/constant bindings and local root shadowing passed the
[halting acceptance](self-hosting/resolved-halting.md). Continue with native
Field locals/assignments and actual runtime slots inside the guest compiler.

2026-09-24 continuation: native Field locals, mutation and lexical shadowing
accepted through full source/JOB/ART1 execution. [Pinned validation](self-hosting/sh3-native-locals-validation.json).
Continue with native Bool/equality/control flow; compiler-scale arena remains
open and is reproduced by the larger assignment workload.

2026-09-24 continuation: native typed functions accepted at Trident
`5339030b66c8e9835d79f175ef2ac5eb98d1e75e`. The installed Joy corpus covers
forward/nested calls, fresh frames, Unit and final callable bindings;
[commands, measurements and limits](self-hosting/native-compiler-functions.md).
Continue with an in-place heap arena in nox and explicit larger structured
pack/run allowance in Joy. Compiler cost increased and deep-call/default-arena
failures remain recorded; full SH3/SH4 and self-build are open.

2026-09-24 continuation: bounded heap arena accepted at nox `568ac16`,
Joy `820041b` and Trident runner `1e08ded`.
[Measured acceptance](self-hosting/native-compiler-arena.md) preserves canonical
artifacts and exact job quotas while admitting larger real native compilations.
The valid 4096-byte whitespace workload remains an explicit SH4 boundary.
Continue with [native scalar coverage](../.claude/plans/native-compiler-scalars.md).

2026-09-24 continuation: native U32 and checked builtin operations accepted at
Trident `fb2bcd9`, with diagnostic acceptance refinement `6a1abc2`.
[Source and installed execution evidence](self-hosting/native-compiler-scalars.md)
preserves previous ART1 identities, separates compilation from runtime conversion
traps, and records increased compiler cost. Continue with
[literal-range loops](../.claude/plans/native-compiler-loops.md); complete SH3/SH4,
C2/C3, six-platform release and native compiler proofs remain open.

2026-09-24 continuation: native reusable literal-range loops accepted at Trident
`7e5a27461b0cbad23db15c249734c3ed9095adf8`.
[Source/JOB tests and installed execution](self-hosting/native-compiler-loops.md)
cover nested scope, early return, raw U32 bounds and exact resource limits,
while preserving prior ART1 identities. Continue with native Noun values and
structured raw entry; full SH3/SH4 and C2/C3 remain open.

2026-09-24 continuation: native Noun values and structured entry accepted at
Trident `4dbd03e95defbff53c27d453ca6ca7e15bf292c7`, with CLI diagnostic assertions
refined at `c5d1901f5be44caf097cc33e7e09132538ce7fa7`.
[Full JOB/ART1 and installed acceptance](self-hosting/native-compiler-nouns.md)
checks complete nested output, runtime projection traps and exact resource bounds.
Continue with canonical aggregate descriptors, Digest and tuples. Full SH3/SH4,
C2/C3 and platform acceptance remain open.

2026-09-24 continuation: canonical aggregate descriptor foundation accepted at
Trident `adbb00e`. [Pinned validation](self-hosting/native-compiler-types.md)
checks logical sharing/depth/arity bounds and retains every earlier positive
ART1 identity. Continue with source Digest identity/indexing and tuples; full
SH3/SH4 and C2/C3 remain open.

2026-09-24 continuation: native Digest identity and checked component reads
accepted at Trident `dbf3c13`. [Pinned validation](self-hosting/native-compiler-digest.md)
compares every identity word with input particle bytes and covers runtime traps,
ordering and exact quotas. Earlier positive ART1 identities remain unchanged.
Continue with tuple annotations/values and destructuring; full SH3/SH4, C2/C3
and six-platform acceptance remain open.


2026-09-24 continuation: native tuple annotations/values and ordered
flat destructuring accepted at `850525c`. [Pinned evidence](self-hosting/sh3-native-tuples-validation.json)
records 1017 Trident, 122 Joy and 380 Trisha passing tests, four existing
ignored Trisha cases, zero Rust warnings, unchanged 133 fixture results and 43 baselines,
614 installed commands / 207 observations and all 91 earlier positive ART1
identities unchanged. The post-commit rebuild reproduces all three binaries
and the executed C1 artifact byte for byte. All 64 formal audits are UNKNOWN.
The old 64-group default arena boundary remains green. Next: repair duplicate
struct initializers in the seed, then nominal structs/field reads and nested
writes. Full SH3/SH4, C2/C3, six-platform acceptance and compiler proofs remain open.


2026-09-24 continuation: seed struct literals reject repeated initializers at
`ac2481d`, closing an exactly-once field-contract violation before the nominal
struct increment. [Pinned evidence](self-hosting/unique-struct-initializers-validation.json)
records 1020 / 122 / 380 passing owner tests, unchanged 133 fixture rows and 43
manual baselines, nine installed CLI commands and unchanged complete C1 bytes.
All 64 formal audits remain UNKNOWN; guest structs and full SH3/SH4 stay open.


2026-09-24 continuation: nominal descriptor identity, field layout and owner-based
visibility accepted at `1e36ceb`. [Pinned evidence](self-hosting/native-compiler-nominal-validation.json)
records 1026 / 122 / 380 passing owner tests, unchanged 133 fixture rows and 43
manual baselines, 614 general CLI commands plus 52 component commands, and all
105 prior positive ART1 identities unchanged. All 67 formal audits are UNKNOWN.
Source struct declarations/constructors/reads follow; full SH3/SH4, C2/C3 and
six-platform acceptance remain open. Noun temperature stays 128K.


2026-09-24 continuation: source nominal declarations, constructors and field reads
accepted at `60ea10e`. [Pinned evidence](self-hosting/native-compiler-records-validation.json)
records 1034 / 122 / 380 passing owner tests, 133 unchanged fixture rows and 43
manual baselines, 712 installed CLI commands / 240 observations, and all 105
previous positive ART1 identities unchanged. Rebuilt binaries and complete C1
are byte-identical to those tested. All 71 formal audits remain UNKNOWN.
The valid combined long-name workload exhausts 786432 arena nodes and stays
explicitly open under SH4. Next: persistent static field writes, then arrays
and imports. Full SH3/SH4, C2/C3, six platforms and native proofs remain open.
Noun temperature stays 128K.


2026-09-24 continuation: persistent static record field writes accepted at
`324a015`. [Pinned evidence](self-hosting/native-compiler-record-writes-validation.json)
records 1040 / 122 / 380 passing owner tests, 133 unchanged fixture rows and 43
manual baselines, 773 installed CLI commands / 262 observations and all 119
previous positive ART1 identities unchanged. Eight new successful JOB1 programs
exercise snapshots, nested values, calls, loops and complete Noun/Digest/tuple
replacement. Constructor continuations decode owned layouts once; public
Seq/Bytes admission remains intact. The five retained wide-source programs still
exhaust 786432 nodes during generation; direct emitter path tests do not close
this SH4 boundary. All 74 formal audits are UNKNOWN. Arrays, real imports,
full SH3/SH4, generated compiler profiles, C2/C3, six platforms and native proof
gates remain open. Noun temperature stays 128K.


2026-09-25 continuation: fixed Field-array values, exact annotations and checked
reads accepted at `a427531`. [Pinned evidence](self-hosting/native-compiler-arrays-validation.json)
records 1048 / 122 / 380 passing owner tests, 133 unchanged fixture rows and 43
manual baselines, 851 installed commands / 288 observations and all 127 prior
positive ART1 identities unchanged. Fourteen new successful JOB1 programs cover
array transfer, snapshots, typed calls and shared reads. Public source arenas
remain unchanged; the separate 4096-function planner component uses its own
larger allowance. Post-commit installs reproduce the executed binaries and C1.
All 79 formal audits are UNKNOWN. Next: unify seed typed constant resolution,
then guest constants/attributes/asserts and real imports. Full SH3/SH4, generated
profiles, C2/C3, six platforms and native proofs remain open. Noun stays 128K.


2026-09-25 continuation: shared typed Rust seed constants accepted at `2f6b0ef`.
[Pinned evidence](self-hosting/typed-constants-validation.json) records
1058 / 122 / 380 passing owner tests, 133 unchanged fixture rows and 43 baselines,
61 installed commands and 20 cases exercised on nox and Triton. Final active
value/type/visibility now agree across checking, generic specialization and
lowering; aliases preserve lexical ownership and raw integer dimensions.
Rejected sources preserve existing artifacts. Rebuilds reproduce all three
binaries; entire C1 matches the array delivery, whose 851-command guest corpus
was not rerun for this seed-only change. All 79 formal audits remain UNKNOWN.
Next: guest constants, attributes/asserts, then imports. Full SH3/SH4, generated
profiles, C2/C3, six platforms and native proofs stay open. Noun stays 128K.

2026-09-25 continuation: retained guest packages accepted at `473d20c`.
[Pinned evidence](self-hosting/guest-package-validation.json) records
1120 / 122 / 380 passing owner tests, 133 unchanged baseline rows and 43 manual
programs, 1192 installed commands / 401 observations and all 189 previous
positive ART1 identities unchanged. The original 61/62-bit record-write vectors
now compile and execute to 3199 within the same 786432-node arena; 63–65 remain
explicit allocation failures. The corpus selects the supported 60000ms host
deadline, with Joy's 30000ms default and deterministic quotas unchanged.
All 92 formal audits remain UNKNOWN. Package index 4096 is tested separately
from compact IDs and source capacity. Guest graph/linking, whole compiler scale,
generated compiler profiles, C2/C3, six platforms and native Zheng proofs remain
open. Noun stays 128K.
