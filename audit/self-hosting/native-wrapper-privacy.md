# Native wrapper ownership prerequisite

Date: 2026-09-24. Trident source:
`2c66db3cb133ee6b461b7dc9f6e3cb0662a59c54`.
Joy fixture: `612cfa1373ad2070dc105a39a59cc4633cd6a8b0`.
Trisha fixtures: `f472d6bad61f42626dba33bfc872bbcb324299a2`.
Full dependency pins, commands and log digests:
[validation receipt](sh1-privacy-validation.json).

## Problem and correction

The documented private-field rule was unenforced at construction, reading,
mutation and patterns. Semantic struct equality also used only a short name
and layout. A caller could both access an imported wrapper's internals and
pass a same-shaped local structure in its place. Either bypass would undermine
the planned bounded Seq/Bytes validators.

Semantic structures now carry their defining module. Imports, nested field
types, ordinary return signatures and generic environments preserve that owner.
All field access sites check visibility; layout remains complete for lowering.
Qualified diagnostics distinguish nominally different structures.

The module closure rejects duplicate parsed declarations before specialization.
Resolver discovery keys alone are insufficient: a differently named import or
a commented/tab-separated entry header could previously conceal another file
declaring the owner's name. The new guard rejects those examples.

The existing U256, hash-state, signature and quantum arithmetic APIs use raw
public data. Their externally consumed fields now say `pub` explicitly. Curve
point fields, SHA message schedules and LUT-sponge state remain private.
Imported-layout fixtures in all three repositories now declare the fields
they intentionally read as public. Algorithms and baseline programs are unchanged.

## Validation

Commands ran before committing the listed source revisions; those source and
test files were committed unchanged. Contract documentation was finalized
before commit. This is local development evidence, separate from release or CI.

| Repository / command | Result at listed source revision |
|---|---|
| Trident: `cargo test --workspace --locked` | 898 passed, no failures or ignored tests |
| Trident: `cargo check --workspace --all-targets --locked` | Pass, no Rust warnings |
| Joy: `CARGO_TARGET_DIR=../trident/target cargo test --workspace --release --locked` | 89 passed, no failures or ignored tests |
| Joy: `CARGO_TARGET_DIR=../trident/target cargo check --workspace --all-targets --locked` | Pass, no Rust warnings |
| Trisha: `CARGO_TARGET_DIR=../trident/target cargo test --release --locked -p trisha-rs` | 362 passed, no failures, 4 existing heavy proof tests ignored |
| Trisha: `CARGO_TARGET_DIR=../trident/target cargo run --release --locked -p trisha -- bench` | 133/133 fixtures, 43/43 independent baselines verified |

Every fixture result/cycle row matches the previous control-delivery bench at
Trident `4317c767607ce83210348d285c1390df0bed5d94`. Earlier failing fixture
and benchmark gates, caused by undeclared public fields, remain in the receipt.

New tests reject forged local copies, private constructor fields, reads/writes
through aliases, arrays and nested structures, forwarded/generic results and
duplicate-owner source files. Positive tests execute owner-provided constructors
and updates on nox. Shared TIR and editor checks use the same admission gate.
The pattern test exercises the semantic AST explicitly: qualified pattern syntax
is not yet parsed. Local private patterns remain valid.

Formal `trident audit` runs on the changed library declarations report UNKNOWN
for unsupported analysis; they establish no proof of the libraries. ECDSA's nox
audit correctly rejects its streaming input requirements; the explicit Triton
audit reports UNKNOWN. Exact commands and statuses are in the receipt.

## Scope and next step

This is source-level ownership. A general fixed-word entry ABI can still
materialize typed scalar structures from external words, and stack assembly has
its own trust boundary. Native raw entry is exactly Noun-to-Noun, so native
wrappers must be obtained through their validating source functions.

Seq/Bytes implementation and production Joy compiler-job admission remain open.
This prerequisite does not close SH1 or establish self-compilation/proving.
