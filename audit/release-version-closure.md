# Coordinated breaking release: version and Cargo closure map

Date: 2026-09-11. Initial read-only review, followed by parent-authorized application of the proposed versions and compiler API 2. The tables retain the pre-change versions as the migration record. Registry availability/name ownership was not checked or claimed. Versions must additionally be free in the intended registry before publication.

## Applied version set (previous → new)

Use new pre-1.0 minor versions for changed execution/proof acceptance and public interfaces. Cargo treats the leftmost nonzero version component as the compatibility boundary; behavior changes also require maintainer judgment ([Cargo SemVer guidance](https://doc.rust-lang.org/cargo/reference/semver.html)). Do not reuse an earlier candidate version for these changed binaries and artifacts.

| Package | Previous | Applied |
|---|---|---|
| `bbg` | 0.2.1 | **0.3.0** |
| `cyber-joy` | 0.4.0 | **0.5.0** |
| `cyber-lens` | 0.1.3 | **0.2.0** |
| `cyber-lens-assayer` | 0.1.1 | **0.2.0** |
| `cyber-lens-brakedown` | 0.1.1 | **0.2.0** |
| `cyber-lens-ikat` | 0.1.1 | **0.2.0** |
| `cyber-lens-porphyry` | 0.1.1 | **0.2.0** |
| `cyber-nox` | 0.2.0 | **0.3.0** |
| `joy-rs` | 0.4.0 | **0.5.0** |
| `lens-cli` | 0.1.2 | **0.2.0** |
| `nox-cli` | 0.1.0 | **0.2.0** |
| `trident-lang` | 0.3.0 | **0.4.0** |
| `trisha` | 0.2.0 | **0.3.0** |
| `trisha-honeycrisp` | 0.2.0 | **0.3.0** |
| `trisha-neptune` | new private adapter crate | **0.3.0** |
| `trisha-rs` | 0.2.0 | **0.3.0** |
| `trisha-wgpu` | 0.2.0 | **0.3.0** |
| `zheng` | 0.3.3 | **0.4.0** |
| `zheng-cli` | 0.1.0 | **0.2.0** |

Reasons: Trident changes ownership, public APIs, generated code/event output and cryptographic library behavior; Trisha changes Triton2 to7, native and recursive proof formats, input rejection and compiler integration; Joy changes its private proof backend/envelope; Zheng and BBG change execution/state relations and certificate acceptance. Brakedown opening v3 breaks the old opening transcript/representation, inherited by Ikat and Assayer. Porphyry now rejects unsupported multipoint batches rather than accepting unbound claims. Lens facade and CLI expose those changes. nox depends on the changed Brakedown implementation under its public brakedown feature, so the proposed0.3 isolates that proof behavior even though its arithmetic interpreter source is unchanged in the current diff.

Trisha GPU/helper crates expose the new Trident runtime types and input contract; release them with the CPU crate at0.3. The nox and Zheng CLI versions advance independently from their libraries.

**Minimal unchanged set:** keep Lens core0.1.2 and Binius0.1.1: no changes to those crate sources/public contracts were found in this working diff. Keep Strata0.1.1, Hemera0.3.1, Honeycrisp helper crates0.2.0 and Tape0.1.0 unless their separate owners identify additional unreleased changes. `trident-silicon`0.1.0 remains `publish=false`; updating its nox requirement does not certify experimental backends. Do not bump `lens/workspace.package.version` blindly: core inherits it. Set CLI0.2 explicitly if keeping the unchanged core0.1.2.

## Complete local package closure

Derived from `cargo metadata --format-version 1 --all-features --locked --offline` for Trident, Trisha, Joy, Zheng, Lens and BBG, plus direct nox workspace inspection. This includes optional/dev edges and local vendor patches, not every unrelated crate in the monorepo. Registry dependencies remain governed by their lockfiles.

| Local package | Manifest | Current → proposed |
|---|---|---|
| `bbg` | `bbg/rs/Cargo.toml` | 0.2.1 → 0.3.0 |
| `fjall` | `bbg/rs/vendor/fjall/Cargo.toml` | 2.11.2 → 2.11.2 |
| `cyber-hemera` | `hemera/rs/Cargo.toml` | 0.3.1 → 0.3.1 |
| `acpu` | `honeycrisp/acpu/Cargo.toml` | 0.2.0 → 0.2.0 |
| `aruminium` | `honeycrisp/aruminium/Cargo.toml` | 0.2.0 → 0.2.0 |
| `unimem` | `honeycrisp/unimem/Cargo.toml` | 0.2.0 → 0.2.0 |
| `cyber-joy` | `joy/cli/Cargo.toml` | 0.4.0 → 0.5.0 |
| `joy-rs` | `joy/rs/Cargo.toml` | 0.4.0 → 0.5.0 |
| `cyber-lens-assayer` | `lens/assayer/Cargo.toml` | 0.1.1 → 0.2.0 |
| `cyber-lens-binius` | `lens/binius/Cargo.toml` | 0.1.1 → 0.1.1 |
| `cyber-lens-brakedown` | `lens/brakedown/Cargo.toml` | 0.1.1 → 0.2.0 |
| `lens-cli` | `lens/cli/Cargo.toml` | 0.1.2 → 0.2.0 |
| `cyber-lens-core` | `lens/core/Cargo.toml` | 0.1.2 → 0.1.2 |
| `cyber-lens-ikat` | `lens/ikat/Cargo.toml` | 0.1.1 → 0.2.0 |
| `cyber-lens-porphyry` | `lens/porphyry/Cargo.toml` | 0.1.1 → 0.2.0 |
| `cyber-lens` | `lens/src/Cargo.toml` | 0.1.3 → 0.2.0 |
| `nox-cli` | `nox/cli/Cargo.toml` | 0.1.0 → 0.2.0 |
| `cyber-nox` | `nox/rs/Cargo.toml` | 0.2.0 → 0.3.0 |
| `strata-compute` | `strata/compute/Cargo.toml` | 0.1.1 → 0.1.1 |
| `strata-core` | `strata/core/Cargo.toml` | 0.1.1 → 0.1.1 |
| `strata-ext` | `strata/ext/Cargo.toml` | 0.1.1 → 0.1.1 |
| `strata-genies` | `strata/genies/rs/Cargo.toml` | 0.1.1 → 0.1.1 |
| `strata-jali` | `strata/jali/rs/Cargo.toml` | 0.1.1 → 0.1.1 |
| `strata-kuro` | `strata/kuro/rs/Cargo.toml` | 0.1.1 → 0.1.1 |
| `strata-nebu` | `strata/nebu/rs/Cargo.toml` | 0.1.1 → 0.1.1 |
| `strata-proof` | `strata/proof/Cargo.toml` | 0.1.1 → 0.1.1 |
| `strata-trop` | `strata/trop/rs/Cargo.toml` | 0.1.1 → 0.1.1 |
| `cyber-tape` | `tape/impl/rust/Cargo.toml` | 0.1.0 → 0.1.0 |
| `trident-lang` | `trident/Cargo.toml` | 0.3.0 → 0.4.0 |
| `trident-silicon` | `trident/silicon/Cargo.toml` | 0.1.0 → 0.1.0 |
| `tasm-lib` | `trisha/.vendor/tasm-lib/Cargo.toml` | 7.0.0 → 7.0.0 |
| `tasm-object-derive` | `trisha/.vendor/tasm-object-derive/Cargo.toml` | 7.0.0 → 7.0.0 |
| `triton-air` | `trisha/.vendor/triton-air/Cargo.toml` | 7.0.0 → 7.0.0 |
| `triton-constraint-builder` | `trisha/.vendor/triton-constraint-builder/Cargo.toml` | 7.0.0 → 7.0.0 |
| `triton-constraint-circuit` | `trisha/.vendor/triton-constraint-circuit/Cargo.toml` | 7.0.0 → 7.0.0 |
| `triton-isa` | `trisha/.vendor/triton-isa/Cargo.toml` | 7.0.0 → 7.0.0 |
| `triton-vm` | `trisha/.vendor/triton-vm/Cargo.toml` | 7.0.0 → 7.0.0 |
| `twenty-first` | `trisha/.vendor/twenty-first/Cargo.toml` | 1.1.0 → 1.1.0 |
| `trisha` | `trisha/cli/Cargo.toml` | 0.2.0 → 0.3.0 |
| `trisha-honeycrisp` | `trisha/honeycrisp/Cargo.toml` | 0.2.0 → 0.3.0 |
| `trisha-neptune` | `trisha/neptune/Cargo.toml` | new → 0.3.0 (`publish=false`) |
| `trisha-rs` | `trisha/rs/Cargo.toml` | 0.2.0 → 0.3.0 |
| `trisha-wgpu` | `trisha/wgpu/Cargo.toml` | 0.2.0 → 0.3.0 |
| `zheng-cli` | `zheng/cli/Cargo.toml` | 0.1.0 → 0.2.0 |
| `zheng` | `zheng/rs/Cargo.toml` | 0.3.3 → 0.4.0 |

## Every local dependency edge needing a requirement edit

Proposed requirements below are normal Cargo caret minimums, e.g. `version = "0.4.0"`; source candidate lockfiles pin exact resolved versions. New path-only requirements are needed for registry packaging even when the dependency itself is unchanged. Unchanged versioned edges are intentionally omitted. Workspace-inherited edges must be edited at their workspace declaration, not duplicated in each member.

| Consumer manifest | Dependency package | Current requirement | Required minimum |
|---|---|---|---|
| `bbg/rs/Cargo.toml` | `cyber-lens` | `^0.1.3` | `0.2.0` |
| `bbg/rs/Cargo.toml` | `cyber-nox` | `^0.2` | `0.3.0` |
| `bbg/rs/Cargo.toml` | `zheng` | `^0.3` | `0.4.0` |
| `honeycrisp/acpu/Cargo.toml` (dev) | `strata-nebu` | `*` | `0.1.1` |
| `joy/cli/Cargo.toml` | `joy-rs` | `^0.4.0` | `0.5.0` |
| `joy/cli/Cargo.toml` | `trident-lang` | `^0.3` | `0.4.0` |
| `joy/cli/Cargo.toml` (dev) | `bbg` | `^0.2` | `0.3.0` |
| `joy/rs/Cargo.toml` | `bbg` | `^0.2` | `0.3.0` |
| `joy/rs/Cargo.toml` | `cyber-nox` | `^0.2` | `0.3.0` |
| `joy/rs/Cargo.toml` | `trident-lang` | `^0.3` | `0.4.0` |
| `joy/rs/Cargo.toml` | `trisha-rs` | `*` | `0.3.0` |
| `joy/rs/Cargo.toml` | `zheng` | `^0.3.1` | `0.4.0` |
| `lens/assayer/Cargo.toml` | `cyber-lens-brakedown` | `^0.1.1` | `0.2.0` |
| `lens/cli/Cargo.toml` | `cyber-lens` | `*` | `0.2.0` |
| `lens/ikat/Cargo.toml` | `cyber-lens-brakedown` | `^0.1.1` | `0.2.0` |
| `lens/src/Cargo.toml` | `cyber-lens-assayer` | `^0.1.1` | `0.2.0` |
| `lens/src/Cargo.toml` | `cyber-lens-brakedown` | `^0.1.1` | `0.2.0` |
| `lens/src/Cargo.toml` | `cyber-lens-ikat` | `^0.1.1` | `0.2.0` |
| `lens/src/Cargo.toml` | `cyber-lens-porphyry` | `^0.1.1` | `0.2.0` |
| `nox/cli/Cargo.toml` | `cyber-nox` | `*` | `0.3.0` |
| `nox/cli/Cargo.toml` | `strata-nebu` | `*` | `0.1.1` |
| `nox/rs/Cargo.toml` | `cyber-lens-brakedown` | `^0.1` | `0.2.0` |
| `trident/Cargo.toml` | `cyber-nox` | `^0.2` | `0.3.0` |
| `trident/silicon/Cargo.toml` | `cyber-nox` | `^0.2` | `0.3.0` |
| `trisha/cli/Cargo.toml` | `trident-lang` | `^0.3` | `0.4.0` |
| `trisha/cli/Cargo.toml` | `trisha-honeycrisp` | `^0.2` | `0.3.0` |
| `trisha/cli/Cargo.toml` | `trisha-rs` | `^0.2` | `0.3.0` |
| `trisha/cli/Cargo.toml` | `trisha-wgpu` | `^0.2` | `0.3.0` |
| `trisha/honeycrisp/Cargo.toml` | `acpu` | `*` | `0.2.0` |
| `trisha/honeycrisp/Cargo.toml` | `aruminium` | `*` | `0.2.0` |
| `trisha/honeycrisp/Cargo.toml` | `trident-lang` | `^0.3` | `0.4.0` |
| `trisha/rs/Cargo.toml` | `trident-lang` | `^0.3` | `0.4.0` |
| `trisha/rs/Cargo.toml` | `zheng` | `*` | `0.4.0` |
| `trisha/wgpu/Cargo.toml` | `trident-lang` | `^0.3` | `0.4.0` |
| `zheng/cli/Cargo.toml` | `cyber-hemera` | `*` | `0.3.1` |
| `zheng/cli/Cargo.toml` | `cyber-lens` | `*` | `0.2.0` |
| `zheng/cli/Cargo.toml` | `strata-nebu` | `*` | `0.1.1` |
| `zheng/cli/Cargo.toml` | `cyber-nox` | `*` | `0.3.0` |
| `zheng/cli/Cargo.toml` | `cyber-tape` | `*` | `0.1.0` |
| `zheng/cli/Cargo.toml` | `zheng` | `*` | `0.4.0` |
| `zheng/rs/Cargo.toml` | `cyber-lens` | `^0.1.3` | `0.2.0` |
| `zheng/rs/Cargo.toml` | `cyber-nox` | `^0.2` | `0.3.0` |

`honeycrisp/acpu` dev-nebu is inherited from `honeycrisp/Cargo.toml`; set the version there. The Lens algebra/Hemera requirements already have compatible versions and need no release-driven edit. Source closure keeps vendored fjall2.11.2, the seven Triton/tasm crates7.0.0 and twenty-first1.1.0 unchanged; do not assign upstream crates local product versions.

Regenerate/check all affected root lockfiles after applying edits: `trident/Cargo.lock`, `trisha/Cargo.lock`, `joy/Cargo.lock`, `zheng/Cargo.lock`, `lens/Cargo.lock`, `nox/Cargo.lock`, `bbg/rs/Cargo.lock`; Honeycrisp if its inherited dev requirement changes. Before migration, six reviewed root metadata checks passed and nox failed because its lockfile was stale. The applied checkpoint below supersedes that finding: all eight roots now pass locked metadata.

## Version literals and protocol identifiers

- Product/owner versions come from `env!("CARGO_PKG_VERSION")` in Trisha/Joy `rs/target.rs`; clap versions derive from Cargo. Rebuild embedded packages, lock artifacts and expected package hashes after a bump.
- Before migration, `compiler_api = 1` and `schema_version = 1` were used in Trisha/Joy target constructors and Trident package validation. The review recommended compiler_api2 for a coordinated incompatible compiler/warrior contract; update constructors, validator, target-package test fixtures and CLI fake-warrior descriptions together. JSON schema1 can remain if its structure is unchanged. If compiler_api is intended to describe only JSON compatibility, explicitly document that narrower meaning instead of claiming old-warrior compatibility.
- Protocol identifiers must not be mechanically replaced with Cargo versions: native `stark-triton-v7`; CCS `zheng-ccs-triton7-zk-v2`; Joy `joy-nox-ccs-triton7-zk-v3`/`JOYZK003`; public `JOYEXEC2`; public-state `JOYST001`; BBG certificate2; Brakedown commitment2 and opening3; recursive witness schema1. Current source already distinguishes these.
- Lens CLI `src/artifact.rs` version2 and `src/assayer.rs` version1 are file-envelope versions; `src/algo.rs` and `src/assayer.rs` contain `/v0.1.0` transcript domains. They are protocol domains, not stale Cargo labels to replace blindly. Check whether the opening3 payload is explicitly distinguished by existing decoding before choosing any additional envelope/domain bump.
- Pinned upstream7.0.0/1.1.0 strings occur in Trisha `patches/apply.nu`, `patches/tasm.nu`, build-candidate checks and migration/wire tests. Preserve them unless the backend changes again.
- Release smoke asserts `stark-triton-v7` and `JOYZK003`; retain assertions. Recreate source snapshots, provenance hashes, package JSON, installed binaries and proof fixtures after the final version/requirement edits. Historical changelog versions and frozen protocol vectors should remain historical.

## Registry package and path-escape blockers

Cargo requires a version alongside a path dependency for registry publication; published resolution uses that version rather than the local path ([Cargo dependency documentation](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)). Path-only edges are enumerated above. This alone is insufficient:

1. `trisha/rs/build.rs` scans sibling `../lib`, `../targets`, `../networks`; `rs/target.rs` directly includes sibling target/network files. `trisha/cli/build.rs` reads `../networks/neptune/states`. These resources are outside the individual crate roots and are absent in isolated `.crate` archives.
2. `joy/rs/target.rs` includes `../targets/nox/capabilities.json`, also outside its crate. Trisha tests include sibling baselines/examples; packaged test targets require those resources too if retained. A package `include` pattern does not make arbitrary sibling trees part of an isolated crate. Move/package generated resources or introduce publishable resource crates before claiming registry installation works.
3. Trisha and Joy rely on root `[patch.crates-io]` vendor overlays. Patches are only read from the workspace root, not propagated through published dependencies ([Cargo overrides](https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html)). A versioned registry dependency on trisha-rs will not install those overlays automatically. Either use compatible upstream APIs without patches or publish/rename owned forks with explicit dependencies, subject to upstream licensing and registry ownership.
4. Path-version fallback for vendored `bbg/rs/vendor/fjall` resolves registry fjall2.11.2 when published; verify that local modifications are not required before relying on that fallback. This review did not assert byte identity of that vendor copy.
5. Source packaging currently preserves sibling repositories and regenerates vendor overlays, so it can handle these paths. `package-source.nu` is a coordinated source distribution, not evidence that `cargo publish` or `cargo install <registry-name>` succeeds. No registry upload/package verification was performed.

## Platform evidence and available checks

Host observed: Darwin arm64, `aarch64-apple-darwin`, rustc1.95.0 (Homebrew), LLVM22.1.3. The current session passed Trident release tests (765 at the last compiler checkpoint) and neural compilation on this host. Trisha7 functional tests and Joy checks are separately recorded by their owners; the expensive recursive outer proof is a separate active gate. This is not an all-platform certificate.

Installed Rust targets: `aarch64-apple-darwin`, `aarch64-linux-android`, `i686-unknown-linux-musl`, `wasm32-unknown-unknown`, `x86_64-unknown-linux-gnu`. Only the first is natively executable here. Installed std libraries alone do not provide a cross linker, Linux runtime, Android NDK/device, Wasm engine or GPU.

Available honest next checks:

- Host CPU: rebuild the final versioned archive with `nu trisha/scripts/build-candidate.nu <archive> <prefix>` then run `smoke-release.nu`; captures installed-bin discovery, source assets, native7 proof and Joy v3 verification. Require the final source hashes and real recursive gate result.
- Linux type/build check: `cargo check --manifest-path trident/Cargo.toml --target x86_64-unknown-linux-gnu` and equivalent CPU-only Trisha/Joy checks, provided target-specific dependencies/toolchains resolve. This was **not run in this read-only version review**. A native Linux CI/runner is still required for linkage and runtime/proof validation.
- 32-bit/Wasm/Android: installed-target compilation can reveal unsupported assumptions, but these are not current release promises. Native prover dependencies and OS integrations may reject those targets. Do not infer support from the Trident target-design catalog.
- Metal/WGPU/Honeycrisp: host availability does not establish complete proving or execution semantics. Run backend-specific tests and supported capability reports; CPU release results do not certify experimental GPU paths.
- Minimum Rust version: current compiler1.95 does not prove the declared lower MSRV (Lens declares1.89). Run the pinned minimum toolchain separately before publishing an MSRV promise.

Trident README machine/target tables include design and experimental architectures, not verified host platforms. Current packaging scripts explicitly describe CPU coordinated candidates and exclude live deployment/unsupported network capabilities. Preserve those qualifications; raising Cargo versions must not silently promote catalog entries into working release claims.

## Applied checkpoint

The 18 proposed package versions and dependent requirements were applied after
parent authorization. Lens core and Binius versions remain unchanged. Target
schema stays 1; compiler API is now 2, with API 1 explicitly rejected. Root
lockfiles were refreshed only through offline Cargo metadata resolution, not
`cargo update`; observed dependency updates were local workspace packages.
The previously stale nox lockfile was refreshed too. Packaging and platform
limitations above remain applicable; this version change is not publication.

Final metadata validation: all eight roots listed above pass offline locked
all-features resolution. Registry package/version sets for the six roots with
a pre-migration snapshot are unchanged. Focused API 2 checks passed: Trident
target-package/CLI-warrior/source-parity tests, Trisha owner-package tests and
Joy build metadata tests. Smoke script syntax was checked; the newly versioned
installed full smoke remains the separate final-candidate gate.

## 2026-09-12 adapter and distribution closure

The new `trisha-neptune` crate makes Trisha a five-crate workspace. The CLI's
default `triton` feature enables its optional path dependency; a mining-only
CLI does not pull in transaction proving. The adapter belongs to the warrior,
outside the compiler and CPU execution engine. It constructs canonical output
commitments and validates a complete supplied transaction intent before the
explicit authenticated-gateway submission path.

The production adapter adds seven upstream Git packages, all version 0.15.1
and pinned to Neptune commit `9869b5e35b659dc520fad51ba5a9c812fed46db0`:
consensus, primitives, mutator-set, job-queue, rpc-api, rpc-macros and wallet.
This deliberately extends the earlier registry-unchanged checkpoint; the
Trisha and Joy lockfiles now carry the new adapter closure. No broad
`cargo update` was used. The isolated protocol oracle has its own lockfile.

Fresh `--all-features --locked --offline` metadata passes for Trident, Trisha,
Joy, Zheng, Lens, BBG (`bbg/rs`), nox and Honeycrisp. The original seven-root
package inventory now contains **45 local packages**, including the new
adapter; including the entire Honeycrisp workspace gives 49. The source
packager's narrower Trisha+Joy dependency closure contains **39 local packages
across 10 repositories**. These are different denominators, not omitted files:
each selected repository is copied completely except explicit build artifacts.
Receipts: `/tmp/cyber-release-metadata-<repository>.json`.

The source distribution carries local repositories and eight pinned patched
Triton-family sources. Other locked Git/registry sources still need Cargo
cache or network access; it is not an offline vendor cache. The public-state
fixture builder now ships its own manifest and lockfile under
`trisha/scripts/fixtures/` and builds with `--locked --offline`. Deterministic
archive ordering, fixed prefix, normalized metadata and complete dirty-input
detection pass their regression tests. Final committed source and installed
binary receipts remain separate release gates.

## Typed-entry interface continuation — 2026-09-12

The unshipped coordinated versions remain Trident0.4 / Trisha0.3 / Joy0.5.
Compiler API3 supersedes API2 for resolved typed executable-entry metadata.
`trident::COMPILER_API` now owns the value; all compiled descriptors and core
package fixtures use it. Schema1 is unchanged. API0/1/2/future values are rejected
by regression tests. Frozen earlier candidate receipts retain API2 identities.
The canonical contract is `reference/warrior-api.md`; actual typed entry
behavior and proof evidence belong to Trisha's entry ABI reference and audit.
