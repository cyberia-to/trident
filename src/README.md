# Source Architecture

Trident owns the frontend, shared IR and reference nox lowering. Trisha owns
Triton lowering and runtime integration. Joy owns the nox/Zheng warrior.
See the [target reference](../reference/targets.md) and
[ownership review](../audit/target-ownership.md).

```text
syntax -> AST -> module resolution -> target-aware typecheck
                                      |
                         +------------+-------------+
                         |                          |
                   ir/tree/lower/nox          ir/tir/builder
                         |                          |
                   .nox formulas           Trisha lowering -> .tasm
                         +------------+-------------+
                                      |
                                 ProgramBundle
                                      |
                                Joy / Trisha
```

## Module map

| Directory | Responsibility |
|---|---|
| [`syntax/`](syntax/) | Lexing, parsing, AST and formatting |
| [`typecheck/`](typecheck/) | Target-aware signatures, types, generics and borrow checks |
| [`api/`](api/) | Public compilation/checking APIs and `CompileOptions` |
| [`config/`](config/) | Project configuration, resource resolution and versioned target packages |
| [`ir/tir/`](ir/tir/) | Typed shared stack IR, builder, stack model and semantic optimization |
| [`ir/tree/`](ir/tree/) | Reference nox noun/formula compilation |
| [`runtime/`](runtime/) | `ProgramBundle` and runtime adapter traits |
| [`cost/`](cost/) | Generic cost interfaces and analysis; Triton model lives in Trisha |
| [`verify/`](verify/) | Compiler symbolic analysis and formal-checking tools |
| [`package/`](package/) | Content-addressed definitions, registry and dependencies |
| [`deploy/`](deploy/) | Target-format artifacts and manifest integrity checking |
| [`lsp/`](lsp/) | Editor support using selected target metadata |
| [`cli/`](cli/) | Commands, target selection and warrior delegation |

[`lib.rs`](lib.rs) re-exports public interfaces. Filesystem library layout is
outside `src/`: portable source modules in `lib/`, discovery/design records
in `catalog/`. Build embedding makes these compiler resources available in
installed binaries.

## Boundaries

`CompileOptions::with_package` supplies ABI, module contents and intrinsic
capabilities together. Generated `std.target` constants follow that same
ABI. Package identity and effective module contents participate in bundle
source identity.

Typed TIR crosses the in-process Trident/Trisha compiler boundary; generated
assembly crosses the runtime boundary in `ProgramBundle`. Trident contains
no production Triton lowering or Neptune SDK. A frozen foreign machine
fixture exists solely for shared compiler unit tests.

Other machine catalog entries are declarations, not implemented lowerers.
Likewise native execution coverage and proof coverage are separate: the
public Zheng certificate has a bounded supported surface and no secret,
state or zero-knowledge guarantee.
