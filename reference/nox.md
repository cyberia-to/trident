# nox compilation

Trident's default target lowers type-checked AST modules to nox noun
formulas. The stack TIR is the interface to foreign warriors; nox uses
its own tree representation. Joy executes the emitted formula and
produces and verifies Zheng proofs.

The nox backend accepts the `nox` target only. Other tree architectures,
including Nock, require their own lowering and cannot receive a nox
formula through an implicit fallback.

## Module and profile semantics

The compiler resolves and type-checks modules in dependency order.
`#[cfg(flag)]` selects functions, constants, structs and events using
the active compile flags, including the entry function. An inactive
item cannot replace an active definition during lowering.

Function bodies retain their defining module's constants, private
helpers, struct layouts and import aliases. Qualified calls such as
`pkg.math.add` and the imported short alias `math.add` refer to that
module's function. Equal member names in different modules have
distinct identities. Calls inline the callee against a fresh parameter
subject after evaluating arguments in the caller's scope.

Local bindings and parameters shadow module constants. A shadowed
constant is unavailable to compile-time folding of loop bounds and
array indices. A dynamic bound uses the bounded-loop lowering; a
dynamic index is rejected until the target supports it.
Each unrolled loop iteration carries its immutable U32 index as a scoped
compile-time value. Array reads and writes may use that index, including in
bounded loops with runtime end conditions. A nested or shadowing binding keeps
its own value; an ordinary runtime variable does not inherit the loop index.

Imported `#[intrinsic(...)]` declarations select the corresponding
builtin lowering. A declaration whose builtin has no nox implementation
produces a compilation error.

Assembly, static reduction costs and bundle state metadata use the same
resolved-module lowering and active compile flags. Bundle metadata
lists active functions. Compilation errors propagate to the caller.

## Subject and supported surface

The external input is a flat sequence of canonical field words, represented by
Joy as `[word_last [... [word0 0]]]`. The selected source entry consumes exactly
its typed signature: parameter/struct declaration order and increasing tuple,
array and digest limb order. Missing or extra words fail, even for unused
parameters. Bool words are0 or1 using the existing nox convention (0=true,
1=false); U32 words are below2^32. The generated formula checks these conditions,
so execution and execution proofs enforce the same input contract.

The compiler reconstructs the ordinary internal parameter subject
`[param_last [... [param0 0]]]`: structs/tuples/arrays use zero-terminated lists;
Digest uses the native balanced four-limb tree. Ordinary function calls already
construct this subject and do not receive an external adapter. XField has no
implemented nox representation. Entry layout is bounded by58 public words,
58 aggregate elements per list and16 nested aggregate levels. Unresolved or
excessive layouts fail compilation before constructing an unbounded formula.
Deep aggregate navigation uses consecutive canonical axes of at most62 edges,
without truncating a longer path into one field word. Reads and writes preserve
all path segments; each write evaluates its RHS once against the original
subject and reconstructs the untouched siblings. Paths are bounded to4096 edges;
edit emission also observes the compiler's2,000,000-node construction budget.
A program with a reachable state read receives the BBG root above the external
words. The adapter preserves that root above the reconstructed parameters; its
bundle sets `reads_state`, which tells the warrior to supply the root.

Compiler API3 includes this typed source-entry contract. Raw hand-authored nox
formulas retain their own subject contract. Exported compiled nox formulas carry
the checks themselves and require no runtime metadata to reconstruct inputs.

The supported surface includes field arithmetic and comparisons,
assertions, mutable local bindings, conditionals, early returns,
bounded loops, nonrecursive function calls, structs, arrays with
constant indices, tuples, one-shot hash, scalar `divine()` and state
reads in entry functions and imported or local helpers.

State dependence follows the active, module-qualified call graph. A stateful
callee receives `[root_tree [arg_last ... [arg0 0]]]`; a stateless callee keeps
the ordinary parameter-only subject. The hidden root survives local bindings,
conditional calls and transitive imported helpers. Unqualified user functions
shadow builtin names consistently during both call resolution and state
analysis. Inactive cfg definitions and unreachable helpers do not change the
selected entry subject or its `reads_state` metadata.

Current limits: calls are statically inlined; recursive calls are
rejected. Dynamic array indexing, extension-field operations, incremental sponge,
mutable RAM and stream I/O builtins produce explicit errors. Inlining
has a node budget; static loop unrolling and subject depth have fixed
limits. Constant declarations currently require integer literals, and
size-generic calls require explicit literal size arguments.

Executable coverage: `tests/nox_surface.rs` compiles source and
projects, reduces their emitted formulas on nox, and compares runtime
results and reduction bills with the bundle and cost APIs.

## Native compiler data (0.4 development)

[SH0.2](self-hosting-data.md) specifies first-class Noun values, canonical
persistent sequences and four-byte U32 packing for exact source bytes. This
extension now implements the Noun type, seven `vm.nox.noun` intrinsics and
raw ART1 entry/emission. The limits and flat-word ABI above describe the
legacy adapter; Noun entries require `joy build --emit artifact`.
Host arena IDs must never stand in for Noun values.

The minimal operations can use existing nox patterns: cons, checked axis/atom
projection, full particle equality and identity. Dynamic lookup can construct
an axis formula and evaluate it with deterministic composition. No guest
`is_atom` operation is assumed, and witness calls cannot supply compiler work.
Native Boolean results retain 0=true / 1=false. The
[conformance evidence](../audit/self-hosting/native-data.md) records which small
formulas actually ran. Joy transports and executes complete raw artifacts;
source collection libraries, reusable runtime loops/calls, production JOB1/RES1
admission and compiler-scale Zheng coverage remain later gates.

[SH0.3](self-hosting-jobs.md) defines explicit structured raw-noun/compiler-job
profiles and canonical NOXDAG01 transport. These preserve complete result roots;
they do not use the existing flat output words as a program artifact. ART1
contains executable metadata/formula, while RES1 binds a particular compile job.
Keeping producer/job identity outside ART1 is required for C2/C3 byte equality.
