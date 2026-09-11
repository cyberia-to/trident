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
| Program and control flow | `FnStart`, `Entry`, `Call`, `Return`, `IfElse`, `Loop` | Structural intent; backend chooses labels and branch instructions |
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

The shared [`StackManager`](../src/ir/tir/stack/mod.rs) tracks bindings,
positions and spills. The builder produces typed operations, including
cleanup and spill sequences. Optimizations in Trident preserve semantic
stack effects. They must not introduce Triton instruction count limits into
the common representation.

Trisha handles Triton instruction limits, batching, deep access legalization,
rendering and linking under `trisha/rs/lower/`. For example, a shared `Pop(n)`
is a logical effect; legal Triton batches are an adapter concern. Stack
layout configuration is supplied by the target ABI. It is not a global
compiler constant.

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
Calls, secrets, state access and dynamic continuations are outside the
current public certificate relation.

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
linear verification; it does not provide ZK or state proof support.

See [Warrior API](warrior-api.md) for package boundaries and the
[ownership review](../audit/target-ownership.md) for migration gates.
