# SH0.1 — compiler source and migration inventory

Date: 2026-09-23. Baseline Trident source:
`360b737e073ca2f969ab0c78460b4228bcac7b78` (unchanged compiler modules from
release `531e93c4a08bd715a8e8ec61d2089ba7768c6d39`).
Delivery branch: `feat/0.4-sh0-inventory`; integration: `release/0.4`.

The [generated inventory](compiler-subset.json) parses the checkout with the
Rust AST parser and follows transitive `use` imports. It covers **10 modules,
985 function declarations, 9,288 lines and 321,415 exact source bytes**. The
seven compiler modules account for 9,237 lines / 320,140 bytes; their three
intrinsic modules add 51 lines / 1,275 bytes.

This is a syntactic source inventory: all declarations, including unused ones
and inactive cfg branches if present. It is not call-graph reachability, inferred
type coverage, a support certificate or a runnable compiler project. Contract
annotations are inventoried as attributes; their string predicates and inline
assembly are not recursively parsed as Trident expressions.

## Module disposition

The names below identify current modules. Proposed native replacements are
implementation responsibilities, not claims that those modules exist.

| Current module | Functions | Bytes | Native disposition / owner |
|---|---:|---:|---|
| `std.compiler.lexer` | 88 | 30261 | Reuse token rules; replace RAM/source buffers with native byte/sequence state. Trident, SH1/SH3 |
| `std.compiler.parser` | 269 | 93548 | Reuse parsing rules; repair child/item connectivity and move AST/work stacks to native collections. Trident, SH3 |
| `std.compiler.typecheck` | 189 | 57287 | Reuse checking rules; repair names/parameters/scopes, replace RAM tables and encode native data types. Trident, SH3 |
| `std.compiler.pipeline` | 46 | 10739 | Replace fixed scratch addresses and TIR result with the native compiler job/result pipeline. Trident, SH1/SH2 |
| `std.compiler.codegen` | 208 | 69183 | Replace stack-TIR generation with typed AST-to-nox generation. Preserve relevant source semantics and regression cases. Trident, SH3 |
| `std.compiler.optimize` | 28 | 29408 | Stack/TIR passes do not define the native pipeline. Keep an explicit initial native optimization policy; port only justified semantic transformations. Trident, SH3 |
| `std.compiler.lower` | 145 | 29714 | Legacy TASM emission/opcode constants. Excluded from the proposed native compiler after dependency removal; foreign target implementation remains Trisha-owned. Do not delete it to make inventory counts look smaller |
| `vm.core.convert` | 3 | 323 | `as_field`/`as_u32` are called; `split` is only declared in this closure. Preserve checked conversion semantics. Trident/nox ABI, SH1 |
| `vm.core.field` | 5 | 461 | Only `field.neg` is called by qualified path; ordinary arithmetic also uses language operators. Existing nox field operations are a seed foundation. Trident/nox |
| `vm.io.mem` | 4 | 491 | `mem.read`/`mem.write` must be replaced by explicit persistent compiler state. Block operations are declarations only. Trident, SH0.2/SH1 |

No native `.tri` NounBuilder, source-package decoder, complete module driver or
standalone compiler entry is present in this closure. They must be added and
inventoried, with their dependencies, before SH4/SH5. Maintain the original
module-to-replacement mapping as files are split or moved. A replacement frontend
still has to process the real compiler source and preserve its language semantics.

## Exact import closure

```text
pipeline  -> lexer, parser, typecheck, codegen, optimize, vm.io.mem
lexer     -> vm.core.convert, vm.core.field, vm.io.mem
parser    -> lexer, vm.core.convert, vm.core.field, vm.io.mem
typecheck -> parser, vm.core.convert, vm.core.field, vm.io.mem
codegen   -> parser, lower, vm.core.convert, vm.core.field, vm.io.mem
optimize  -> lower, vm.core.convert, vm.core.field, vm.io.mem
lower     -> vm.core.convert, vm.core.field, vm.io.mem
```

Names without a prefix above are in `std.compiler`. Intrinsic modules have no
imports. `codegen` and `optimize` import `lower` for opcode definitions; a native
frontend/driver must remove that target coupling explicitly. The generated JSON
retains all imported modules, per-file BLAKE3 source identities, function names,
syntactic call counts, loop bounds and source-line examples for each feature.
BLAKE3 here identifies audit inputs; it does not replace native Hemera identities.

## Feature disposition for the seed/native port

The counts refer to source occurrences, not executed operations. Feature keys
in parentheses match the JSON; nested expressions contribute to multiple
categories. Existing seed support is bounded by the nox implementation, and
does not imply the `.tri` prototype implements the same construct correctly.

| Feature keys / observation | Seed/native disposition |
|---|---|
| `file.module` (10), `item.use` (29), `item.fn` (985), `visibility.pub` (200) | Existing Rust syntax/resolution. Native module loading, symbol identity and reusable calls remain SH3/SH4 |
| `type.Field` (2776), `type.U32` (135), `type.Bool` (18), `return.implicit_unit` (293) | Existing seed types. Preserve canonical ranges and nox Boolean ABI; implement native frontend checking |
| `type.Tuple` (5), `expr.tuple` (1), `binding.tuple` (10) | Existing bounded aggregates/destructuring; native frontend and source-level multi-result semantics need coverage |
| `type.Digest` (2) | Only signatures of unused `mem.read_block`/`write_block`. Not a demonstrated digest workload for the compiler |
| `attribute.intrinsic` (12), `fn.declaration_only` (12) | Three VM modules declare 12 intrinsics; only five distinct qualified intrinsic function paths are called here |
| `binding.name` (1300), `binding.mutable` (135), `binding.inferred_type` (20), `stmt.let` (1310), `stmt.assign` (249), `place.variable` (249) | Rust supports locals and assignments; `.tri` variable/scope bugs require repair. RAM state changes require native persistent-state adaptation |
| `stmt.if` (1027), `stmt.return` (22), `block.tail` (1717), `stmt.expr` (1447) | Existing bounded seed control flow; port/check early returns, effects and implicit tails against independent results |
| `stmt.for` (163), `loop.bounded` (163) | All source loops have explicit bounds. Replace static expansion with bounded runtime execution; see sizes below |
| `expr.call` (5852), `expr.variable` (7297) | Seed calls inline; native code must share function bodies. Includes 638 `mem.read/write` call sites to migrate |
| `expr.binary` (2235), `operator.+` (1162), `operator.*` (187), `operator.==` (843), `operator.<` (37) | Existing scalar nox lowering. Preserve integer/field and Boolean semantics and native frontend type rules |
| `operator./%` (6) | Unsupported by current nox seed. All six sites are in the stack/TASM stages scheduled for native replacement; the existing lexer/parser/typechecker contain none |
| `expr.literal` (4338), `literal.integer` (4256), `literal.bool` (82) | Existing seed literal syntax. Canonicalization/range checks remain required in native frontend |

No syntactic `struct`, array type/construction/index, match statement, generic
parameter/argument, user constant declaration, I/O declaration, assembly or
contract attribute occurs in these source modules. Lexer/parser constants naming
such language features are ordinary Field-returning functions and integer tags.
Those features may still be required by **the new native data representation**;
SH0.2 must inventory additions rather than mistake their current absence for
permission to omit them from the eventual compiler closure.

### Intrinsics and loops

Qualified external call paths actually present are `convert.as_field`,
`convert.as_u32`, `field.neg`, `mem.read` and `mem.write`. The current source
has no stream input/output driver. Pipeline test wrappers add stream I/O and
must not be confused with the compiler module closure.

| Compiler module | Largest declared loop bound |
|---|---:|
| lexer | 8193 |
| parser | 32768 |
| typecheck | 131072 |
| codegen | 131072 |
| optimize | 65536 |
| lower | 200000 |
| pipeline | No loops |

Bounds are finite work-queue guards, not evidence that truncation/exhaustion is
handled correctly. Native SH1/SH4 must enforce explicit exhaustion diagnostics,
and test the actual queue/data capacity independently from the iteration count.
The six divmod sites are `codegen.tri:1023,1028,1943` and
`lower.tri:208,739,872`; the port may remove their packed encodings rather than
introducing divmod solely to preserve foreign-target representations.

## Reproduce and maintain

From a pinned compatible workspace, at the Trident root:

```sh
cargo test --locked --example selfhost_inventory
cargo run --locked --example selfhost_inventory -- --root . --output audit/self-hosting/compiler-subset.json --check
```

To regenerate, omit `--check`, then review this disposition against the JSON
diff. The checker returns failure without modifying a stale receipt. It reads
actual checkout bytes, rejects missing imports/declaration mismatches/parse
errors, and emits stable module ordering and paths independent of checkout
location. AST enum matches are exhaustive, so AST additions require updating
the walker rather than silently skipping a new node kind.

Tests cover comments that resemble code, nested expressions, transitive diamond
imports, relocation, missing/misnamed/malformed modules, source identity changes
and invalid module paths. The original runtime/frontend failure probes remain
in [the starting assessment](../soft3-self-compilation-readiness-2026-09-23.md).

[Delivery validation](sh0-inventory-validation.json): the default-feature release
workspace suite passed 853 tests across 19 harnesses, with zero failures/ignored;
the inventory example passed five further tests. All-target checking passed.
The earlier unoptimized full-suite attempt was stopped before completion and
is not counted as a pass. Neural features, cross-platform release certification
and proving benchmarks were not rerun for this source-inventory delivery.

SH0.1 is complete for this baseline. SH0 stays open until native data operations,
job/artifact encoding, resource policy and owner contracts (SH0.2–SH0.5) close.
