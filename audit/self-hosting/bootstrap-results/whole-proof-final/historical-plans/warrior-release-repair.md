# Trident and warriors — repair and release plan

Status: approved by the owner on 2026-09-11; implementation in progress on `fix/warrior-release` branches.
Evidence: `../audits/2026-09-11-warrior-release.md`.
Objective: source compilation, execution, proof generation and verification work across Trident + Joy/nox/Zheng and Trident + Trisha/Triton; both distributions install outside the development tree.

## 1. Define the final contract

Update the canonical warrior API and migration record first. Core owns parsing/typechecking, resolved modules, TIR, nox semantics and generic metadata; Trisha owns Triton lowering, costs, proof/runtime integration, Neptune resources and hand baselines. Restore the generic neural harness/target split recorded in the owner's settled decision; do not silently treat the wholesale move as that design.

Document one effective target-resolution rule (explicit CLI selection, project selection, nox default). Unsupported terrain and absent warrior produce nonzero exit. Keep bundle metadata complete through warrior construction.

## 2. Repair correctness before packaging

- Trident: cfg filtering, lexical shadowing/constant folding, qualified imported-function lowering; exercise compile, costs and bundle paths together.
- Trisha: source -> TIR -> owned lowering -> metadata-preserving bundle for run/prove/deploy. Reject unknown/non-Triton targets; move Neptune/baseline resources and remove sibling-asset assumptions.
- Joy/Zheng/Lens: migrate authenticated opening serialization and recursive verification together; bind public output claims to the proven statement. Retain rejection of altered proofs, roots, openings, assembly and claimed output.
- CLI: correct warrior dispatch, proof exit codes, claim/digest validation and collision-free batch output; replace false deployment success with an explicit unsupported result until an actual deployment implementation passes its checks.
- Close compiler issue #40 with valid deep-stack assembly and reference-backed execution; repair baseline drift. Restore the moved benchmark command with honest coverage/results.

Use disjoint repository/module scopes and atomic feature-branch commits. Preserve all pre-existing work. Changes to upstream components require rebuilding the dependent warriors.

## 3. Establish release gates

Regression tests must execute the bugs' examples and compare independent expected results. Add process-level source tests so library mocks/manual bundles cannot bypass the public CLI again.

For each supported target: `.tri` -> build -> run -> prove -> verify, directly and through Trident delegation; include imports, project profiles/targets, public and secret inputs, batch operation, and a tampered-proof rejection. Verify failure cases return nonzero and produce no success artifacts.

Run required checks and complete suites, costs/benchmarks and audit checks after fixes. Document the actual supported language surface. A fully working release claim requires implementing promised surface gaps or explicit owner acceptance of a narrower release scope; passing arithmetic smoke tests alone is insufficient.

## 4. Distribute and release together

Proposed distribution: versioned Trident registry package and coordinated GitHub warrior releases with tested binaries plus reproducible source/bootstrap instructions and pinned dependencies. Ship required std/vm/os assets. Preserve the upstream patch workflow; publishing renamed forks of Triton dependencies is a separate decision, not a routine version edit.

Choose one breaking compiler version and update Joy/Trisha requirements together; remove obsolete competing release sequences. Test installation into an isolated prefix and run from a directory outside every checkout with TRIDENT_* unset. Build a fresh Trisha source distribution with its vendored patch bootstrap; the author's sibling tree must not be required.

Only after these gates pass: prepare final release notes with verified platform/support matrix, checksums and installation commands; merge reviewed changes and publish coordinated releases. Download/install those exact artifacts and repeat the same end-to-end smoke and negative tests.

Completion evidence belongs in the audit/release record: commits, artifact versions, commands, actual outputs, test totals and remaining explicit scope limits.

## Current work checkpoint

2026-09-11: fixes committed on `fix/warrior-release` branches. No release published.

- Trident: 718 default tests pass, 44 opt-in neural tests pass; nox cfg/import/shadow corrections, unsupported-target guard, real nox #[test] runner, portable resources, strict metadata-preserving bundle JSON and CLI dispatch.
- Trisha: 216 default tests pass, 50 opt-in neural adapter tests pass; all-feature workspace check passes. Triton lowering, Neptune library and baselines are warrior-owned. Generic neural harness is restored in core.
- SHA-256 matches the FIPS empty-message digest on the real Triton VM; 42 stdlib tests parse real library assembly. Deep-stack and aggregate-layout faults fixed. Historical baselines without execution fixtures remain UNVERIFIED and block complete benchmark acceptance.
- Candidate versions remain unpublished: Trident 0.3.0, Trisha 0.2.0, Joy 0.4.0. Final core registry package, clean source-archive builds, isolated cargo installs and archived-binary smoke pass on Darwin arm64.
- Joy/Zheng safe fixes preserve authenticated TensorMerkle openings, reject unsupported public-output claims, and disable unauthenticated recursive axis/look openings. Current Joy integration gate: 24 pass, 3 state-proof acceptance failures; keep those failures visible.
- Full proof release requires verifier-checked execution/copy/public IO constraints, sound accumulation and privacy. Design: `~/cyber/zheng/.claude/plans/authenticated-execution-release.md`. Owner explicitly reaffirmed full release scope; do not substitute successful arithmetic proving for execution-proof soundness.
- Preserve original Zheng CLAUDE.md/Cargo.lock and nox Cargo.lock modifications. Joy lock regenerated for coordinated dependency versions; initial copy remains /tmp/joy-release-initial-Cargo.lock.
- Final evidence, local candidate artifacts and exact remaining gates: audit/warrior-release-validation.md. Protocol redesign and live deployment are not complete.
