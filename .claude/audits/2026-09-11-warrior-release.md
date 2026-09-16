# Trident / Trisha / Joy release audit — 2026-09-11

Verdict: release blocked. S3 moved substantial implementation, but the source CLI integration is broken, nox emits wrong results for valid programs, and current zheng artifacts cannot be serialized.

## Scope and evidence

Audited local heads: trident `657d50e`, trisha `0d1cfcd`, joy `63bec5e`, zheng `5a2c591`, lens `48c22e0`. Trident and Trisha were clean. Joy had an existing modified Cargo.lock; its initial bytes were backed up before resolving the current local dependencies and restored after testing. Other repositories' changes were preserved.

Read the development doctrine, five foundational documents, both compiler/warrior instructions, migration plan, warrior API and quality protocol. Independent read-only reviews covered compiler architecture, Triton transfer, and Joy/Zheng integration. This is a migration/release audit with concrete repros, not an exhaustive cryptographic audit of every dependency.

| check | observed result |
|---|---|
| trident `cargo test --workspace --locked` | 725 passed, 0 failed across compiler, integration, silicon and binary suites |
| trisha `cargo test --workspace --locked` | 295 passed, 0 failed; 1 doctest ignored; 3 vendored triton-vm warnings |
| joy `cargo test --workspace --locked` | refuses stale lockfile |
| joy `cargo test --workspace --offline` | unit tests 10 passed; integration 19 passed, 7 failed |
| fresh debug binaries | Trident, Trisha and Joy build successfully |
| trident `cargo check -p trident-lang --locked` | succeeds without warnings |
| trident `cargo package -p trident-lang --locked` | archive verified; 479 files; package still version 0.2.0 |
| trisha `cargo publish --dry-run -p trisha-rs` | fails: trident-lang path dependency has no version |
| GitHub release inventory | Trident latest v0.2.0; Trisha has no release |

Runtime repros below used freshly built binaries, explicitly selected on PATH. Installed older binaries can conceal current failures. No release was published and no implementation was changed.

`trident audit` now requires a source argument. Auditing Joy's add fixture returns "safe" with zero constraints; this is not evidence of execution or proof correctness.

## Blocking findings

### A1 — source CLI broken by S3

`../trisha/cli/compile.rs:24` calls `trident::compile_to_bundle`, which now rejects stack targets in `src/api/mod.rs:505`. Both `trisha run/prove input.tri` and delegated `trident run --target triton input.tri` exit 1. Source deploy shares this path. The migrated integration suite constructs its own bundle using `build_tasm`, bypassing the broken CLI helper.

Repro: `program audit` followed by `fn main() { pub_write(42) }`. `trisha build` succeeds. `trisha run` and `trisha prove` report "trident no longer lowers to 'triton' assembly in-process".

Control: `trisha run --tasm audit.tasm` outputs 42 in 5 cycles; `prove --tasm` writes a real proof (padded height 256); both Trisha verify and delegated Trident verify report PASS. The raw runtime/prover works for this case; source integration does not.

### A2 — nox ignores conditional compilation

`src/ir/tree/lower/nox.rs:351` collects every function; the API passes an unfiltered AST. Repro:

```text
program cfg
#[cfg(debug)]
fn pick() -> Field { 1 }
#[cfg(release)]
fn pick() -> Field { 2 }
fn main() -> Field { pick() }
```

Fresh Joy run outputs 2 for both `--profile debug` and `--profile release`; debug must output 1. Existing compiler defect, independently blocking the requested nox release.

### A3 — nox breaks lexical shadowing

`src/ir/tree/lower/nox.rs:1038` checks global constants before local scope; `eval_const` at line 842 repeats that assumption. `const X: Field = 5; fn main() -> Field { let X: Field = 7; X }` outputs 5 instead of 7 (using normal newline-separated Trident declarations). Fix both execution and constant folding, including loop/index expressions.

### A4 — nox loses imported function definitions

`src/api/mod.rs:162` lowers only the entry module after resolving/typechecking the project. `use math` with `math.add(3, 4)` and an adjacent module declaring `pub fn add(a: Field, b: Field) -> Field { a + b }` fails with "call to unknown function 'math.add'". The architecture needs qualified resolved-module lowering; flattening bare names would create collisions.

### A5 — current Zheng artifacts cannot be saved

Lens now creates `Opening::TensorMerkle` (`../lens/brakedown/src/lib.rs:224`). Zheng's wire adapter accepts only `Opening::Tensor` (`../zheng/rs/src/wire.rs:37`). Seven Joy integration tests fail; several tamper tests fail before reaching their mutations.

Fresh `joy run rs/tests/fixtures/add.tri --input-values 3,5` returns the correct 24 for `(a+b)*a`. Fresh `joy prove` on the same source exits 1: "cannot serialize proof artifact: Serde Serialization Error". Updating documentation or weakening tests cannot repair this contract.

Also inspect the old-Tensor-only branches in Zheng's `ccs/verifier_steps.rs` and `ccs/transcript.rs`; do not release by merely making serialization accept the new variant. Opening authentication and recursive verification need adversarial regression coverage.

### A6 — CLI target and failure contracts are inconsistent

`src/cli/build.rs:61–79` resolves a project's Triton target but finds/forwards the original default nox target. Run/prove select the warrior before applying the project target. Executable stubs confirm all three choose `trident-nox` for a project with `target = "triton"`.

With PATH limited to system binaries, Trident run/prove/verify return exit 0 when no warrior is available; verify does so even for a nonexistent proof file. Their install hint names Trisha for nox. Separately, `../trisha/rs/lower/mod.rs:327` ignores target-resolution errors: `trisha build --target garbage` produces TASM and exits 0.

### A7 — transfer and architecture record disagree

Triton lowering/linker, AET costs and neural implementation moved to Trisha. `os/neptune` and hand baselines remain in Trident; migrated tests still locate `../../trident/os`. The S3 completion claim exceeds the transfer performed.

The plan's recorded owner decision keeps the generic neural harness in the core with a warrior target implementation. S3 moved the whole neural subsystem. Reconcile this explicitly with the recorded design. Warrior API documentation still describes the removed neural feature and pre-S3 bundle path; CLAUDE pipeline/key-module descriptions are stale.

### A8 — installation and coordinated versions are unfinished

Trisha depends on sibling source paths and ignored patched `.vendor` directories. A crates.io release strips workspace patches, so adding dependency versions alone is insufficient. A fresh clone needs bootstrap/dependencies; no independent distribution has been verified. README points `cargo install --path .` at a virtual workspace.

Trident finds libraries beside its executable, through cwd ancestors, or environment variables (`src/config/resolve/mod.rs:54–111`). Copying the binary to an isolated directory breaks a source importing `vm.core.field`; the repo binary succeeds. Cargo package validation alone does not prove usable installed libraries.

Breaking compiler/API changes still carry 0.2.0. Release versions and Joy/Trisha dependency requirements must be coordinated. The migration plan names incompatible release sequences; replace them with one current sequence. No joint runtime/release CI gate exists.

## Additional release defects found by inspection

- Trisha `rs/warrior.rs:131` returns a digest as deployment success without submitting a transaction. Batch CLI reports "deployed". Execution of a live deployment was deliberately not attempted.
- Trisha CLI always selects the CPU warrior; README's default GPU claim is unsupported.
- Trisha `cli/neptune.rs:76` ignores requested address index and key type.
- Trisha `cli/main.rs:103` drops trailing digest words; `rs/convert.rs:54` truncates or zero-fills malformed claim hashes. Proof-format validation is missing.
- Trisha batch proving derives filenames from basename only, so distinct input directories can overwrite each other's artifacts.
- Joy `cli/verify.rs:56–67` compares `--claim` to metadata output, while `rs/warrior.rs:452–464` authenticates assembly and proof, not that output metadata. Prove the claimed output's binding before presenting it as verified.
- Existing trident issue #40 records out-of-range Triton `dup 24` and invalid hand baselines. It remains open; no clean full benchmark run was established in this audit. Neither CLI now exposes the planned `bench` command.

## Limits and next step

Successful unit suites establish useful local coverage, not full language support or release readiness. The nox standard-library census currently covers two modules; its test explicitly pins that restricted surface. GPU proving, live Neptune deployment, clean-machine release installation and a complete adversarial proof audit were not validated.

The repair/release gates are in `../plans/warrior-release-repair.md`. The repository's `reference/quality.md` audit protocol requires confirmation of the concrete fix plan before implementation changes.
