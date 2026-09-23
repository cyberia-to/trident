# SH1 native Noun and source ART1 — 2026-09-23

Delivered source-language data foundation, not a self-hosting claim.

The compiler accepts native Noun parameters/results/locals and containing
aggregates; `vm.nox.noun` supplies seven checked pure operations. Width is
optional: native trees never acquire a fabricated field-word width. Shared TIR
has a checked builder boundary, including inferred native calls, imported
layouts and deferred generic bodies. Foreign targets, constants, event/flat-I/O
layouts, implicit arithmetic/equality/index/loop-bound conversions reject Noun.

The separate seed API emits complete canonical ART1(0,0,0,formula) for exactly
`fn main(input: Noun) -> Noun`. Joy compiles it with `build --emit artifact` and
executes it with `run-artifact`. No bracket serialization or flat output adapter
occurs between them. Full transport limits, pure-profile rejection and atomic
publication remain enforced. The old flat entry rejects Noun-bearing signatures.

## Review findings fixed

- Native `Digest` is balanced [[a b][c d]], while tuple destructuring previously
  read it as a list. Both let/assignment destructuring now use the native axes;
  execution compares all four identity limbs with the real nox digest.
- Noun could reach numeric loop/index positions because legacy type checking
  computed but ignored those types. All three boundaries now require Field/U32.
- Strict artifact transport exposed noncanonical Field literal/constant output.
  AST lowering now reduces Field values according to the documented modulus;
  p and u64::MAX are executed regression cases. U32 and size checks remain.
- TIR unit-returning local functions clear inherited builtin return metadata;
  scalar and unit functions may shadow native intrinsic names normally.

Read-only independent review covered type/layout boundaries, all expression
positions, active cfg, generic bodies, lexical intrinsic aliases and raw
lowering. Main review also checked source hash tags, emission bounds, canonical
encoding, worker joining and scalar-verifier coverage whitelists.

## Validation

- Trident package: 836 passed, including 17 new native-source cases. Silicon:
  34 passed. Zero failures or ignored cases. Final diagnostic cleanup was
  followed by the 683 library tests and all 17 native cases again.
- Joy workspace: 88 passed, zero failed/ignored, including three source→ART1→VM
  CLI tests and existing public/state/private proof tests.
- Trisha-rs: 362 passed, zero failures, four pre-existing expensive proof gates
  ignored. These are explicitly not claimed as rerun. Its benchmark executes
  133/133 fixtures with all 43/43 independent baselines verified.
- Workspace/all-target checks for Trident and Joy pass without warnings.
- `trident audit lib/vm/nox/noun.tri --json` returns UNKNOWN/exit2: native
  intrinsic declarations have no analyzable scalar bodies. Source contracts are
  specified and runtime-tested; this is not a formal Noun verification claim.

Actual source CLI observations, compared byte-for-byte with independently
hand-built native fixture outputs:

| Program | Output | Charged reductions | Allocated nodes | Peak frames |
|---|---|---:|---:|---:|
| input0 +7+7 | atom14 | 12 | 29 | 5 |
| identity | complete [[1 2]3] | 6 | 21 | 3 |
| Field literal p | atom0 | 6 | 18 | 3 |

The [validation manifest](sh1-noun-validation.json) pins commands, logs and
source hashes. Joy keeps full CLI receipts in
`joy/audit/raw-source-observations-2026-09-23.json`.

## Still open

Calls inline and loops unroll in this delivery. Seq/Bytes libraries, reusable
calls/loops with checked dynamic indexing, production JOB1/RES1 admission and
all SH2+ self-compilation/proof gates remain open. The next implementation is
[reusable native control flow](native-control-design.md). Physical compiler-scale
memory remains SH4 work; the seed/raw arena has 196608 usable nodes.
