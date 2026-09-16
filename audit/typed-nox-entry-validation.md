# Typed nox entry validation — 2026-09-12

The accepted source signature now governs external public input inside the
compiled formula. [Canonical contract](../reference/nox.md). This is part of
compiler API3; hand-authored raw nox retains its own subject ABI.

## Reproduced defects

The preserved FINAL3 Joy binary accepted U32 input4294967296 and returned it;
accepted unused Bool2; accepted missing unused Field input; and interpreted
three inputs7,19,31 for two parameters as19,31, returning1931. A valid Pair
entry with flat public input7,19 failed with an axis error. Exact sources,
commands and results: `/tmp/nox-entry-abi-review-20260912/receipt.json`.
These probes were executions, not proof-generation or forgery evidence.

## Implementation and boundaries

The compiler consumes exactly the flattened signature, checks every leaf even
when unused, reconstructs internal list/balanced-Digest shapes, and preserves
any authenticated BBG root. Guards are formula operations, so exported .nox
and verifier-derived execution relations share the checks. Bool keeps the
native nox0=true/1=false encoding; U32 requires less than2^32. Entry shape
bounds prevent unbounded generation and are specified in the reference.

An independent review found imported array dimensions losing their lexical
module and long aggregate paths truncating to one u64. Lexical size resolution
now preserves module constants and generic parameters. Reads use consecutive
canonical axes; edits rebuild siblings from the original subject and evaluate
the RHS exactly once. The original deep source now executes successfully.
[Independent findings and closure](nox-entry-abi-review.md).

## Executed verification

- Full native nox source surface: **45 passed**, no failures/ignored/warnings,
  both source profiles included. `/tmp/nox-typed-entry-reviewed-surface.log`.
  This includes every aggregate input leaf, narrow bounds, missing/extra input,
  malformed nouns, nested imported layouts, balanced Digest, long read/write
  paths, preserved siblings, ordinary calls, state and prior loop regressions.
- Joy typed entry: **3 tests passed**, no failures/ignored/warnings,50.11s in
  release with4 Rayon threads and no trace cache.
  `/tmp/joy-typed-entry-proofs-release.log`. The tests generate four public
  execution certificates across two profiles and one actual private Triton
  STARK. Freshly decoded verification succeeds; mistyped/mis-sized input and
  changed public result reject. Constant output11 does not let a bad unused
  parameter pass the public execution verifier.
- Imported lexical size unit and independent installed source regressions
  passed separately; exact logs and original/fixed values are in the review.

An initial native test helper placed an unnecessarily large arena on its test
thread's stack and stalled before execution; that run was cancelled. The
bounded4096-node helper passes the native suite. An initial debug-mode private
proof run was stopped before completion and replaced by the completed release
run above. Neither interrupted run is counted as a pass or an additional proof.

These are source/workspace checks. The next frozen candidate still needs its
complete installed build/smoke, exact binary hashes and distribution receipts.
General dynamic nox relations and whole-library formal assurance remain distinct
open release-ledger requirements; this result does not claim them.
