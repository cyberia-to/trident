# 🗡️ Trident Language Reference

[IR Reference](ir.md) | [Target Reference](targets.md) | [Grammar](grammar.md) | [Error Catalog](errors.md) | [Agent Briefing](briefing.md)

Trident is a programming language for provable computation. The implemented
warriors are Joy for nox/Zheng and Trisha for Triton/Neptune. The shared frontend
and target packages provide the boundary for additional machines; catalog
entries alone do not supply an executable backend.

Version 0.5 | File extension: `.tri` | Compiler: `trident`

---

# Part I — Universal Language (Tier 0 + Tier 1)

Part I defines the shared language. A selected target package determines the
available operations, field/digest ABI and runtime/proof capabilities. Programs
must meet that target's supported surface; unsupported operations fail during
compilation. For example, nox currently rejects `reveal` and `seal`, while Trisha
implements their Triton event ABI. See the [target reference](targets.md) and
[warrior contract](warrior-api.md) for discovery and compatibility checks.

---

## 1. Programs and Modules

Every `.tri` file starts with exactly one of:

```trident
program my_program      // Executable — must have fn main()
module my_module        // Library — no fn main, provides reusable items
```

### Imports

```trident
use merkle                      // import module
use crypto.sponge               // nested module (directory-based)
```

Rules:
- `use` imports a module by name, accessed via dot notation (`merkle.verify(...)`)
- No wildcard imports (`use merkle.*` is forbidden)
- No renaming (`use merkle as m` is forbidden)
- No re-exports — if A uses B, C cannot access B through A
- No circular dependencies — the dependency graph must be a DAG

### Visibility

Two levels only:
- `pub` — visible to any module that imports this one
- default — private to this module

No `pub(crate)`, no `friend`, no `internal`.

```trident
module wallet

pub struct Balance {
    pub owner: Digest,      // visible to importers
    amount: Field,          // private to this module
}

pub fn create(owner: Digest, amount: Field) -> Balance {
    Balance { owner, amount }
}
```

A structure's identity includes its defining module. Identical names and field
layouts in different modules denote different types. Import aliases and function
return values preserve that identity, including size-generic functions.

Only the defining module may initialize, read, assign, or explicitly name a
private field in a match pattern. Importers may construct a structure only when
all its fields are public; otherwise they obtain values through its module's
functions. Passing and storing an opaque value does not expose its fields.

### Project Layout

```text
my_project/
├── main.tri            // program entry point
├── merkle.tri          // module merkle
├── crypto/
│   └── sponge.tri      // module crypto.sponge
└── trident.toml        // project manifest
```

#### trident.toml

```toml
[project]
name = "my_project"
version = "0.1.0"
entry = "main.tri"
```

---

## 2. Types

### Primitive Types

| Type | Width | Description |
|------|------:|-------------|
| `Field` | 1 | Native field element of the target VM |
| `Bool` | 1 | Field constrained to {0, 1} |
| `U32` | 1 | Unsigned 32-bit integer, range-checked |
| `Digest` | D | Hash digest `[Field; D]` — universal content identifier |

`Field` means "element of the target VM's native field." Programs reason about
field arithmetic abstractly; the target implements it concretely.
Integer literals used as Field values represent their residue modulo the target
field's modulus. Emitted machine constants use the canonical representative.
This rule does not wrap U32 literals or compile-time array dimensions: their
respective integer range and overflow checks still apply.

`Digest` is universal — every target has a hash function and produces digests.
It is a content identifier: the fixed-width fingerprint of arbitrary data. The
width D varies by target (5 on TRITON, 4 on MIDEN, 8 on SP1/OPENVM, 1 on CAIRO).

No implicit conversions. `Field` and `U32` do not auto-convert. Use `as_field()`
and `as_u32()` (the latter inserts a range check).

For extension field types, see [Extension Field](#16-extension-field).

### Native data extension (0.4 development)

[Native compiler data](self-hosting-data.md) specifies the new `Noun` primitive,
checked `vm.nox.noun` operations and persistent `std.nox.seq` / `std.nox.bytes`
wrappers. The 0.4 development compiler implements `Noun` and the seven native
operations together with parsing, type checking and direct nox lowering.
Reusable raw calls/loops and the collection wrappers are implemented in 0.4
development. Production Joy compiler-job admission remains SH1 work.

`vm.nox.noun` exports `atom(Field) -> Noun`, `pair(Noun, Noun) -> Noun`,
`head(Noun) -> Noun`, `tail(Noun) -> Noun`, `as_field(Noun) -> Field`,
`eq(Noun, Noun) -> Bool`, and `identity(Noun) -> Digest`. Their intrinsic names
are `nox_noun_` plus the function name. Projections check shape; equality uses
the full native identity. These pure operations evaluate operands once, left
to right. `as_field(U32)` remains the separate ordinary widening conversion.

`fn main(input: Noun) -> Noun` uses the explicit ART1 raw input/output
profiles 0/0 through `compile_raw_artifact_project` or `joy build --emit artifact`.
This entry accepts the complete input noun and returns the complete result.
Flat I/O declarations and host services are forbidden in the raw profile.
The flat bundle entry adapter rejects Noun-containing parameters and results.
Noun is allowed in internal native aggregates, with explicit checked operations;
constants, event payloads, flat RAM/I/O declarations and `==` reject it.

A Noun carries an immutable native subtree, with variable field-word width.
Its containing aggregates also require a native layout. The fixed-width tables
below describe the existing source types; they must not be extended by treating
Noun as a scalar field word. The initial capability is nox-only, with explicit
conversions and separately versioned structured entry transport. Other targets
must reject it until their own representation is specified and implemented.

### Composite Types

| Type | Width | Description |
|------|-------|-------------|
| `[T; N]` | N * width(T) | Fixed-size array, N compile-time known |
| `(T1, T2, ...)` | sum of widths | Tuple (max 16 elements) |
| `struct S { ... }` | sum of field widths | Named product type |

Array sizes support compile-time expressions: `[Field; N]`, `[Field; M+N]`,
`[Field; N*2]`.

No enums. No sum types. No references. No pointers. All values are passed by
value. Fixed-layout stack backends flatten structs to stack/RAM elements;
native nox keeps each field as a subtree.

### Type Widths

Fixed-layout types have a compile-time-known width measured in field elements.
`Noun` and every containing aggregate have no fixed field-word width; the
semantic API returns `None` for these layouts. Shared TIR rejects them before
computing widths, including inside deferred generic bodies.
Widths marked with a variable are resolved from the target configuration.

| Type | Width |
|------|-------|
| `Field` | 1 |
| `Bool` | 1 |
| `U32` | 1 |
| `Digest` | D (`digest_width` from target config) |
| `[T; N]` | N * width(T) |
| `(T1, T2)` | width(T1) + width(T2) |
| `struct` | sum of field widths |

---

## 3. Declarations

### Functions

```trident
fn private_fn(x: Field) -> Field { x + 1 }
pub fn public_fn(x: Field) -> Field { x + 1 }
```

- No default arguments, no variadic arguments
- No function overloading, no closures
- No recursion — call graph must be a DAG
- Maximum 16 parameters (stack depth)
- Tail expression is the return value

### Size-Generic Functions

```trident
fn sum<N>(arr: [Field; N]) -> Field {
    let mut total: Field = 0
    for i in 0..N { total = total + arr[i] }
    total
}

fn concat<M, N>(a: [Field; M], b: [Field; N]) -> [Field; M+N] { ... }
```

Size parameters appear in angle brackets. Each unique combination of size arguments
produces a monomorphized copy at compile time.

```trident
let a: [Field; 3] = [1, 2, 3]
let total: Field = sum(a)       // N=3 inferred from argument type
let total: Field = sum<3>(a)    // N=3 explicit
```

Only integer size parameters — no type-level generics.

Public size-generic functions retain their size parameters across imports.
Explicit sizes and sizes inferred from argument shapes have the same contract
as local calls. Each instance is checked in its defining module, preserving
private helpers, constants, types and conditional compilation. Instantiation
errors (including invalid body operations) are compilation errors, not unchecked
backend calls. Specializations receive collision-free internal names. Checked
size arithmetic rejects unresolved names and overflow. A project is limited to
1,024 concrete instances and 128 dependency-expansion rounds; exceeding a
limit is a compilation error.
Every function checks explicit returns and its reachable terminal expression
against the declared return type; an omitted return type is unit. A value-returning
function must return on every reachable path. A terminal if/else or match
forwards its branch terminal values; intermediate branch values are discarded.
An empty loop cannot establish
that obligation.


### Structs

```trident
struct Point { x: Field, y: Field }
pub struct PubPoint { pub x: Field, pub y: Field }

let p = Point { x: 1, y: 2 }
let x: Field = p.x
```

Struct literals may list fields in any order. Every declared field must appear
exactly once. Field expressions are evaluated exactly once in **struct declaration
order**, independently of their textual order in the literal. The flattened
value uses that same declaration order, recursively for nested structs; arrays
and tuples retain element order. This rule also determines the order of I/O and
other effects in field expressions.

### Events

```trident
event Transfer { from: Field, to: Field, amount: Field }
```

Events are declared at module scope. Payloads contain at most nine flattened
field words; scalars, arrays, tuples, digests and structs use their resolved
target widths. Events are emitted with `reveal` (public) or `seal`
(committed) — see [Part II: Events](#10-events).

### Constants

Constant references preserve their declared type, including across imports.
A U32 integer constant must be in the range 0 through 4,294,967,295.
Field literal values are reduced modulo the selected field prime when executed;
compile-time array dimensions remain checked integers and are not field-reduced.

```trident
const MAX_DEPTH: U32 = 32
pub const ZERO: Field = 0
```

Inlined at compile time. No runtime cost.

### I/O Declarations (program modules only)

```trident
pub input:  [Field; 3]      // public input (visible to verifier)
pub output: Field            // public output
sec input:  [Field; 5]      // secret input (prover only)
sec ram: { 17: Field, 42: Field }   // pre-initialized RAM slots
```

---

## 4. Expressions and Operators

### Operator Table

| Operator | Operand types | Result type | Description |
|----------|---------------|-------------|-------------|
| `a + b` | Field, Field | Field | Field addition |
| `a + N` | Field, literal | Field | Immediate addition |
| `a * b` | Field, Field | Field | Field multiplication |
| `a == b` | Field, Field | Bool | Field equality |
| `a < b` | U32, U32 | Bool | Unsigned less-than |
| `a & b` | U32, U32 | U32 | Bitwise AND |
| `a ^ b` | U32, U32 | U32 | Bitwise XOR |
| `a /% b` | U32, U32 | (U32, U32) | Division + remainder |

No subtraction operator (`-`). No division operator (`/`). No `!=`, `>`, `<=`,
`>=`. No `&&`, `||`, `!`. Use builtins: `sub(a, b)`, `neg(a)`, `inv(a)`.

For extension field operators, see [Extension Field](#16-extension-field).

### Other Expressions

```trident
p.x                             // field access
arr[i]                          // array indexing
Point { x: 1, y: 2 }           // struct initialization
[1, 2, 3]                       // array literal
(a, b)                          // tuple literal
{ let x: Field = 1; x + 1 }    // block with tail expression
```

---

## 5. Statements

### Let Bindings

```trident
let x: Field = 42                          // immutable
let mut counter: U32 = 0                   // mutable
let (hi, lo): (U32, U32) = split(x)       // tuple destructuring
```

### Assignment

```trident
counter = counter + 1
p.x = 42
arr[i] = value
(a, b) = some_function()                   // tuple assignment
```

Tuple assignment evaluates its right-hand side once. Every target must already
exist, be mutable, and have the corresponding component's type. Its arity must
match the tuple (or the target's Digest/XField width). `_` is a discard binding
in a `let` pattern; it is not a tuple-assignment target.

### If / Else

```trident
if condition {
    // body
} else {
    // body
}
```

No `else if` — use nested `if/else`. Condition must be `Bool` or `Field`
(0 = false, nonzero = true).

### For Loops

```trident
for i in 0..32 { body }               // constant bound — exactly 32 iterations
for i in 0..n bounded 64 { body }     // runtime bound — at most 64 iterations
```

All loops require a constant end or an explicit `bounded` annotation. Local
variables shadow same-named global constants; an immutable local end still
requires a bound. Native loop-index specialization follows the
[native runtime contract](self-hosting-runtime.md#native-bounded-loops).

No `while`. No `loop`. No `break`. No `continue`.

### Match

```trident
match value {
    0 => { handle_zero() }
    1 => { handle_one() }
    _ => { handle_default() }
}
```

Patterns: integer literals, `true`, `false`, struct destructuring, `_` (wildcard).
Exhaustiveness is enforced — wildcard `_` arm is required unless all values are covered.

```trident
// Struct pattern matching
match p {
    Point { x: 0, y } => { handle_origin_x(y) }
    Point { x, y: 0 } => { handle_origin_y(x) }
    _ => { handle_general(p.x, p.y) }
}
```

### Return

A return exits the current function, including from nested conditionals and
fixed or explicitly bounded loops. Later iterations and statements have no
effects. On nox these loops are unrolled with return-aware continuations; the
existing formula/iteration limits still apply, and unbounded dynamic loops
remain rejected. Returning from an inlined helper exits that helper only.

```trident
fn foo(x: Field) -> Field {
    if x == 0 { return 1 }
    x + x                      // tail expression — implicit return
}
```

---

## 6. Builtin Functions

### I/O and Non-Deterministic Input

Signatures are available only when the resolved target package declares the intrinsic. Triton supports streaming public/secret I/O. Nox programs receive a subject and return a result; they do not support Triton streaming I/O. Joy's stateless public certificates reject secret and state witnesses; distinct private and authenticated-state protocols support the declared bounded cases.

| Signature | Description |
|-----------|-------------|
| `pub_read() -> Field` | Read 1 public input |
| `pub_read2()` ... `pub_read5()` | Triton: read exactly N public inputs; width five returns Digest |
| `pub_write(v: Field)` | Write 1 public output |
| `pub_write2(...)` ... `pub_write5(...)` | Write N public outputs |
| `divine() -> Field` | Read 1 secret input (prover only) |
| `divine3() -> (Field, Field, Field)` | Read 3 secret inputs |
| `divine5() -> Digest` | Triton: read exactly five secret inputs as Digest |

The generated `vm.io.io` module exposes tuple reads for widths two through D−1 (`read2`, `read3`, and `read4` on Triton), plus `read_digest() -> Digest` and `divine_digest() -> Digest` using the selected digest width. Calls must be supported by the selected target capabilities. The old `io.read5` and `io.divine5` aliases are absent. `std.target` is generated from the same resolved ABI: nox has digest width 4, hash rate 8, stack depth 0; Triton has 5, 10, and 16.

### Field Arithmetic

| Signature | Description |
|-----------|-------------|
| `sub(a: Field, b: Field) -> Field` | Subtraction: a + (p - b) |
| `neg(a: Field) -> Field` | Additive inverse: p - a |
| `inv(a: Field) -> Field` | Multiplicative inverse |

### U32 Operations

| Signature | Description |
|-----------|-------------|
| `split(a: Field) -> (U32, U32)` | Split field to (hi, lo) u32 pair |
| `as_u32(a: Field) -> U32` | Range-checked conversion |
| `as_field(a: U32) -> Field` | Type cast (zero cost) |
| `log2(a: U32) -> U32` | Floor of log base 2 |
| `pow(base: U32, exp: U32) -> U32` | Exponentiation |
| `popcount(a: U32) -> U32` | Hamming weight (bit count) |

### Assertions

| Signature | Description |
|-----------|-------------|
| `assert(cond: Bool)` | Crash VM if false — proof generation impossible |
| `assert_eq(a: Field, b: Field)` | Assert equality |
| `assert_digest(a: Digest, b: Digest)` | Assert digest equality |

### Memory

| Signature | Description |
|-----------|-------------|
| `ram_read(addr) -> Field` | Read 1 word |
| `ram_write(addr, val)` | Write 1 word |
| `ram_read_block(addr) -> Digest` | Read D words (D = digest width) |
| `ram_write_block(addr, vals)` | Write D words |

### Hash

| Signature | Description |
|-----------|-------------|
| `hash(fields: Field x R) -> Digest` | Hash R field elements into a Digest (R = target hash rate) |

`vm.crypto.hash.native(...)` is generated for the selected machine: eight Field arguments and a four-field Digest on nox, ten arguments and a five-field Digest on Triton. `vm.crypto.hash.single(value)` fills the remaining native inputs with zero. Algorithm and input layout belong to compilation identity; equal source arguments across targets do not imply equal digests. Explicit Tip5 is `vm.triton.hash.tip5(...)`, supplied by Trisha. A catalog entry alone does not implement hashing or any other intrinsic.

For sponge, Merkle, and extension field builtins (Tier 2-3), see
[Part II](#part-ii--provable-computation-tier-2--tier-3) below.

### Portable OS (`os.*`)

Portable `os.neuron`, `os.signal`, and `os.time` modules are design concepts, not implemented libraries. The compiler has an `os.state.read` builtin for nox. Joy authenticates public BBG certificates with JOYST001 and supports bounded hidden queries over complete public tables with JOYZK003. This does not implement live state synchronization or a private database. `lib/os/` is reserved for implemented portable contracts rather than populated with placeholders.

Network-specific SDKs are supplied by their owning runtime package. For example, `os.neptune.*` requires the explicit `neptune` target and lives in Trisha. The bare `triton` package does not export Neptune modules. See [os.md](os.md) for the design boundary and [warrior-api.md](warrior-api.md) for implemented package capabilities.

---

## 7. Attributes

| Attribute | Meaning |
|-----------|---------|
| `#[cfg(flag)]` | Conditional compilation |
| `#[test]` | Test function — run with `trident test` |
| `#[pure]` | No I/O side effects allowed |
| `#[intrinsic(name)]` | Declares a target intrinsic; signature and reachable capability are checked |
| `#[requires(predicate)]` | Precondition — modeled by `trident audit` in its [supported formal subset](formal-audit.md) |
| `#[ensures(predicate)]` | Postcondition — `result` refers to return value |

```trident
#[pure]
fn compute(a: Field, b: Field) -> Field { a * b + a }

#[requires(amount > 0)]
#[ensures(result == sub(balance, amount))]
fn withdraw(balance: Field, amount: Field) -> Field {
    sub(balance, amount)
}

#[test]
fn test_withdraw() {
    assert_eq(withdraw(100, 50), 50)
}
```

---

## 8. Memory Model

### Stack

The compiler manages logical stack positions. The target adapter implements
access beyond its machine's directly accessible stack window (16 words on
Triton). Compiler-generated cleanup and stack access must preserve source RAM.

The developer does not manage the stack.

### RAM

Word-addressed memory. Each cell holds one Field element.

Ordinary source RAM has no compiler-reserved address interval. Temporary memory
used to implement a stack operation must have its prior contents restored before
the next source operation, including inline assembly and target calls. An
explicitly documented runtime intrinsic may have its own memory preconditions.

```trident
ram_write(17, value)
let v: Field = ram_read(17)
```

RAM is non-deterministic on first read — if an address hasn't been written,
reading returns whatever the prover supplies. Constrain with assertions.

### No Heap

No dynamic allocation. No `alloc`, no `free`, no garbage collector. All data
structures have compile-time-known sizes. Machine legalization may temporarily
borrow zero RAM cells to implement a deep stack operation and restore those
cells afterwards. The amount needed follows from the stack depth; finding the
cells can depend on RAM contents, so a static size alone does not bound that
search's execution cost. Failure to find space must fail explicitly rather than
overwrite source memory.

---

## 9. Inline Assembly

```trident
asm(+1) { push 42 }                // pushes 1 element
asm { dup 0 add }                  // doubles the anonymous word, zero net effect
asm(-1) { write_io 1 }             // consumes that word
asm(triton)(+1) { push 42 }        // target-tagged + effect
asm(miden) { dup.0 add }           // MIDEN assembly
```

Target-tagged blocks are skipped when compiling for a different target.
A bare `asm { ... }` is treated as `asm(triton) { ... }` for backward
compatibility.

The compiler does not parse, validate, or optimize assembly contents. The effect
annotation `(+N)` / `(-N)` is the contract between hand-written assembly and
the compiler's stack model.

Assembly executes on the current operand stack. It may create and consume
anonymous words with its declared effect, but must preserve the compiler-owned
prefix, including live named variables and control-flow bookkeeping. A negative
effect that would consume a named binding is rejected. As with the declared net
effect, preserving that prefix inside opaque assembly is the author's obligation;
the compiler cannot infer clobbers from an unparsed instruction body. Named
locals are not moved to a hidden RAM frame around assembly. RAM reads and writes
inside assembly therefore see the same memory as surrounding source operations.

---

## 10. Events

Events are structured data output — the universal communication mechanism.
On provable targets, events are how programs talk to the OS. On native
targets, they're structured logging (like `console.log`).

### Declaration

Events are declared at module scope (see [Section 3](#3-declarations)):

```trident
event Transfer { from: Field, to: Field, amount: Field }
```

The complete flattened payload must fit nine field words, including all
coordinates of aggregate fields. A Triton `Digest` occupies five words.

### Reveal (Public Output)

```trident
reveal Transfer { from: sender, to: receiver, amount: value }
```

Public output is the event tag followed by the flattened payload in declaration
order. Tags are zero-based declaration indices in the compiled module. Struct
fields follow their declaration order; arrays and tuples follow element order.
Payload expressions are evaluated once in declaration order, regardless of the
order of named fields in the statement. The verifier sees all data.

Triton implements this event wire format. The current nox compiler rejects
`reveal` and `seal`; they are not portable to nox yet.

### Seal (Committed Secret)

```trident
seal Transfer { from: sender, to: receiver, amount: value }
```

Triton applies fixed-length Tip5 to `[tag, payload..., zero padding...]`, a
ten-word block, and emits its five digest coordinates in canonical order.
Payload evaluation and flattening match `reveal`. This corrects the previous
reversed/misindexed event payload and misplaced seal padding: consumers of old
artifacts must regenerate expected output and commitments.

Only the digest is emitted by this statement. Whether the execution proof hides
the payload depends on the selected proof backend; the statement itself does
not supply encryption, randomness, or protection against guessing small inputs.

---

## 11. Audit vs Verify

Two distinct commands share the "check correctness" concept:

| Command | What it checks | How | Where |
|---------|---------------|-----|-------|
| `trident audit` | Source code contracts | Symbolic execution, algebraic solver | Local (Trident) |
| `trident verify` | Proof artifacts | STARK/SNARK proof verification | Warrior (external) |

`audit` checks supported straight-line scalar `#[requires]`/`#[ensures]`
contracts without executing the program. Static tautologies or an UNSAT solver
query can discharge obligations; sampling is not a universal proof. Unsupported
semantics and absent obligations return UNKNOWN with nonzero exit status. See
[the formal audit contract](formal-audit.md) for the exact scope and exit codes.

`verify` checks a proof file produced by `trident prove`. It delegates
to a warrior binary that has the target-specific verifier (e.g. triton-vm's
`verify()` function). "Client" and "warrior" are two naming registers
for the same concept — geeky and gamy respectively. Both refer to the
external binary that handles execution, proving, and verification for a
specific battlefield (target). See [targets.md](targets.md#warriors).

---

## 12. Type Checking Rules

- No implicit conversions between any types
- No recursion — the compiler rejects call cycles across all modules
- Exhaustive match required (wildcard or all cases covered)
- `#[pure]` functions cannot perform I/O (`pub_read`, `pub_write`, `divine`,
  `sponge_init`, etc.)
- `#[intrinsic]` only allowed in std modules
- `asm` blocks tagged for a different target are rejected
- Dead code after unconditional halt/assert is rejected
- Unused imports produce warnings

---

## 13. Permanent Exclusions

These are design decisions, not roadmap items.

| Feature | Reason |
|---------|--------|
| Strings | No string operations in any target VM ISA |
| Dynamic arrays | Unpredictable trace length |
| Heap allocation | Non-deterministic memory, no GC |
| Recursion | Unbounded trace; use bounded loops |
| Closures | Requires dynamic dispatch |
| Type-level generics | Compile-time complexity, audit difficulty |
| Operator overloading | Hides costs |
| Inheritance / Traits | Complexity without benefit |
| Exceptions | Use assert; failure = no proof |
| Floating point | Not supported by field arithmetic |
| Macros | Source-level complexity |
| Concurrency | VM is single-threaded |
| Wildcard imports | Obscures dependencies |
| Circular dependencies | Prevents deterministic compilation |

---

# Part II — Provable Computation (Tier 2 + Tier 3)

The implemented operations below require explicit intrinsic capabilities from the selected package. Triton provides them through Trisha; nox does not provide this sponge/Merkle/extension-field ABI. Catalog tiers are design classifications, not installed backend support.

Target-native `hash()` is documented in
[Section 6](#6-builtin-functions). The builtins below are Tier 2+.

---

## 14. Sponge

The sponge API enables incremental hashing of data larger than R fields.
Initialize, absorb in chunks, squeeze the result. The rate R is
10 for the implemented Triton package. `vm.triton.hash.sponge_squeeze()` returns `[Field; 10]`, not a five-field Digest.

| Signature | IR op | Description |
|-----------|-------|-------------|
| `sponge_init()` | `SpongeInit` | Initialize sponge state |
| `sponge_absorb(fields: Field x R)` | `SpongeAbsorb` | Absorb R fields |
| `sponge_absorb_mem(ptr: Field)` | `SpongeLoad` | Absorb R fields from RAM |
| `sponge_squeeze() -> [Field; R]` | `SpongeSqueeze` | Squeeze R fields |

---

## 15. Merkle Authentication

| Signature | IR op | Description |
|-----------|-------|-------------|
| `merkle_step(idx: U32, d0, d1, d2, d3, d4: Field) -> (U32, Digest)` | `MerkleStep` | One tree level up |
| `merkle_step_mem(idx: U32, d0, d1, d2, d3, d4, ptr: Field) -> (U32, Digest, Field)` | `MerkleLoad` | Tree level from RAM |

`vm.triton.merkle.step` wraps the flattened Triton builtin. Its sibling digest comes from Triton's nondeterministic digest queue. `vm.triton.merkle_proof.verify` provides the repeated path operation. These modules belong to Trisha; the previous generic `vm.crypto.merkle` and `std.crypto.merkle` aliases are absent.

---

## 16. Extension Field

The implemented extension field has degree three on Triton. Availability also requires the corresponding package intrinsics; a descriptor width alone does not implement arithmetic.

### Type

| Type | Width | Description |
|------|------:|-------------|
| `XField` | E | Extension field element (E = `xfield_width` from target config) |

### Operator

| Operator | Operand types | Result type | Description |
|----------|---------------|-------------|-------------|
| `a *. s` | XField, Field | XField | Scalar multiplication |

### Builtins

| Signature | IR op | Description |
|-----------|-------|-------------|
| `xfield(x0, ..., x(E-1)) -> XField` | *(constructor)* | Construct from E base field elements |
| `xinvert(a: XField) -> XField` | `ExtInvert` | Multiplicative inverse |
| `xx_dot_step(acc: XField, ptr_a: Field, ptr_b: Field) -> (XField, Field, Field)` | `FoldExt` | XField dot product step |
| `xb_dot_step(acc: XField, ptr_a: Field, ptr_b: Field) -> (XField, Field, Field)` | `FoldBase` | Mixed dot product step |

Dot steps return the updated accumulator and both RAM pointers. `xx_dot_step` advances both pointers by three; `xb_dot_step` advances the extension pointer by three and the base-field pointer by one. Coefficients use source tuple order; Trisha converts to Triton native ordering during lowering. These are arithmetic primitives, not a complete recursive proof verifier.

Note: The `*.` operator (scalar multiply) maps to `ExtMul` in the IR.

---

## 17. Proof Composition (Tier 3)

Recursive proof composition is not implemented by the released Neptune SDK. The old `os.neptune.proof.verify_inner_proof` prototype failed to constrain computed FRI/OOD/constraint values and is excluded from the production target package. Its source and dependent examples remain in `trisha/examples/experimental/neptune`; importing `os.neptune.proof` fails compilation.

`ProofBlock` currently supplies an IR container, not an authenticated verifier circuit. `FoldExt`, `FoldBase`, `ExtMul`, and `ExtInvert` supply arithmetic only. Their presence must not be interpreted as verification of an inner proof or a Neptune transaction. Trisha's CPU Triton STARK prover/verifier is a separate, implemented capability.

---

## 🔗 See Also

- [Agent Briefing](briefing.md) — AI-optimized compact cheat-sheet
- [Standard Library](stdlib.md) — `std.*` modules
- [OS Reference](os.md) — OS concepts, `os.*` gold standard, extensions
- [VM Reference](vm.md) — VM registry, lowering paths, cost models
- [CLI Reference](cli.md) — Compiler commands and flags
- [Grammar](grammar.md) — EBNF grammar
- [IR Reference](ir.md) — Compiler intermediate representation (54 ops, 4 tiers)
- [Target Reference](targets.md) — OS model, integration tracking, how-to-add checklists
- [Error Catalog](errors.md) — All compiler error messages explained
- [Tutorial](../docs/tutorials/tutorial.md) — Step-by-step developer guide

---

*Availability is defined by the resolved compiler and warrior package capabilities.*
