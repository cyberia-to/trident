# External dependency changes before FINAL6

2026-09-12. Read-only review of BBG, Honeycrisp, Neuron and Strata changes listed
in `/tmp/final5-live-source-delta.json`, compared with frozen FINAL5 at
`/tmp/cyber-release-extracted-v7-final5/cyber-source`. Hemera is under a separate
cryptographic review; no conclusion about its changes is implied here. External
production code, manifests and locks were preserved.

## BBG

Five storage Rust files and one new specification change reader compatibility:
`storage/database/generation.rs` introduces Original < NeuronV1 < AuthenticatedV1
using the existing migration status values absent/complete, neuron-v1 and
new auth-v1. Promotion is monotonic and transactional. Migration and transfer
preparation now promote instead of overwriting an existing stronger marker.
Normal database open rejects unknown/incomplete generations; prior readers
reject the new auth-v1 marker. New tests cover durable promotion, no downgrade,
unknown/incomplete statuses and migration preserving AuthenticatedV1.

This is an intentional persisted storage-reader contract change. It does not
make BBG the network signature verifier: the owning authenticated adapter must
enforce the marker's signed-submission semantics, as the new spec states.

Product metadata resolves BBG with `serde`, without SSD/HDD storage backends.
The changed storage code belongs to a product dependency package, but the Joy
state-certificate loader does not open a persistent storage database. Exact
byte comparison confirms `certificate.rs`, `proof.rs`, and `state/commits.rs`
unchanged from FINAL5; the full source delta contains no certificate/public
state serialization edits. Old certificate fixture bytes are not invalidated by
this storage marker change. This does not pre-approve separate Hemera changes.
No storage migration was run against user data in this review.

## Honeycrisp

The sole delta removes `version = "0.1.1"` from the workspace `nebu` dependency
while retaining its `../strata/nebu/rs` path/package. `acpu` inherits that
workspace dependency. Locked product metadata resolves exactly the same local
strata-nebu package/version/path before and after, so there is no product
resolution change here. A path-only requirement is less restrictive for future
edits; candidate source/lock inventories must continue pinning actual bytes.

## Neuron

New `model/action.rs` defines bounded, versioned public signed-action envelopes;
`model/Cargo.toml` extends its serde feature with optional serde_json, and the
node authority delegates existing statement signatures to the owning Mudra
helper. The external workspace lock records these and other workspace member
requirements. These are real Neuron API changes, not merely formatting, but
neither neuron-model nor neuron-node is in the product dependency closure.

The only product Neuron dependency remains dependency-free `neuron-id` 0.1.0.
Its manifest and source are unchanged; `neuron/id/src/lib.rs` was compared
byte-for-byte. Product identity bytes and public state serialization therefore
do not inherit the new action envelope. No claim is made here that the entire
external Neuron node/mudra application graph passed a new runtime suite.

## Strata

`cli/src/main.rs` only changes import ordering and expands an if-expression
across lines. It is not in the product closure. The resolved strata-nebu
library identity is unchanged. No algorithm/vector or wire-format delta was
found in this reviewed Strata change.

## Manifest and lock evidence

Ran `cargo metadata --format-version 1 --locked --all-features` separately for
Trident, Trisha and Joy in both the frozen tree and live checkout. All six
commands succeeded without lock updates. The union of local
(name, version, relative manifest path) tuples is **41 packages on both sides,
with no additions, removals or identity changes**. This is the three-root
all-features denominator, not the source packager's narrower root count.
Resolved relevant features are unchanged: BBG serde; neuron-id none; acpu none;
aruminium none; strata-nebu default+serde.

Exact metadata: `/tmp/external-dependency-continuation-metadata.json`.
Reviewed file identities: `/tmp/external-dependency-continuation-files.json`.
Every current file in this assigned delta still matched the root-supplied SHA
at the end of review. No concrete product closure or public-format blocker was
found in these four repositories. Final source stability still requires the
separate Hemera review, other owners' completion and a fresh frozen inventory;
this local conclusion does not declare the whole candidate stable.
