# Virtual Machine Reference

[Target ownership](targets.md) · [IR](ir.md) · [Runtime contracts](os.md)

A VM contract describes field and value widths, native hashing, the artifact
format and compiler-visible operations. Execution and proof support belong
to the installed warrior and must be checked separately.

## Implemented compilation paths

| Target | Compilation | Artifact | Runtime owner |
|---|---|---|---|
| nox | Trident AST/module lowering into noun formulas | `.nox` bracket notation | nox, integrated by Joy |
| Triton | Trident typed TIR consumed and legalized by Trisha | `.tasm` | Trisha / Triton VM |

Nox does not pass through a generic stack-to-tree TIR lowering. Triton's
instruction selection, stack instruction limits, rendering and linking are
owned by Trisha. There is no currently installed Trident Miden, Nock, RISC-V,
EVM, WASM or native register backend just because a catalog record exists.
Register, circuit and GPU lowering diagrams in older versions of this
reference described proposed architecture, not current implementations.

## Source namespaces and ownership

`lib/vm/` holds generic language intrinsic contracts. It is separate from
`catalog/vm/`, which contains discovery and design information. Trisha's
`targets/triton/` owns the authoritative Triton ABI and ISA metadata;
Trident's catalog contains its owner reference.

Trisha provides the explicit `vm.triton.*` modules for Tip5, sponge and
Triton Merkle witness conventions. Generic `vm.io.io` declarations depend
on the resolved target. Nox public inputs and outputs are function arguments
and results; nox does not acquire Triton I/O queues or RAM simply because
those names exist in the language's signature catalog.

`std.target` is generated during module resolution. Typechecking, codegen,
editor metadata and SDK resolution must use the same target package.
Unsupported intrinsics produce diagnostics rather than selecting a foreign
ABI implicitly.

## Machine parameters

| Parameter | nox | Triton |
|---|---|---|
| Architecture | Tree | Stack |
| Native field | Goldilocks | Goldilocks |
| Native hash | Hemera | Tip5 |
| Digest field elements | 4 | 5 |
| Hash rate | 8 | 10 |
| Operand stack depth | 0 | 16 |
| Artifact suffix | `.nox` | `.tasm` |

Equal field representation does not make hash functions, digest encodings or
proof witness conventions interchangeable. A native hash operation is part
of the target ABI. Algorithms requiring a specific hash must import that
specific implementation.

## Declared machines

The records under [catalog/vm](../catalog/vm/) preserve machine descriptions
and design work. `status = declared` records cannot authorize build, run,
prove or deploy. Old numeric levels, proposed output extensions and notes in
individual catalog documents are not evidence of an installed implementation.

To add an implementation, provide the actual lowering and runtime owner,
a versioned target package with capabilities, content-identified modules,
and target-specific conformance tests. Editing a TOML descriptor is not a
backend implementation.

## Execution and proof boundaries

Joy's nox runner can execute a broader surface than Zheng currently proves.
The public certificate supports bounded static tags 0–15 with documented
restrictions. Calls, state lookups, secret witnesses and dynamic continuation
proofs are unavailable. Certificates disclose the full witness and are not
zero knowledge or succinct.

For Trisha, consult its installed `describe` output and release evidence.
A shared TIR operation, native instruction or SDK source file alone does not
establish proof or network transaction support. See
[Warrior API](warrior-api.md) and the
[ownership review](../audit/target-ownership.md).
