# Trident Intermediate Representation

Typed stack IR shared by the Trident frontend and the Trisha adapter.
The reference nox compiler follows its own AST-to-noun path, not this stack
pipeline. See the [IR reference](../../../reference/ir.md).

## Structure

- [`mod.rs`](mod.rs): `TIROp` definitions and debug formatting.
- [`builder/`](builder/): AST-to-IR translation using target ABI widths.
- [`stack/`](stack/): typed stack effects, bindings and spill layout.
- [`optimize/`](optimize/): semantic transformations over typed operations.

Triton legalization, instruction rendering and linking live in
`trisha/rs/lower/`, not a `lower/` module inside this directory. The builder
emits typed operations directly rather than generating TASM strings for a
second parser.

Structural `IfElse`, `IfOnly` and `Loop` operations retain nested bodies.
Counts and depths in stack operations express semantic effects; the target
adapter enforces instruction limits. Inline `Asm` is explicitly target
assembly, even though the surrounding operation is typed.

The operation tier names organize the source; they do not promise support
on every catalog machine. The resolved package's intrinsic list and actual
backend implementation determine which operations are available.

Target parameters come from `TerrainConfig` through compilation options.
The full resource/capability contract is documented in
[Warrior API](../../../reference/warrior-api.md).
