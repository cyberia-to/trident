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

Imported `#[intrinsic(...)]` declarations select the corresponding
builtin lowering. A declaration whose builtin has no nox implementation
produces a compilation error.

Assembly, static reduction costs and bundle state metadata use the same
resolved-module lowering and active compile flags. Bundle metadata
lists active functions. Compilation errors propagate to the caller.

## Subject and supported surface

Entry parameters form a right-nested list, last parameter first:
`[param_last [... [param0 0]]]`. A program that directly reads state
receives the BBG root above the parameters. Its bundle sets
`reads_state`, which tells the warrior to supply that root.

The supported surface includes field arithmetic and comparisons,
assertions, mutable local bindings, conditionals, early returns,
bounded loops, nonrecursive function calls, structs, arrays with
constant indices, tuples, one-shot hash, scalar `divine()` and state
reads in the entry function.

A reachable state read in a called function, including an imported or
transitively called function, currently rejects compilation, cost
analysis and bundle construction. It cannot produce a bundle with an
incorrect `reads_state = false`. Inactive cfg definitions and unreachable
helpers do not change the entry subject.

Current limits: calls are statically inlined; recursive calls are
rejected. A return inside a loop, state reads inside a called function,
dynamic array indexing, extension-field operations, incremental sponge,
mutable RAM and stream I/O builtins produce explicit errors. Inlining
has a node budget; static loop unrolling and subject depth have fixed
limits. Constant declarations currently require integer literals, and
size-generic calls require explicit literal size arguments.

Executable coverage: `tests/nox_surface.rs` compiles source and
projects, reduces their emitted formulas on nox, and compares runtime
results and reduction bills with the bundle and cost APIs.
