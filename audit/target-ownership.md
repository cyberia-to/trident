# Target ownership review — 2026-09-11

Status: approved migration implemented and committed; installed and archived
distribution checks passed on Darwin arm64 CPU. Full proof-release gates remain open.
The initial findings below are retained as the audit trail. The implementation
checkpoint records their disposition. Current contracts are in
[warrior-api](../reference/warrior-api.md).

## Finding

The bulk Triton migration is real, but its semantic boundary is incomplete.
Trident has no tracked baseline files; its empty `baselines/` directory was
removed during this review. Trisha tracks 45 files under `baselines/triton/`:
43 TASM programs, one source fixture and one execution fixture. Current benchmark
instructions now point to Trisha. Historical baseline coverage remains 1/43;
physical ownership does not establish correctness of the other 42 programs.

The review inspected tracked assets, manifests, resource loading, target
resolution, typechecking, shared TIR, packaging and documentation. It is a
static ownership audit, not a new full runtime or cryptographic release audit.

## Inventory before this migration

| Location | Actual contents | Architectural role |
|---|---|---|
| `trident/vm/` | 8 `.tri`, 21 target TOML files, 21 Markdown files | Intrinsic declarations mixed with machine catalog |
| `trident/os/` | Zero `.tri`, 25 target TOML files, 7 state TOML files, 25 Markdown files | Protocol catalog and deployment presets |
| `trisha/os/neptune/` | 18 `.tri`, 3 TOML files, one Markdown file | Neptune SDK plus duplicated descriptors |
| `trisha/baselines/triton/` | 45 tracked files | Triton expert assembly and validation fixtures |

Counts refer to files present at review time. Directory names and namespaces
currently coincide, but they represent different responsibilities.

## Violations found before implementation

| Area / evidence | Consequence | Destination or correction |
|---|---|---|
| `trident/std/target.tri` | Claims current-target constants but fixes digest=5, rate=10, stack=16; nox uses 4, 8, 0 | Generate constants from the resolved compilation target |
| `trident/vm/crypto/hash.tri` | Calls the intrinsic `tip5` with ten inputs, while nox's native hash is Hemera | Explicit Tip5 belongs to Triton SDK; a generic native hash needs its own documented semantics |
| `trident/vm/crypto/merkle.tri`, `std/crypto/merkle.tri` | Five-word digest layout and Triton nondeterministic digest queue | Separate portable algorithm from target witness ABI; reject absent capabilities |
| `trident/vm/io/io.tri` | `read5` / `divine5` return a target-sized Digest | Distinguish fixed-size tuple reads from native Digest reads |
| `trident/std/crypto/auth.tri` | `lock_and_read_kernel` implements Neptune kernel-MAST policy | Move Neptune policy to Trisha; retain only truly generic preimage helpers in std |
| `trident/src/typecheck/mod.rs` | `TypeChecker::new` defaults to Triton, unlike compilation options defaulting to nox | Pass the same resolved target into build, check, audit and LSP |
| `trident/src/ir/tir/optimize/mod.rs`, `builder/helpers.rs` | Shared optimizer hardcodes pop/hint batch 5 and swap depth 15 | Keep semantic optimizations in core; instruction legality belongs to Trisha |
| `trident/src/ir/tir/stack/mod.rs`, `builder/helpers.rs` | Stack manager emits TASM strings which the builder reparses | Emit typed IR operations directly; render assembly once in the warrior |
| `trident/src/config/target/mod.rs` | Triton resolution returns a Rust literal before consulting its TOML | One authoritative descriptor, generated/loaded consistently |
| Neptune target and two state TOML files in both repos | Byte-identical duplicated ownership, plus separate `trisha/cli/state.rs` constants | One Trisha-owned protocol/state dataset consumed by CLI and compiler adapter |
| `trident/src/config/target/state.rs` | State lookup uses filesystem only despite embedded state resources | Resolve state through the owning runtime package, independently of checkout |
| `trident/src/config/resolve/resolver.rs` | Ambient files precede embedded warrior modules | Explicit, versioned module providers; deliberate overrides recorded in build identity |
| `trident/src/deploy/mod.rs`, `cli/package.rs`, `cli/deploy.rs` | Generic package paths use `program.tasm` | Use the resolved artifact format/extension |
| `trident/src/lsp/semantic/asm.rs`, `lsp/builtins.rs` | Triton instruction/hash descriptions used generically | Target-specific editor metadata supplied with the target description |

`vm/*/target.toml` status levels also describe removed Miden/Nock/register
lowerers. `reference/targets.md`, `reference/ir.md` and `src/README.md` retain
pre-migration architecture claims. `reference/os.md` describes portable
`os.neuron`/`signal`/`event` modules that do not exist as implementations.
The implemented `os.state.read` compiler builtin has a much narrower surface;
it does not establish working state proofs.

## Adopted ownership

Keep four source namespaces: `vm.*`, `std.*`, `os.*`, `os.<network>.*`.
Choose their implementation by explicit package and target, independently of
physical directory layout.

- Trident owns language semantics, frontend/typechecking, shared IR, portable
  algorithms, generic intrinsic contracts, compiler tools and generic neural
  infrastructure. It retains the existing nox lowering as the reference
  compilation path. Nox execution belongs to nox, proofs to Zheng, their
  warrior integration to Joy.
- Trisha owns Triton instruction selection, legalization, linking, costs,
  execution/proof integration, ISA metadata, Tip5/Triton ABI bindings and
  expert TASM. It also owns Neptune SDK, transaction policy, node integration
  and public deployment presets.
- Joy owns nox warrior execution/proof capabilities and cyber integration.
  The nox machine contract should come from nox and be consumed consistently
  by the compiler and Joy; moving nox lowering to Joy is a separate change.
- A discovery catalog records names, ownership and declared targets. An
  installed implementation provides actual capabilities. A catalog entry
  alone never promises build/run/prove/deploy support.
- Chain-specific endpoints and genesis/state identity belong to the runtime
  owner. User endpoint overrides belong to project/user configuration.
  `soft3` remains an integration/client layer; compiler ISA details do not
  belong there. Protocol doctrine belongs in the cyber graph.

## Adopted physical layout

```text
trident/
  src/                         frontend, typecheck, shared IR, nox lowering
  lib/
    vm/                        generic intrinsic contracts
    std/                       portable libraries
    os/                        implemented portable runtime contracts only
  catalog/
    vm/                        discovery records and declared target docs
    os/                        network owner references and design status
  benches/                     portable inputs and independent references

trisha/
  rs/lower/                    Triton selection, legalization and linking
  rs/cost/                     Triton cost model
  rs/                          CPU execution/proof adapter
  cli/                         build/run/prove/verify + Neptune commands
  wgpu/, honeycrisp/            backend implementations and explicit stubs
  targets/triton/               authoritative machine/ABI/editor metadata
  lib/vm/triton/                explicitly Triton intrinsics
  lib/os/neptune/               Neptune SDK and transaction policy
  networks/neptune/             network manifest and states/*.toml
  baselines/triton/             expert TASM + reference fixtures

joy/
  rs/                          nox/Zheng warrior integration
  targets/nox/                 warrior capabilities referencing nox contract
  lib/os/cyber/                implemented cyber bindings, when available
```

The empty/future library locations are explanatory, not directories
that should be scaffolded now. Keeping `baselines/triton/` in Trisha preserves
its useful namespace and avoids cosmetic renaming.

## Resource and capability contract

Use the existing `CompileOptions.module_sources` as the migration starting
point. Introduce a small versioned target-package description containing
machine identity/version, compilation ABI, module identities and content
hashes, compiler API compatibility, and required/available intrinsics.
Runtime capabilities separately describe run/prove/verify/deploy, supported
execution subsets and proof formats. The current public Zheng certificate
and a future private proof are distinct capabilities.

A minimal built-in discovery map can route `nox -> joy`, `triton -> trisha`
and `neptune -> trisha` without carrying duplicate network/ISA definitions.
An installed warrior exports its package description without executing the
user program. The transport is schema-1 JSON from `describe --target`, or an explicit
`TRIDENT_TARGET_PACKAGES/<target>.json` file. It introduces no dynamic plugin
framework or additional repository. TIR remains the in-process Rust contract.

Resolve the package once and use it for CLI, library APIs, LSP, typecheck,
codegen and bundle metadata. Release builds use embedded or locked sources;
filesystem overrides must be explicit. Do not silently substitute Hemera
for Tip5. A native-hash API may be target-dependent only when that difference
is visible in its contract and artifact identity.

Separate compilation identity (machine ABI, modules, compile-time network
bindings) from deployment selection (state, endpoint). Compiler-facing
manifests and SDK modules must travel with installed packages, with no
sibling checkout dependency. State selection should not change assembly
unless a program explicitly requests a compile-time state value.

## Migration order and gates

1. Specify target/package/capability ownership in canonical reference; correct
   stale capability claims. Keep discovery working while providers are added.
2. Unify resolved-target propagation and generated `std.target`; exercise
   digest/hash differences through check, build and LSP for nox and Triton.
3. Split the SDK by semantics: move explicit Triton intrinsics and Neptune
   auth; update all imports, fixtures and wrappers together. Unsupported
   operations fail at typecheck with the missing capability.
4. Remove ISA string generation and limits from shared TIR; verify typed
   stack effects, wide values, deep access and actual emitted Triton programs.
5. Transfer full target/state metadata to its single owner, implement package
   resource lookup, then perform the `lib/`/`catalog/` directory move. Update
   build embedding, package inclusion, editor navigation and documentation.
6. Verify isolated installed Trident+Joy and Trident+Trisha, both supported
   proof paths, failure on unsupported capabilities, target-correct package
   extensions and deployment-state lookup. Benchmark coverage must report
   every unverified baseline honestly.

Moving all of `vm/` into Trisha would discard language intrinsic contracts.
Moving `os/` as a whole would give Trisha ownership of unrelated platforms.
A directory-only move would preserve the width, typecheck and ABI defects.
The semantic split therefore precedes the physical reorganization.

## Implementation checkpoint

All source paths in this table describe the migrated layout.

| Gate | Implementation and evidence |
|---|---|
| Target identity | Versioned `TargetPackage`, validated module bytes/declarations, ABI bounds, compiler compatibility and selected network. Descriptor and CLI dispatch share one provider lookup. Unsupported commands fail before dispatch. |
| Semantic capabilities | Complete library declarations remain available; transitive requirements of reachable functions are checked against the selected package. Inactive cfg declarations and unused unsupported helpers do not create false requirements. Invalid intrinsic signatures always fail. |
| Compiler/editor parity | nox default for checking; project/explicit targets reach check, build, audit and LSP. Live editor buffers are resolved without temporary source writes. Single-source APIs validate package identity and selected-entry requirements too. |
| Portable ABI | `std.target`, native hash, Digest and I/O contracts derive from the selected target. Explicit Tip5/Merkle bindings live in Trisha. Neptune auth moved to its SDK; portable scalar preimage checks remain in `std.crypto.preimage`. |
| Typed TIR | Stack manager emits typed operations; string reparse and shared Triton legalization limits removed. Trisha owns batching and deep stack access. Independent interpreter regressions cover stack operations; actual VM tests cover emitted code. |
| XField correctness | Corrected five-word dot-step signature and tuple destructuring. Real Triton execution exposed reversed coefficient order in dot-step and inverse lowering; independent arithmetic expectations now pass. Checked U32 casts reject overflow, and local function shadowing is preserved. |
| Metadata ownership | Trisha owns machine manifest, intrinsic/ISA lists, Neptune SDK and network/state data. CLI state presets are generated from the same manifests. Joy owns its runtime capability descriptor. Core retains the canonical reference nox machine contract. |
| Resource layout | Trident `lib/` and `catalog/`; Trisha `lib/`, `targets/`, `networks/`. Embedded resources replace checkout-dependent resolution. Other catalog machines explicitly remain declared designs. |
| Artifact packaging | Artifact extension follows the selected machine; manifests name and authenticate the actual program file. CLI packaging delegates foreign lowering, preserves network identity and hashes effective sources/ABI/cfg. Unknown foreign costs are absent. |
| Runtime guards | Trisha CPU and GPU adapters reject wrong target/state bundles. Raw assembly and proof formats validate target too. Joy rejects unsupported state inputs. Runtime metadata distinguishes public Zheng certificates from private proofs. |
| Recursive Neptune SDK | The former `verify_inner_proof` computed intermediate values without asserting validity. Removed from production packages together with dependent entry programs; preserved under `examples/experimental/neptune` with an explicit unimplemented status. Production imports fail. |

Final source-suite checkpoint: Trident 755 default + 44 neural tests pass;
Trisha 227 default + 50 neural tests pass. All-features checks pass; Trisha
has three pre-existing warnings in its vendored dependency. These totals include static/library tests;
they do not imply execution coverage for every standard-library function.

Joy's complete suite has 46 passing tests and three retained state-proof
acceptance failures (`UnsupportedRecursiveOpening`). Its new public execution
and installed-package tests pass. The failures require authenticated recursive
state constraints; changing ownership cannot make them pass.

Full release gates still include private/state proofs, recursive Neptune
verification and transaction validation, live deployment, and executable
reference fixtures for all claimed baselines. The historical benchmark is
1/43 verified. These remain explicit open work; no release was published by
this migration. Final installed/source/binary candidate checks and hashes are in
[warrior-release-validation](warrior-release-validation.md).
