# FINAL5 architecture and ownership review

Reviewed 2026-09-12 against frozen FINAL5 source archive SHA256
`7c63fcad9d12a220036b531f85eacc1a91984ff87c18687703aeeac57b3df415`.
This review does not publish a release or replace the separate 198-proof gate.
Production was initially inspected read-only; the two concrete shared-TIR findings
below were subsequently authorized for repair in live source. Frozen FINAL5 is
unchanged.

## Actual ownership

| Concern | Implementation owner and evidence |
| --- | --- |
| Language, type checking, portable algorithms | Trident `src/typecheck`, `lib/std`, `lib/vm/core`, `lib/vm/io` |
| Generic TIR and typed entry metadata | Trident `src/ir/tir`; API 3 `EntryParameters` describes primitive leaves, not Triton instructions |
| Reference nox semantics | Trident `src/ir/tree/lower/nox.rs` and `nox/`; this direct AST path is intentional, not a second Triton runtime |
| Triton ISA, entry marshaling, linking, instruction bounds | Trisha `rs/lower/{triton,entry,legalize,target_call,linker}.rs` |
| Native execution and STARK proof boundary | Trisha `rs/warrior.rs`, `rs/convert.rs`, `rs/recursive`; Triton 7/native claim version 5 |
| Neptune machine/network SDK and fixed consensus policies | Trisha `lib/os/neptune`, `networks/neptune`, `rs/recursive/neptune.rs`; transport/transaction adapter in `neptune/` |
| Hand assembly and independent reference fixtures | Trisha `baselines/triton`: all 43 `.tasm` files; no `.tasm` or `baselines/triton` files remain in Trident |
| nox runtime and public/private/state execution artifacts | Joy, using upstream nox/Zheng and the Trisha private checker |

Trident's Cargo manifest has no Triton VM, tasm-lib or Neptune dependency.
Its `catalog/vm/triton/owner.toml` and `catalog/os/neptune/owner.toml` contain
external-owner discovery metadata. The owner names and terrain/union matching
in `src/config/target/discover.rs` are dispatch and identity checks, not machine
implementations. Other catalog designs do not establish executable support.
There is no portable `trident/lib/os` implementation to migrate. Language-level
`os.state.read` is a semantic operation; the state certificate, runtime and
proof boundary belong to the runtime owners.

## Installed resources and API 3

Trident `build.rs` embeds sorted `lib` and `catalog` source/descriptors.
Trisha `rs/build.rs` separately embeds its `lib`, `targets` and `networks`.
`rs/target.rs` exports Triton modules for `triton`, adds Neptune modules/states
and canonical policy intrinsics only for `neptune`, and seals the package.
Joy `rs/target.rs` uses the canonical upstream nox ABI and embeds its own runtime
capabilities. The retired handwritten Neptune proof implementation is outside
the production SDK; the production recursive entry belongs to Trisha.

`TargetPackage::validate` checks compiler API/schema, bounded ABI/module data,
module declaration identity and hashes, selected OS namespace, union/terrain
and state identities. `CompileOptions::with_package` takes the package sources
and ABI together; `validate` rejects substituted ABI, including noncanonical
nox shapes. Generated generic hash/I/O declarations derive their dimensions
from the selected ABI. Reachable intrinsic capability checks remain necessary:
an unused portable module declaration does not assert runtime availability.

Resolver precedence is explicit overlay, supplied package sources, explicit
dependency, compiler embedded library; unsupplied reserved `std`/`vm`/`os`
modules do not silently fall back to an ambient checkout. An overlay is an
explicit editor input, not a runtime provider replacement.

Sixteen selected implementation/reference files (resources, builders,
package/discovery, API, Trisha entry/target/build, Joy target/capabilities and
three canonical references) were compared byte-for-byte between live and
frozen FINAL5 before repair and were identical. Installed FINAL5 Linux and
Darwin smoke receipts additionally exercise these resources outside the source
checkout. See Trisha `audit/final5-linux-validation.md` and
`audit/final5-darwin-receipt.json` for exact binaries and proof interoperability;
these receipts are stronger evidence than a catalog listing.

## Concrete findings and live repairs

1. `src/ir/tir/builder/expr.rs` tracked `XFieldMul` results as three words even
   for a different selected extension width. FINAL5 Triton uses three words;
   direct nox does not use this TIR path. The live repair uses
   `target_config.xfield_width`. The regression checks widths 2, 4 and 7 and
   observes the exact depth of a retained neighboring value after the product.
2. `src/ir/tir/builder/legalize.rs` redundantly lowered deep Dup/Swap into
   compiler scratch RAM according to the target stack window. Although typed
   and not literal TASM, this violated the agreed owner-only machine-window
   boundary. The live repair removes that module and the build-file final pass.
   Trisha's existing machine legalizer remains authoritative. Compiler scratch
   for semantic temporaries, indexing and generic spill management is retained.
   Native program identities/costs may change; FINAL5 proofs cannot be relabeled
   as proofs of changed native programs.

## Documentation contradictions requiring cleanup

- `reference/os.md` says Joy rejects state requests and lacks authenticated
  public proof support. Current Joy capabilities and actual FINAL5 smoke support
  bounded authenticated public BBG certificates, JOYST001, and hidden query
  coordinates over complete public tables through JOYZK003. There is still no
  live Cyber state synchronization/deployment or private database guarantee.
- The early API example in `reference/warrior-api.md` recommends
  `TerrainConfig::triton()` and old `vm/<engine>/target.toml` paths. The former is
  test-only; runtime integrations must use the owner package resolution APIs.
  Its claim that supplied module sources only apply when no file exists also
  disagrees with the explicit package-source precedence above.
- Earlier wording in that reference calls the migration interfaces merely
  proposed, while its later API 3 section documents the implemented contract.
- Trisha architecture previously described compiler final-pass deep access
  legalization. The live description now assigns it to the owner lowerer.
- `audit/target-ownership.md` contains explicitly dated historical coverage and
  platform gaps. Those are history, not current support claims; use current
  candidate receipts and the active release ledger for readiness.

The supported runtime matrix is narrower than the catalog: Trisha provides
Triton/Neptune CPU execution and STARK proofs; generic deployment remains false,
with a separate validated-intent Neptune CLI demonstrated only at the isolated
local node boundary. Joy provides nox and its `cyber` alias, public execution,
private Triton-backed execution, and bounded authenticated public-state proofs.
GPU mining selection does not select a GPU proving backend. None of this review
claims public-network admission, block confirmation or unrestricted proof shape.

## Live repair validation

The authorized repairs completed with these bounded checks:

- Core TIR suite: 51 passed, zero failures/ignored, 629 filtered.
- Trisha `target_ownership`: 11 passed, zero failures/ignored. Raw deep Dup/Swap
  checks every word at depths 16, 17, 31 and 64; the new source case reverses a
  40-word array through a callee and verifies the returned array, original
  caller array and scalar sentinel in both debug and release profiles.
- Alternate XField widths 2/4/7 preserve the neighboring stack value's exact
  access depth. No arithmetic semantics for an unimplemented target are claimed.
- Independent custom-token canonical codec/own-program fixture generator:
  one selected test passed. The initial unchanged-fixture run retained the
  expected three stale self-hash rejections (130/133 passed). Only `input` and
  `secret` fields of the three positive custom-token fixtures were regenerated;
  all hand streams, expected outcomes, amounts and negative fixtures are unchanged.
- Fresh full execution with both exact FINAL5 and live binaries: each 133/133
  fixtures and 43/43 baselines passed. Of 99 positives, 48 use fewer cycles and
  51 are unchanged; none increases. All hand cycles and 34 rejection statuses
  are unchanged. Total source cycles across these fixtures decrease by 15,283.
- Actual native Program hashes change in 40 of 84 distinct source/target pairs;
  44 are identical. This is a real program-identity change, not whitespace.
  There were no new proof generations in this repair gate.

Machine-readable before/after native hashes, all 133 cycle rows, binary hashes,
fixture rebinding identities and log hashes are preserved in
[`owner-stack-legalization-receipt.json`](owner-stack-legalization-receipt.json).
Frozen FINAL5 proof receipts continue to describe FINAL5 only; changed live
programs require fresh proof evidence before their own release candidate.

Files changed for this repair: `src/ir/tir/builder/expr.rs`,
`src/ir/tir/builder/mod.rs`, deletion of `src/ir/tir/builder/legalize.rs`,
`src/ir/tir/builder/tests/advanced.rs`; in Trisha,
`rs/tests/target_ownership.rs`, `docs/explanation/architecture.md`, and the
three `baselines/triton/fixtures/custom-token-v2-{burn,mint,transfer}/vector.bench.toml`
files. No owner ISA algorithm needed modification: the existing Trisha deep
access implementation passed the source and raw-IR checks unchanged.

Rust compilation emitted no compiler warnings in these targeted gates. The
existing `target_ownership` negative/source fixtures print two language warnings
for unused imports (`vm.crypto.hash` and `os.neptune.xfield`); these are recorded,
not counted as zero diagnostic output. Whitespace checks passed for the edited
Rust and architecture files.
