# Plan: Self-Hosted Typechecker (typecheck.tri) + Prove End-to-End

Status: historical prototype plan; superseded 2026-09-23.
Follow [self-hosting on soft3](../../reference/self-hosting.md) and its
[progress ledger](../../audit/self-hosting-progress.md). The RAM/Triton design
and completion language below describe the old plan, not current readiness.

## Context

Lexer and parser are self-hosted (`std/compiler/lexer.tri` 824 LOC, `std/compiler/parser.tri` 2,723 LOC). Next self-hosting piece: typechecker. This is needed regardless of backend (TASM or CORE). Secondary goal: prove the existing lexer/parser benches end-to-end via trisha.

## Architecture

The Rust typechecker is ~2,843 LOC across 8 files. The type system is simple:
- 9 types: Field, XField, Bool, U32, Digest, Array, Tuple, Struct, Unit
- No polymorphism, no subtyping, no inference (almost)
- Two-pass: (1) register all signatures/structs/consts, (2) check function bodies
- Scopes: push/pop stack of variable maps
- ~30 builtins registered at startup

Same pattern as parser: single bounded loop, explicit state machine, data in RAM.

## Data Representation

All data lives in RAM (same as lexer/parser). Key structures:

### Type encoding (1 word tag + up to 3 words payload)
```
TY_FIELD=1, TY_XFIELD=2, TY_BOOL=3, TY_U32=4, TY_DIGEST=5
TY_ARRAY=6, TY_TUPLE=7, TY_STRUCT=8, TY_UNIT=9
```
- Scalars: just tag (1 word)
- XField/Digest: tag + width (2 words)
- Array: tag + inner_type_index + length (3 words) — inner_type_index points to type pool
- Tuple: tag + count + start_index (3 words) — sub-types stored in type pool
- Struct: tag + struct_table_index (2 words)

### Type pool
Fixed-size array in RAM. Each resolved type gets an index. Avoids recursive structures.

### Symbol tables (flat arrays in RAM)
- **Functions**: (name_hash, param_count, param_types_start, return_type_index) per entry
- **Variables** (scope stack): (name_hash, type_index, is_mutable, scope_depth)
- **Structs**: (name_hash, field_count, fields_start) — fields = (name_hash, type_index, is_pub)
- **Constants**: (name_hash, type_index, value)

Name matching: hash-based (no string comparison — only token position hashes from parser AST nodes).

## Memory Layout

```
state_base     +0..+31     Typechecker state (32 words)
type_pool      +0..+N*4    Pool of resolved types
fn_table       +0..+M*4    Function signature table
var_stack      +0..+V*4    Variable scope stack
struct_table   +0..+S*8    Struct definitions
const_table    +0..+C*3    Constants
err_base       +0..+E*3    Errors (severity, span_start, span_end)
diag_count     count       Number of diagnostics
```

## Implementation Steps

### Step 1: Skeleton + type representation (~400 LOC)
**File**: `std/compiler/typecheck.tri`

- Module header, imports (vm.core.field, vm.core.convert, vm.io.mem, std.compiler.parser)
- Type tag constants (TY_FIELD through TY_UNIT)
- Type pool: `alloc_type()`, `store_type()`, `load_type_tag()`, `type_eq()`
- Type width computation: `type_width(type_index) -> Field`
- Memory layout constants and state accessors

### Step 2: Symbol tables (~500 LOC)
- Function table: `register_fn()`, `lookup_fn()`, `fn_param_type()`, `fn_return_type()`
- Variable scope: `push_scope()`, `pop_scope()`, `define_var()`, `lookup_var()`
- Struct table: `register_struct()`, `lookup_struct()`, `struct_field_type()`
- Constant table: `register_const()`, `lookup_const()`
- Name hashing: use token position or AST node field as identity key

### Step 3: Builtins registration (~300 LOC)
- `register_builtins()` — register ~30 builtin functions
- Parameterized by digest_width, hash_rate, xfield_width (passed in state)
- Groups: I/O, assertions, field ops, u32 ops, hash, merkle, RAM, conversion, xfield

### Step 4: Type resolution (~200 LOC)
- `resolve_type(ast_node_index) -> type_index` — walk AST type nodes → type pool entries
- Handle: named types (Field/Bool/U32/etc.), array types, tuple types, struct references
- `resolve_array_size()` — constant expression evaluation

### Step 5: Expression checking (~600 LOC)
- `check_expr(ast_node_index) -> type_index`
- Literal → Field/Bool/U32
- Var → scope lookup
- BinOp → operand checking + result type
- Call → function lookup, arg count/type checking, return type
- FieldAccess → struct field lookup
- Index → array/tuple element type
- StructInit → field matching
- ArrayInit / Tuple → element checking

### Step 6: Statement checking (~500 LOC)
- `check_stmt(ast_node_index)`
- Let (name + tuple destructure), Assign, If, For, Return
- Match (with exhaustiveness: wildcard / bool coverage)
- Assert/AssertEq, Asm (skip body), Reveal/Seal
- Expression statements

### Step 7: Top-level driver (~400 LOC)
- `check_file()` — main entry point, two-pass
- Pass 1: iterate AST items, register structs/fns/consts/events
- Pass 2: iterate again, check function bodies
- Recursion detection (call graph DFS — iterative, same as Rust impl)
- Unused import detection
- Error collection and reporting

### Step 8: Bench harness (~100 LOC)
**File**: `benches/harnesses/std/compiler/typecheck.tri`
- Program that loads a known AST (from parser output), runs typecheck, asserts no errors
- Reference implementation in Rust for ground truth

## Estimated Size

~3,000 LOC for typecheck.tri (comparable to parser.tri at 2,723 LOC).

## Prove End-to-End (Secondary)

After typecheck.tri, prove existing benches via trisha:

1. `trident build benches/harnesses/std/compiler/lexer.tri` → get .tasm
2. `trisha run lexer_bench` → execute, verify correctness
3. `trisha prove lexer_bench` → generate STARK proof
4. `trisha verify lexer_bench` → verify proof
5. Repeat for parser_bench and pipeline_bench
6. Record cycle counts and proving times
