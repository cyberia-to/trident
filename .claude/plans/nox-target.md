# Nox Native Compilation Target for Trident

## Context

Trident нужен прямой путь компиляции в nox — нашу proof-native VM.
TIR (стековый IR) — лишний: AST уже дерево, nox nouns — тоже деревья.
Прямая трансляция AST → Noun formula без промежуточного стекового IR.

```
Source → AST → TypeCheck → NoxCompiler → Noun formula → nox reduce()
```

## Subject Model

Nox evaluation: `reduce(subject, formula, budget) → result`.
Subject — environment (bindings). Formula — code.

Subject растёт по мере let-bindings:
```
Начало функции: [param0 [param1 [param2 0]]]
После let x = e: [x [param0 [param1 [param2 0]]]]
```

Переменная → axis lookup в subject. Scope stack отслеживает
name → depth. Depth → axis: `stack_axis(depth) = 2^(depth+2) - 2`.

## AST → Noun Mapping

### Выражения

| AST Node | Nox Formula |
|----------|-------------|
| `42` (literal) | `[1 42]` (quote) |
| `true` | `[1 0]` (nox true = 0) |
| `false` | `[1 1]` |
| `x` (variable) | `[0 axis]` (axis lookup) |
| `a + b` | `[5 [compile(a) compile(b)]]` |
| `a - b` | `[6 [compile(a) compile(b)]]` |
| `a * b` | `[7 [compile(a) compile(b)]]` |
| `a == b` | `[9 [compile(a) compile(b)]]` |
| `a < b` | `[10 [compile(a) compile(b)]]` |
| `a & b` | `[12 [compile(a) compile(b)]]` |
| `a ^ b` | `[11 [compile(a) compile(b)]]` |
| `invert(a)` | `[8 compile(a)]` |
| `hash(a)` | `[15 compile(a)]` |

### Let-binding

`let x = expr; body` →
```
[2 [[3 [compile(expr) [0 1]]]    // new subject = [expr_result, old_subject]
    [1 compile(body)]]]           // formula for body (against new subject)
```

Раскладка:
1. `[3 [compile(expr) [0 1]]]` — cons(eval expr, current subject) → new subject
2. `[1 compile(body)]` — quote body formula
3. `[2 [new_subj body]]` — compose: evaluate body against new subject

В body переменная x будет на axis 2 (head), старые переменные сдвинутся.

### If/Else

`if cond { then } else { els }` →
```
[4 [compile(cond) [compile(then) compile(else)]]]
```

Nox branch: yes-arm (then) при 0, no-arm (else) при ≠0.
Nox eq возвращает 0 при равенстве — это "true". Совпадает:
- condition=0 (nox-true) → then_body ✓
- condition≠0 (nox-false) → else_body ✓

### Boolean convention

Nox: 0=true, 1=false. Trident literals: `true`→0, `false`→1 в nox.
Компиляция bool literal уже учтена в таблице выше.

Если bool используется в арифметике (редко в provable code),
нужна конверсия. Phase 1: не поддерживаем bool-arithmetic.

### For loop (bounded)

`for i in start..end { body }` → рекурсивный core.

Subject = `[counter [limit rest]]`. Formula:
```
[4 [[9 [[0 2] [0 6]]]           // test: counter == limit?
    [[0 7]                        // yes (0): return rest (done)
     [2 [[3 [[5 [[0 2] [1 1]]]  // no: new_subj = [counter+1, limit, body(rest)]
              [0 1]]]
         [1 SELF]]]]]]           // recurse
```

Phase 1: поддержка через unrolling (если bound известен).
Phase 2: рекурсивный core pattern.

### Function Call

Для Phase 1: инлайн все функции (single-module, no recursion).
Каждый call site раскрывается как body с подставленными аргументами.

Phase 2: function core — subject содержит battery (набор формул),
call = axis lookup + compose.

### Sequences (multiple statements)

`stmt1; stmt2; expr` →

Если stmt1 это `let x = e`:
- cons e на subject, compile остальное в новом scope

Если stmt1 это `assign x = e`:
- edit subject: `[2 [[EDIT_FORMULA] [1 rest_formula]]]`
- EDIT = rebuild subject with new value at x's axis

Если stmt1 это выражение (side-effect only):
- compose: `[2 [compile(stmt1) [1 compile(rest)]]]`

## Files

### Create

**`vm/nox/target.toml`** — target config
```toml
[target]
name = "nox"
display_name = "NOX"
architecture = "tree"
output_extension = ".nox"

[field]
prime = "2^64 - 2^32 + 1"
bits = 64
limbs = 2

[stack]
depth = 0
spill_ram_base = 0

[hash]
function = "Hemera"
digest_width = 8
rate = 8

[extension_field]
degree = 3

[cost]
tables = ["reductions"]

[status]
level = 3
lowering = "NoxCompiler"
lowering_path = "tree"
cost_model = false
tests = false
```

**`src/ir/tree/lower/nox.rs`** (~400 LOC) — AST → Noun compiler
- `struct NoxCompiler` — scope stack, depth tracker
- `fn compile_file(&mut self, file: &ast::File, exports: &ModuleExports) -> Noun`
- `fn compile_fn(&mut self, func: &FnDef) -> Noun`
- `fn compile_block(&mut self, block: &Block) -> Noun`
- `fn compile_expr(&mut self, expr: &Expr) -> Noun`
- `fn compile_stmt(&mut self, stmt: &Stmt, rest: Noun) -> Noun`
- Nox formula constructors: `nox_axis`, `nox_quote`, `nox_compose`,
  `nox_cons`, `nox_branch`, `nox_add`..`nox_hint`
- `fn stack_axis(depth: u32) -> u64`

### Modify

**`src/ir/tree/lower/mod.rs`**
- Add `pub mod nox;`
- Wire `"nox"` in `create_tree_lowering()`

**`src/api/mod.rs`**
- Import `Arch`, `create_tree_lowering`
- Dispatch by `target_config.architecture`:
  - `Arch::Tree` → NoxCompiler path (AST → Noun, skip TIR)
  - `Arch::Stack` → existing TIR → StackLowering path

Key change in `compile_with_options()`:
```rust
match options.target_config.architecture() {
    Arch::Tree => {
        // Direct AST → Noun (no TIR)
        let noun = nox::NoxCompiler::compile(&file, &exports);
        format!("{}", noun)
    }
    _ => {
        // Existing stack path: AST → TIR → TASM
        let ir = TIRBuilder::new(...).build_file(&file);
        let ir = optimize_tir(ir);
        create_stack_lowering(&name).lower(&ir).join("\n")
    }
}
```

## Verification

1. `cargo check` — zero warnings
2. `cargo test` — all existing tests pass
3. Unit tests in nox.rs:
   - Literal: `compile(42)` → `[1 42]`
   - Add: `compile(3 + 5)` → `[5 [[1 3] [1 5]]]`
   - Variable: `compile(let x = 3; x + 1)` → compose+cons+add formula
   - IfElse: both branches
4. `trident build --target nox simple.tri` → outputs noun formula
5. Feed to `nox::reduce()`, verify result matches Triton output

## Phase 1 Scope (~1 session)

Supported:
- Literals (integer, bool)
- Variables (let bindings, function params)
- Arithmetic: add, sub, mul, invert, neg
- Comparison: eq, lt
- Bitwise: and, xor
- Hash
- IfElse, IfOnly
- Single function (entry point)

Not supported (Phase 2+):
- For loops
- Function calls (multi-function)
- Mutable assignment
- Structs, arrays, tuples
- Memory ops, I/O
- Sponge, Merkle, extension field
- Inline assembly
