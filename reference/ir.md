# Trident IR Architecture

[Language](language.md) · [Targets](targets.md) · [VM contracts](vm.md)

Trident owns source semantics, typechecking and shared typed IR. A foreign
warrior owns instruction selection and assembly legality. The reference nox
compiler currently lowers AST modules directly into noun formulas.

```text
Source -> module resolution -> AST -> target-aware typechecking
                                      |
                    +-----------------+----------------+
                    |                                  |
                nox target                         stack target
                    |                                  |
         Trident NoxCompiler                  Trident TIRBuilder
                    |                                  |
              noun formulas                    typed Vec<TIROp>
                    |                                  |
                  .nox                         Trisha legalization,
                    |                         lowering and linking
                    |                                  |
                    |                                .tasm
                    +-----------------+----------------+
                                      |
                                 ProgramBundle
                                      |
                         warrior execution / proving
```

`ProgramBundle` is the runtime artifact boundary. Typed TIR is an additional
in-process compilation boundary used by Trisha. This diagram does not imply
that every TIR operation is supported on nox or that a generic register, Nock,
circuit or GPU backend exists.

## Shared TIR

The operation definitions live in
[`src/ir/tir/mod.rs`](../src/ir/tir/mod.rs). TIR uses typed stack operations and
nested control flow. The builder tracks value widths and shared stack effects;
it does not render target instruction text and then parse it back into IR.

| Group | Examples | Contract |
|---|---|---|
| Program and control flow | `FnStart`, `EntryParameters`, `Entry`, `Call`, `Return`, `IfElse`, `Loop` | Structural intent; backend chooses labels and branch instructions |
| Stack values | `Push`, `Pop`, `Dup`, `Swap` | Typed stack effects; counts and depths must be legalized for the target |
| Arithmetic and comparison | `Add`, `Mul`, `Invert`, `Eq`, `Lt` | Language operation with target field and representation |
| Word operations | `Split`, `And`, `Xor`, `Shl`, `DivMod` | Backend support and operand constraints required |
| I/O and memory | `ReadIo`, `WriteIo`, `ReadMem`, `WriteMem`, `RamRead`, `RamWrite` | Target-dependent runtime ABI |
| Hash and witnesses | `Hash`, `Hint`, sponge and Merkle operations | Explicit native hash and witness conventions |
| Extension-field/proof operations | Extension arithmetic, folding, verification operations | Require actual backend implementation |
| Passthrough | `Asm { lines, effect }` | Target-specific assembly with declared effect |

The tier labels retained in source organize operation families. They are not
a capability theorem: a tier called universal does not mean every catalog
machine or runtime implements it. An intrinsic must be accepted by the
resolved package and supported by the selected lowering.

## Stack model and target legality

The shared [`StackManager`](../src/ir/tir/stack/mod.rs) tracks bindings and
positions across the full logical stack. The production builder produces typed
cleanup operations and keeps live locals on that stack across source operations.
Optimizations in Trident preserve semantic
stack effects. They must not introduce Triton instruction count limits into
the common representation.

Return-frame cleanup uses abstract stack permutations and pops, preserving the
order of every result word without allocating source RAM. Target legalization
must also preserve source RAM across every source-visible boundary. Consecutive
pure stack operations may share one temporary frame, restored before any other
operation. An internal fixed scratch address is not a public memory reservation.

Trisha handles Triton instruction limits, batching, deep access legalization,
rendering and linking under `trisha/rs/lower/`. For example, a shared `Pop(n)`
is a logical effect; legal Triton batches are an adapter concern. Stack
layout configuration is supplied by the target ABI. It is not a global
compiler constant.

Every function consumes its complete parameter/local frame and leaves exactly
its declared result width, including a function ending in an `if` statement.
Branch cleanup preserves only the returned value, never branch-local bindings.
Calls and aggregate assignments preserve all words below their operand frame.
Source `return` is normalized into a hygienic typed result slot, a function-local
exit flag, and guarded continuations before structural lowering. This preserves
lexical bindings and skips later effects across nested branches and loops. Loop
counters terminate immediately when the flag is set. Synthetic target branch
subroutines therefore never confuse a source function exit with their own return.
Stack-target recursive source calls remain rejected by type checking.

Fixed-size array reads and projected assignments use declared element widths,
including arrays of structs and nested arrays. Runtime indices are checked
before selection (and before an assignment RHS). Balanced structural branches
select the element in O(log n) comparisons with O(n) generated code; they never
turn an unchecked index into a RAM address. Constant reads select directly.
Compound fields and elements replace all their words while preserving neighbors.
Match evaluates its scrutinee once and selects the first matching arm. Field
bindings and arm locals have branch scope; a final match preserves only its
result. All arms share the same caller frame and use structural conditionals.

Inline assembly remains target-specific. Its names and completion metadata
come from the target package, and its declared effect cannot by itself prove
that the assembly has that effect.

## Reference nox lowering

Nox's compiler lives in
[`src/ir/tree/lower/nox.rs`](../src/ir/tree/lower/nox.rs) and its module helpers.
It handles the supported source surface by constructing nox formulas. Public
arguments are subject leaves; results are noun leaves. Imports, function
inlining and bounded source loops are resolved before runtime.

The resulting formulas are executed by nox through Joy. Zheng supports a
bounded static public proof subset of those formulas. Successful lowering
or native execution does not imply that a certificate can be generated.
Private backends constrain atom calls and secrets. State backends bind reads
to authenticated tables; private queries over public tables are bounded to
2048 fields. Dynamic continuations and incompatible branch shapes still exceed
the current relation. See Zheng's execution backend contract for exact bounds.

## Target propagation and identity

`CompileOptions::with_package` installs the target ABI, module sources and
intrinsic set together. `std.target` is generated from that ABI. Source
checking and building must use those same inputs; editor defaults must not
select Triton when the compiler defaults to nox.

The bundle compilation identity includes the selected ABI, package
compilation identity, configuration flags and effective module source
contents. Target-specific modules cannot be substituted without changing
identity. Deployment presets are a separate runtime selection.

## Cost and verification boundaries

Trident provides generic cost interfaces and analysis infrastructure.
Trisha owns Triton instruction costs. Native execution reports measured
reductions or cycles through the warrior; compiler estimates are not runtime
measurements and are not proof-size benchmarks.

Compiler symbolic analysis, target execution and cryptographic verification
are distinct checks. The current Zheng certificate authenticates the
supported public execution relation with a full disclosed witness and
linear verification. Joy also provides a separate randomized Triton-backed
private checker protocol and authenticated public/private-query state protocols.

See [Warrior API](warrior-api.md) for package boundaries and the
[ownership review](../audit/target-ownership.md) for migration gates.

### Event payload stack contract

`Reveal` and `Seal` consume `field_count` flattened field words, with the first
declared coordinate deepest and the last on top. `field_count` counts words,
not AST fields. The builder retains every evaluated payload in its stack model
until all expressions have been evaluated once in declaration order; machine
adapters then reorder the payload for their native I/O or hash ABI. Triton
reveals `tag, payload...` and seals `tag, payload..., zeros` as one ten-word Tip5
block. Both consume the entire payload and leave no result on the operand stack.
The AST cost estimator reserves the full nine-word payload/permutation envelope
when it lacks checked layouts. Nox currently rejects event statements explicitly.

Compiler API 3 includes `EntryParameters`: primitive leaves of the executable
entry signature in source declaration order. This metadata contains no machine
instructions. The owning warrior marshals and validates its inputs before
calling the entry; unresolved leaves must be rejected. Library functions retain
the ordinary caller ABI. See [the warrior contract](warrior-api.md).
