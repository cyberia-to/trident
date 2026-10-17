# Compiling a Program

`trident build` compiles `.tri` source with the Rust frontend. The default
target is nox, which produces a noun formula for Joy. Triton builds use the
installed Trisha runtime to produce Triton Assembly (TASM).

To compile source with the self-built C2 compiler running inside Joy/nox,
follow [Self-Built Compiler](self-hosted-compilation.md). That path uses an
explicit source package and the supported [native compiler subset](../../reference/self-hosting.md#native-compiler-subset-contract).

## Build a source file

```sh
trident build main.tri
```

For a standalone source file with no project target setting, this writes
`main.nox` and prints `Compiled -> main.nox` to stderr. Select the target and
output path explicitly when needed:

```sh
trident build main.tri --target nox -o program.nox
```

The `.nox` file contains the textual noun formula. To retain program metadata
in a bundle for Joy, use its ordinary Rust-backed build command:

```sh
joy build main.tri --emit bundle -o main.bundle.json
```

A bundle, a textual `.nox` formula and a structured ART1 artifact are distinct
formats. The C2 guide uses ART1 with `joy run-artifact`; it documents the
required compiler and input artifacts.

## Build a project

A directory input selects the entry declared in `trident.toml`:

```toml
[project]
name = "my_project"
version = "0.1.0"
entry = "src/main.tri"
target = "nox"

[targets.debug]
flags = ["debug"]

[targets.release]
flags = ["release"]
```

```sh
trident build .
trident build . --profile release
```

The compiler discovers modules reachable from the selected entry. For nox,
the default output is `my_project.nox` in the project root. `entry` defaults
to `main.tri` when omitted. An explicit `--target` overrides the project
target; otherwise the project setting applies, falling back to nox.

Passing a file inside the project keeps that file as the entry. The enclosing
manifest supplies settings and dependencies:

```sh
trident build src/other.tri --target nox -o other.nox
```

`--profile` selects conditional-compilation flags, including a matching
`[targets.<profile>]` section. It is not an optimization-level switch.

## Check source and resolve modules

```sh
trident check main.tri
trident check . --target nox
```

`check` resolves and type-checks the reachable modules without writing a
compiled artifact. Success prints `OK: <input>` to stderr; discovery, parsing
or type errors produce diagnostics and a nonzero exit status. Use the same
target and profile for checking and building. Checking source does not run
the program or establish that it can be proved.

Imports use dotted module names. Local modules are resolved relative to the
entry file's directory: `use crypto.sponge` names `crypto/sponge.tri` there.
Explicit project dependencies and locked dependency paths also participate
in resolution. Imported declarations must match their requested module
owners; dependency cycles fail compilation.

The compiler embeds its owned library sources, whose repository locations
include `lib/std/` and `lib/vm/`. Target packages supply their own intrinsic
and runtime modules. Custom libraries belong in explicit dependencies;
ambient `TRIDENT_STDLIB` or `TRIDENT_OSLIB` overrides are not the current
library-selection interface. In particular, `os.neptune.*` belongs to
Trisha's Neptune package and requires the `neptune` target.

See [Programs and Modules](../../reference/language.md#1-programs-and-modules)
for import and visibility rules, and the [Error Catalog](../../reference/errors.md)
for diagnostic explanations.

## What the compiler produces

The Rust frontend resolves modules, parses source, checks types and
specializes concrete generic uses before target lowering. The two implemented
paths have different representations:

- **nox:** Trident lowers checked AST modules directly into noun formulas.
- **Triton:** Trident builds typed TIR and applies its TIR optimizations;
  Trisha legalizes operations, renders instructions and links the modules
  into TASM.

The [IR reference](../../reference/ir.md) describes those boundaries.
Available intrinsics and lowering support come from the selected target
package. A target catalog entry alone does not implement a backend.

## Triton-specific builds

With a compatible `trisha` installed on the command path:

```sh
trident build main.tri --target triton -o main.tasm
trident build main.tri --target neptune -o main-neptune.tasm
```

`triton` selects the bare VM package; `neptune` also supplies its OS-specific
modules. They are separate selections. The Trident CLI delegates these
builds to Trisha, which emits a linked TASM program with an entry point and
module-qualified function labels.

Use one target selector per command. `--engine` and `--terrain` name VM
selections; `--network` and `--union` name OS selections. These four flags
are mutually exclusive. `--target` is the common shorthand.

## Inspect compilation cost

```sh
trident build main.tri --target nox --costs
trident check main.tri --target nox --costs
```

For nox, `--costs` analyzes the emitted formula and reports a reduction model
and structural counts. Branch-dependent work is reported as a range. This
is static analysis, not a measurement of execution time, proof time or
proof size.

For Triton, `trident build main.tri --target triton --costs` delegates the
report to Trisha's AET-table model, whose current build report analyzes the
entry source alone. `trident check --costs` reports the nox model only and
prints a note for stack targets. Use the runtime's own measurements when
evaluating execution or proving performance.

## Next steps

- [Running a Program](running-a-program.md)
- [Self-Built Compiler](self-hosted-compilation.md)
- [Language Reference](../../reference/language.md)
- [Warrior API](../../reference/warrior-api.md)
