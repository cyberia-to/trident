# Self-hosting on soft3 — progress ledger

Updated: 2026-09-23. Working contract:
[reference/self-hosting.md](../reference/self-hosting.md).
This ledger is the current execution checklist. Dated assessments and receipts
retain their original observations; they are not substituted for gate evidence.

## Current position

**Next: SH1 native source data and Joy protocol admission.** SH0's compiler
inventory, data/job formats and runtime/control-flow contract are reviewed and
specified. Nox's complete codec, lifetime arena allowance and sequential heap
executor and Joy's structured raw run are in `release/0.4`.
The native source Noun type, collections, reusable source calls/loops and
production compiler JOB1/RES1 admission remain required before SH1 closes.
No self-compilation or compiler execution proof is claimed.

Integration: `release/0.4`. First delivery: `feat/0.4-sh0-inventory`, based on
`360b737e073ca2f969ab0c78460b4228bcac7b78`, with pinned release sibling checkouts.
Active isolated checkout: `~/cyber/.worktrees/selfhost-0.4/trident`.
PRs target the integration branch; master stays unchanged until 0.4 acceptance.

| Gate | Status | Missing acceptance evidence |
|---|---|---|
| [SH0](../reference/self-hosting.md#sh0-contract-and-compiler-subset) | Closed — contract gate | [Owner review and runtime evidence](self-hosting/native-runtime.md) |
| [SH1](../reference/self-hosting.md#sh1-native-bootstrap-foundation) | In progress — runtime foundation delivered | Source Noun/collections, reusable lowering and full compiler-job transport |
| [SH2](../reference/self-hosting.md#sh2-first-native-compiler) | Open — needs SH1 | `.tri` compiler running on nox emits a separately executed nox program |
| [SH3](../reference/self-hosting.md#sh3-compiler-language-coverage) | Open — needs SH2 | Whole compiler subset and executed differential/rejection corpus |
| [SH4](../reference/self-hosting.md#sh4-complete-project-and-runtime-scale) | Open — starts after SH1 | Real closure, compiler-scale memory/runtime and boundary receipts |
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
  4097/5000 iterations with traced/run-only agreement. Source lowering is pending.
- [x] SH1 Joy raw transport slice: [PR5](https://github.com/cyberia-to/joy/pull/5),
  complete noun execution/publication with NoTrace;85 workspace tests passed.
- [ ] SH1 Joy compiler-job slice: production JOB1/RES1 admission and binding.
- [ ] SH1 Trident slice: implement Noun type/intrinsics and raw entry/artifact
  generation; port native collections; reusable calls and bounded source loops.

Resolve protocol or language choices explicitly in their owner contracts before
dependent implementation. Keep the next executable task and its dependencies
visible here as those decisions land.

## Known blockers and ownership

| Blocker | Owner / first gate | Baseline |
|---|---|---|
| RAM-based compiler structures; no source-level native dynamic data API | Trident / SH0–SH1 | `.tri` compiler modules and current AST/type system |
| Calls inline; loops unroll; dynamic indexing and large typed entries reject | Trident / SH1 | [nox source probes](self-hosting-2026-09-23/soft3-source-probes.json) |
| Full compiler arena/memory scale and source-loop lowering still unmeasured | nox + Joy + Trident / SH1, SH4 | Heap executor and node cap delivered; compiler-scale workload remains pending |
| Production compiler JOB1/RES1 admission is pending | Joy / SH1 | Complete raw ART1 transport delivered; compiler profile remains rejected |
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

Re-estimate after SH2 and again from SH4's complete-workload measurements.
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
