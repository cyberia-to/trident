# Native compiler data — SH0.2

This is the version 1 data contract for [soft3 self-hosting](self-hosting.md).
It specifies the 0.4 implementation target. The 0.4 development compiler now
accepts the `Noun` primitive, seven `vm.nox.noun` operations and the source
collection APIs below. The stable release does not include this extension;
SH0.3 specifies their job/artifact transport and SH0.4 their runtime budgets.
The original [model receipt](../audit/self-hosting/native-data.md) and the
[source execution receipt](../audit/self-hosting/native-collections.md) distinguish
reference-model tests from source libraries executed through Joy on nox.

## Native values and source types

A `Noun` is an immutable nox value: a canonical Goldilocks atom in
`0..18446744069414584321`, or an ordered pair of nouns. It contains its reachable
data, conceptually; copying a value may share that data. A host arena index is
never a source value, serialized identity, Field cast or portable pointer.
Inputs with noncanonical field words must be rejected, not reduced modulo p.

`Noun` is a new primitive type, initially available only on the `nox` target.
It may appear in parameters, results, local bindings, tuples, fixed arrays and
struct fields. A source constant cannot be a Noun in the initial subset.
No implicit conversions, arithmetic, ordering, indexing syntax or `==` are
defined for it. Use the explicit operations below. Existing fixed-array syntax
keeps its meaning; dynamic sequences use library functions.

Nox stores a Noun parameter/result as one **subtree**, whose field-word width is
variable. The compiler must distinguish native subject slots from scalar/stack
widths throughout type checking, intrinsic ABI, entry adapters and lowering.
Assigning `width(Noun) = 1 field` is invalid. A containing aggregate also lacks
a fixed field-word width. Foreign backends reject these types explicitly until
they implement a separately specified representation. Existing scalar entry
formulas and the flat-word Joy entry ABI retain their versioned contract.
SH0.3 must define a separately identified structured entry profile; a Noun entry
must never silently enter the current flat-word adapter.

## Minimal intrinsic API

Module: `vm.nox.noun`. The 0.4 implementation includes these declarations,
AST/typechecker, direct lowering and target-capability support together.
All operations are deterministic and pure with respect to program state.
Allocation or reduction exhaustion can still fail an execution.

| Function | Meaning | Existing nox mechanism |
|---|---|---|
| `atom(value: Field) -> Noun` | Explicit embedding of a canonical Field | Same runtime atom; retain source type distinction |
| `pair(left: Noun, right: Noun) -> Noun` | Ordered immutable pair | Pattern 3, cons |
| `head(value: Noun) -> Noun` | Left child; fail on atom | Compose with quoted `[0 2]` |
| `tail(value: Noun) -> Noun` | Right child; fail on atom | Compose with quoted `[0 3]` |
| `as_field(value: Noun) -> Field` | Checked atom projection; fail on pair | Pattern 5, add zero; **not** an unchecked type cast |
| `eq(a: Noun, b: Noun) -> Bool` | Full particle identity equality | Pattern 9; 0=true, 1=false |
| `identity(value: Noun) -> Digest` | Four-limb native tree identity | Compose with quoted `[0 0]` |

There is no guest `is_atom`/`is_pair` primitive in current nox. Source code uses
known typed layouts and explicit tags; malformed shape traps at checked
projection. Host-side node-kind inspection is not evidence that a guest program
can perform a recoverable shape test. The initial API does not promise one.
Ordinary evaluation uses deterministic pattern 2 composition. Pattern 16
witness calls cannot implement collection operations or compiler stages.

## Identity, ordering and failure

Nox identity is the full four-limb Hemera tree digest. Atom/pair domains and
left/right order are significant. Nox hash-consing and `eq` use that digest;
semantic equality relies on its collision resistance. There is no structural
collision fallback. Equality of just one digest limb is insufficient. Neither
arena allocation order nor a Rust `Order` participates in the data contract.

`Seq` and `Bytes` use different domain tags and include their exact length, so
an empty byte string, one zero byte and a sequence containing zero have distinct
identities. Logical values have a single tree shape, independent of construction
history. Iteration and search follow increasing indices; compiler symbol lookup
must compare complete names, with stable insertion order and no host hash map.

Invalid tags/shapes/word ranges/padding, an out-of-bounds index, capacity overflow
or budget exhaustion fail explicitly. Operations return no successful partial
value; old roots remain valid. Allocations made before failure may remain charged
to the job's append-only arena. No rollback or reclamation is implied. Collection
precondition failures are fatal execution errors in the minimal API. The compiler
checks expected source errors before calling them and emits diagnostics through
SH0.3's result protocol. Malformed external data may instead fail the job at its
validation boundary; catching arbitrary VM traps inside the guest is not required.

## Canonical indexed tree

`Seq` is a monomorphic, typed wrapper over a sequence of Noun values. It avoids
adding type generics, references or mutable RAM to the compiler subset.

```text
SEQ1 = 1397051697 = 0x53455131
Seq = [SEQ1 [length tree]]
E(0) = 0
E(h+1) = [E(h) E(h)]
height(n) = 0 if n <= 1, otherwise ceil(log2(n))
```

The tree has `2^height(length)` leaf positions, ordered left to right. The first
`length` leaves contain values; every unused leaf is atom zero. An occupied leaf
may itself be a pair: height, not a kind test, determines when traversal stops.
An empty sequence has tree `0`. Height is derived, never serialized. Each subtree
covering only unused positions must equal `E(its height)`; compressed or expanded
alternative padding is invalid. Hash-consing may share these identical subtrees.

Length and indices are U32. The format ceiling is `2^32 - 1` elements; height is
at most 32. Capacity calculations use Field or a checked wider host integer:
`2^32` is a valid tree capacity, but cannot be held in U32. A per-job sequence
limit is required before allocation/validation and may be much smaller.

Lookup follows at most 32 left/right steps. `set` replaces only nodes along that
path, preserving all siblings. `push` writes the first padding leaf; when full,
it first grows the root to `[old_tree E(old_height)]`. Thus direct construction,
append and updates produce identical canonical trees for identical contents.
The source implementation uses bounded loops/explicit path state, not application
recursion. Library internal fixed-depth work stacks may use existing fixed arrays;
their dynamic access needs SH1. No dynamic-axis opcode is assumed: repeatedly
project head/tail, or build and evaluate a bounded axis formula using composition.

Validation checks the wrapper, length limit, required branch shape and padding.
It must have an explicit visit budget, charged even on shared/repeated children.
One visit is entry into a tree node during traversal, including an occupied leaf
or a whole unused subtree comparison. Repeated path reads in a separate payload
validation pass also consume visits (height+1 per read). Fixed wrapper checks and
constructing the at-most-32-level empty-tree identities consume the runtime
reduction/node budget; they do not consume tree visits. A zero visit allowance
cannot validate even an empty tree. Exhaustion fails before the next visit.
Compare a wholly unused subtree to `E(h)` without expanding `2^h` zero leaves.
Occupied leaves are arbitrary Nouns and need no recursive value validation once
the transported DAG has been validated. SH0.3 handles complete DAG validation.
A terminal branch may charge and validate its two leaf positions together,
retaining one visit for each position and checking any unused padding. Occupied
pair-shaped leaves remain opaque values. This avoids allocating traversal tasks
for leaf positions while preserving the exact successful visit allowance.

## Exact byte strings

```text
BYT1 = 1113150513 = 0x42595431
Bytes = [BYT1 [byte_length word_tree]]
word_count = byte_length / 4 + (byte_length % 4 != 0)
word[j] = b[4*j] + 256*b[4*j+1] + 65536*b[4*j+2] + 16777216*b[4*j+3]
```

Missing bytes in the final word are zero. The word tree uses the indexed tree
above with `word_count` occupied leaves; each is an atom in `0..2^32`. Words are
little-endian, and no word can cross the Goldilocks modulus. Unused high bytes
of the final word must be zero. The outer byte length is authoritative; an
embedded Seq wrapper or an extra word-count field is not present.

Byte strings preserve exact bytes, including zero, CR/LF and invalid UTF-8.
Unicode/lexical validation is a compiler responsibility; encoding does not
normalize newlines, paths or text. Diagnostic positions count bytes. Maximum
byte length is `2^32 - 1`, implying at most `2^30` words / 30 tree levels. Compute
ceiling division without overflowing `byte_length + 3` in U32.

No new divmod opcode is required for byte extraction. For a validated U32 word
`w`, byte position `r` in `0..4`, let `m=255*256^r`. Then:

```text
masked = w AND m
byte = as_u32(as_field(masked) * inverse_Field(256^r))
```

The numerator is an exact nonnegative multiple of `256^r`; byte is below 256. Inverse
constants can be quoted. `i/4` similarly uses `(i - (i AND 3)) * inverse_Field(4)`;
no unbounded subtraction loop is needed. Source `sub` and explicit checked
conversions implement the mathematical notation above. The VM word operators
reject inputs above U32 rather than silently truncating them.

## Library surface and compiler migration

Modules `std.nox.seq` and `std.nox.bytes` own private validated wrappers
`Seq` and `Bytes`. Their implementation is Trident library code over the intrinsic
API in `lib/std/nox/`. An unchecked wrapper constructor
is private. All externally received wrappers pass a bounded validator.

The source type checker retains the defining module in struct identity and
enforces private fields at construction, projection, mutation and patterns.
An identical local struct cannot stand in for an imported wrapper. Module names
must be unique in the parsed compilation closure before generic specialization;
aliases and forwarded return values preserve their original owner. This protects
source-level native handles. External raw entry data still requires `from_noun`.

`std.nox.tree` owns the shared raw-tree operations: geometry, empty padding,
path reads/updates, append and bounded shape validation. Its public helpers
consume and produce raw Nouns; they cannot construct a validated Seq/Bytes
handle. Seq and Bytes retain separate private constructors and field owners.
Raw `get`, `set` and `push` require an existing canonical tree for their supplied
length. Untrusted roots must pass `validate` before those operations.
Shape validation returns the remaining visit allowance so Bytes can charge its
word scan against the same budget. The traversal uses a Noun work stack and
bounded helper chunks: returning from each chunk releases its evaluator frames.
The outer driver returns explicitly on completion or fails on exhaustion.
Outer driver frames still grow with the number of chunks; this is not a
constant-memory promise or evidence that a complete compiler workload fits.

| Operation | Seq signature | Bytes signature |
|---|---|---|
| Empty | `empty() -> Seq` | `empty() -> Bytes` |
| Length | `len(s: Seq) -> U32` | `len(b: Bytes) -> U32` |
| Read | `get(s: Seq, i: U32) -> Noun` | `get(b: Bytes, i: U32) -> U32` (0..255) |
| Persistent write | `set(s: Seq, i: U32, v: Noun) -> Seq` | `set(b: Bytes, i: U32, v: U32) -> Bytes` (v<256) |
| Append | `push(s: Seq, v: Noun, max_len: U32) -> Seq` | `push(b: Bytes, v: U32, max_len: U32) -> Bytes` |
| Encode | `to_noun(s: Seq) -> Noun` | `to_noun(b: Bytes) -> Noun` |
| Validate/decode | `from_noun(n: Noun, max_len: U32, max_visits: U32) -> Seq` | `from_noun(n: Noun, max_len: U32, max_visits: U32) -> Bytes` |
| Shared allowance | `from_noun_budget(n: Noun, max_len: U32, remaining: U32) -> (Seq, U32)` | `from_noun_budget(n: Noun, max_len: U32, remaining: U32) -> (Bytes, U32)` |

`std.nox.bytes.BytesTable` retains already validated byte handles across compiler
stages. It is an opaque, persistent, append-only table: `table_empty()` constructs
it, `table_len(table)` returns its length, `table_push(table, value: Bytes,
max_len: U32)` appends within an explicit cap, and `table_get(table, index: U32)`
returns the stored `Bytes` after checking the index. Old table and byte values
remain unchanged by later appends or byte updates. Each entry stores the private
length/root representation and a derived packed-word count inside the owning
library. The private count equals ceil(length / 4), is derived by trusted
constructors and preserved by set/table snapshots. External decoding initializes
it only after validation; push updates it
only when crossing a packed-word boundary. BYT1 serialization and admission
visit charges stay unchanged. Callers cannot construct,
project or mutate its backing sequence. There is no raw-Noun table constructor or
serialization API. External bytes still cross `from_noun_budget` exactly once;
table reads spend runtime reductions/nodes without repeating admission work or
granting a fresh validation allowance. The module graph keeps these typed tables
in its state and uses bounded indices in its traversal records.

Lengths must fit caller/job caps; push checks before incrementing, never wraps.
Validators consume a visit allowance and trap on exhaustion. Exact global
reduction/node limits belong to SH0.4 in addition to these algorithmic bounds.
The budget variants return the unspent allowance, including Bytes word-read
charges. A guest admission pass threads this value through successive wrappers;
it never resets the allowance per field. The convenience `from_noun` wrappers
delegate to the budget variants and discard the remainder. Fixed record checks
belong to the caller's record-validation accounting.
Byte pushes/sets use the affected packed word and one path update. A bulk builder
may reduce allocation history, but must produce the identical canonical tree.

Compiler source buffers become Bytes. Token/AST/type/symbol/work tables become
Seq wrappers with record-specific constructors and validators. IDs are checked
indices into a declared table, never RAM addresses. Compiler records use explicit
versioned tags where variants need distinguishing; their detailed schemas belong
to the compiler port. State updates return new roots; callers explicitly rebind
their state. Source spans retain exact byte offsets. No type-generic map, mutable
heap, OS service or network registry is needed for this initial representation.

The [inventory disposition](../audit/self-hosting/compiler-subset.md#native-data-additions-sh02)
tracks new types, structs, operations, dynamic indexing and runtime-loop needs.
It does not insert proposed library files into the observed source inventory.

## Conformance and owner boundaries

Golden tree/identity vectors and a Rust reference model are in
`examples/selfhost_data*` and
[native-data-vectors.json](../audit/self-hosting/native-data-vectors.json).
Tests must independently check exact byte values/tree shapes, alternative
construction histories, persistent writes, malformed padding and bounds. Small
formula probes execute the checked projections, identity/equality and byte
extraction on the actual nox evaluator. The model is not a guest compiler/library.

Trident owns source types/libraries and reference lowering; nox owns noun and
pattern semantics; Joy must preserve complete tree topology across structured
job/result transport. Zheng must bind all four identity limbs, shapes, checked
projections, dynamic traversal and runtime control flow in the selected profile.
Current run support does not establish that its production relation proves this
workload. Native proof acceptance remains SH7/SH8.

Per-operation logarithmic tree work does not bound total historical allocations:
the current arena does not free old roots. Packed source buffers reduce input
leaves, while ASTs, updates, formulas and traces still consume budget. SH4 must
measure the complete compiler workload; this contract does not certify it fits
Joy's current fixed arena or nox's recursion limit.
