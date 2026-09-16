# Independent Nox typed-entry ABI review

Date: 2026-09-12. Scope: `src/ir/tree/lower/nox/entry.rs`, its call from
`NoxCompiler::compile_fn`, module/type qualification and aggregate navigation,
`reference/nox.md`, compiler API 3 in `reference/warrior-api.md`,
`tests/nox_surface/entry.rs`, and Joy `rs/tests/typed_entry.rs`.

Status: both identified entry findings are corrected and independently retested.
The reviewer implemented the owner-authorized lexical-size correction after the
read-only review; the owner implemented chunked paths. Reviewed files are stable.
Frozen FINAL3 was not changed.

## Contract and sound portions

The formula consumes exactly the declared flat public-word sequence, in source
parameter/field/index order. It then reconstructs the internal reverse parameter
stack, with declaration-order cons lists for structs/tuples/arrays and a balanced
four-limb Digest tree. Ordinary calls do not invoke this adapter. The public
arity check is sequenced before reconstruction and the body; it checks the exact
zero terminator rather than merely checking that enough inputs exist.

Each Field leaf is used by addition with zero, requiring an atom even when the
source body ignores that parameter. Bool and U32 use native less-than guards
against 2 and 2^32. The Bool convention remains Nox's 0=true/1=false. Only the
selected guard branch executes the `inv(0)` crash; its scalar result does not
force the body result to have scalar shape. Rebuilding the complete subject
executes every narrow-type check before any source effect.

Stateful entries select external words below the BBG root and restore the
original root above reconstructed parameters. The root is not counted as a
public word or interpreted as an aggregate parameter. Existing state surface
coverage uses the actual look provider; this review did not generate a new
state proof. Arity currently allows 58 words without state, 57 with a root.

Input navigation is safe under these limits: generated axes remain below the
field modulus and word arithmetic cannot overflow. Layout recursion checks
16 levels and 4096 nodes; each list is capped at 58 elements. Checked addition
and multiplication protect symbolic array-size arithmetic. These constraints do
not, however, bound every *internal* aggregate path (finding 2).

The guards live inside the exported formula, not solely in the host CLI. Thus
public and private execution relations bind them with the complete formula,
input, output and native cost. Inspected Joy tests independently mutate each
public aggregate leaf, output, unused narrow inputs and arity. The private test
also exercises divine input. This review does not claim to have executed those
proof tests; the owner runs their receipts separately. Native adapter work
contributes real reductions, including checks for unused inputs; it cannot be
priced as the old body-only entry.

## Finding 1: imported symbolic array lengths lose lexical ownership

A accepted imported type fails to compile through the adapter:

```trident
// lib.tri
module lib
const N: U32 = 2
pub struct Pair { words: [Field; N] }
```

```trident
// import.tri
program entry
use lib
fn main(x: lib.Pair) -> Field { x.words[0]*100+x.words[1] }
```

The fresh release compiler rejects it with `nox: unresolved entry array length`.
`modules.rs::qualified_type` qualifies Named types but clones ArraySize; later
`entry_size` resolves N through the entry module instead of lib. A same-named
entry constant could select a different shape. The unused-import warning is a
separate diagnostic and is not the fatal cause.

Required correction: resolve or qualify all array-size expression leaves in the
lexical defining module, recursively through nested imported struct fields and
function signatures. Preserve checked arithmetic. Test a foreign N, a conflicting
entry N, and a nested imported array; assert actual source-order execution.

## Finding 2: bounded flat entry can overflow a fused internal axis

Construct Inner with 40 zero-width `[Field;0]` fields followed by `value: Field`;
construct Outer with 40 such fields followed by `inner: Inner`. Then:

```trident
fn main(x: Outer) -> Field { x.inner.value }
```

This is one public word, two shallow structures and bounded lists. Compilation
succeeds, but actual native execution on input 7 rejects `axis out of range`,
instead of returning 7. Concatenating both field paths exceeds the machine axis
width. `dotted_axis`, `place_axis`, and `elem_access` all use unchecked
`axis_compose`; bounding public-word count alone does not make those safe.

Required correction: preserve path segments and use sequential navigation when
fusion is not representable. Apply the same principle to recursive assignment
reconstruction, not only reads. A fused address must be a canonical field atom,
not merely fit u64. Rejecting every such valid bounded aggregate would narrow
the accepted language rather than complete its implementation. Regressions must
include reading and writing a deep last field, an unaffected sibling, and the
boundary around the largest canonical fused axis.

## Independently executed evidence

`cargo build --release --locked --bin trident` completed successfully on the live
workspace (16.28 s). Both counterexamples were then compiled with that fresh
release compiler. The second emitted formula was executed as a raw `.nox` file
by the actual Joy native runner, avoiding any stale source compiler in Joy.

Exact temporary sources and machine-readable results:
`/tmp/nox-entry-extra-review/{lib.tri,import.tri,deep.tri,receipt.json}`.
Finding 1: compile exit 1. Finding 2: compile exit 0, native run exit 1.
The older FINAL3 observations at
`/tmp/nox-entry-abi-review-20260912/receipt.json` document the original entry bug;
they are not evidence that the new scalar checks fail.

## Authorized correction checkpoint

Following the independent findings, the owner assigned the reviewer the narrow
lexical-size correction in `modules.rs` and the new
`tests/nox_surface/imported_entry.rs`; the owner retains path lowering.
The correction resolves literal module constants before qualifying declarations,
recursively qualifies unresolved size leaves, folds size addition/multiplication
only with checked arithmetic, and preserves explicit function generic names.
The unit test `lexical_sizes_preserve_generics_and_never_wrap_arithmetic` passed
(`/tmp/nox-lexical-size-unit.log`). The integration regression also covers nested
arrays of imported structures with a conflicting entry-module N and an ordinary
local generic call. It exposed missing expression-type recovery through dotted
variables and indexing; that adjacent path correction is with the owner.

A first exploratory imported generic-function call was rejected by the earlier
typechecker (`not generic`), before Nox lowering. It is not claimed fixed by the
lexical-size patch; the preserved-generic regression instead uses the existing
local generic call path. This is a separate imported generics surface limitation.


## Final independent retest and scope closure

Both original sources were recompiled with the freshly rebuilt release compiler,
then executed by the actual native runner:

- Imported Pair, input `[7,19]`: compile/run exit 0, output 719, 32 reductions.
- Deep zero-width-prefix structures, input `[7]`: compile/run exit 0, output 7,
  187 reductions.

Exact post-fix receipt: `/tmp/nox-entry-extra-review/fixed-receipt.json`; the
original failing receipt is retained separately. No failing source was weakened.

The persistent integration regression
`imported_entry_sizes_keep_lexical_constants_and_generic_calls` passed in both
source profiles (`/tmp/nox-imported-entry-fix.log`). It uses a conflicting local
N, a foreign N+1 nested array, checks all six imported scalar leaves, and verifies
a local generic call. The owner-fixed dotted/index expression type recovery is
therefore exercised as part of real execution.

The persistent `deep_aggregate_reads_and_writes_keep_full_paths_and_siblings`
regression independently passed (`/tmp/nox-deep-path-review.log`), exercising
reads, two writes and unaffected siblings in both source profiles. I inspected
`path.rs`: paths are represented without truncation, navigation is split into
canonical 62-edge chunks, reconstruction evaluates the RHS exactly once against
the original subject, and path/emitted-node bounds are explicit. The adapter's
original direct external axes remain safe under its flat-word bound.

No further blocker was identified in this reviewed adapter scope. This is not a
claim that the separately documented general variable-shape Nox proof relation
or imported generic-function typechecking has been implemented. Broad workspace
and genuine proof receipts remain the owner's separately executed release gates.

## Subsequent imported-generics correction

The imported generic-function limitation recorded above was a real finding at
this review's checkpoint. It is now superseded by the separate concrete AST
specialization and body-checking work, independently reviewed in
[Trisha's imported-generics audit](../../trisha/audit/imported-generics-review.md).
That gate has five passing real VM/type-rejection tests (both source profiles)
and a separately executed genuine default-security proof binding program,
public input and public output. The original failure description remains as
historical evidence; it is no longer the current imported-generics status.
