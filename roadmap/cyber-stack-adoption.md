# Cyber Stack Adoption Plan

## Context

The cyber protocol defines a nine-crate stack. Trident is the language
compiler. This plan integrates the canonical cyber crates, adds nox as
a new VM target and cyber as a new OS target, and introduces a direct
AST→Noun compilation path that bypasses TIR for tree targets.

Nock stays. Nox is an additional tree machine.

## The Stack

```
nebu (field) → hemera (hash) → nox (VM) → zheng (proofs) → bbg (state)
                                                            ├→ tru
                                                            └→ plumb
hemera → mudra (crypto)
hemera → radio (transport)
```

## Maturity

| Crate | Version | Status | Ready? |
|-------|---------|--------|--------|
| nebu | 0.1.0 | ~3.3k LOC, field + NTT + extensions + GPU | YES |
| hemera | 0.2.0 | complete, test vectors, unaudited | YES |
| nox | 0.1.0 | spec complete, source skeleton | SPEC ONLY |
| zheng | 0.1.0 | spec complete, placeholder impl | SPEC ONLY |
| bbg | — | spec complete, placeholder impl | SPEC ONLY |

---

## First Principles: Why Trident + nox Are a Natural Pair

Trident and nox share deep structural properties. The synergy is not
accidental — both are designed around the same field, the same
constraints, and the same computational model.

### Shared foundations

| Property | Trident | nox |
|----------|---------|-----|
| Arithmetic | Goldilocks field-native | Goldilocks field-native |
| Execution | Bounded, no recursion | Bounded by focus budget |
| Types | Field, Bool, U32, Digest | field (F_p), word (Z/2^32), hash (4×F_p) |
| Data | Structs = flat field elements | Nouns = binary trees of field elements |
| Control | if/else, for, match | branch (pattern 4) |
| Hash | hemera (content addressing) | hemera (pattern 15) |
| Determinism | Required for proofs | Confluence theorem |
| Proofs | STARK target | Execution trace IS zheng witness |

### Why TIR is the wrong path for nox

Current pipeline:
```
Source (tree) → AST (tree) → TypeCheck → TIR (flat stack) → TreeLowering → Noun (tree)
```

This is tree → flat → tree. Information destruction followed by
reconstruction. Specific losses:

1. **Parallelism destroyed.** `let a = x + y; let b = w * z` — these
   are independent in the AST. TIR sequentializes them into Push/Dup/Add
   /Push/Dup/Mul. In nox, `cons(add(x,y), mul(w,z))` is parallel by
   confluence. Direct lowering preserves independence.

2. **Stack artifacts injected.** TIR adds Dup/Swap/Pop for stack
   management. nox has no stack — the subject tree has O(1) axis
   addressing. Stack ops become useless cons cells in the noun.

3. **Types erased.** TIR is untyped — all 54 ops operate on anonymous
   stack slots. nox needs type tags (field=0x00, word=0x01, hash=0x02)
   for domain separation in hemera hashing. The typed AST has this;
   TIR throws it away.

4. **Spill/reload noise.** TIR's StackManager exists because stack
   machines have finite depth (16 on Triton). nox subject trees can
   be arbitrarily deep. No spill needed, but TIR generates spill ops
   that TreeLowering must undo.

5. **Memoization shape lost.** nox's cybergraph caches results by
   `H(formula, object)`. Source-shaped nouns produce sub-trees that
   match across similar programs → more cache hits. Stack-shaped nouns
   produce unique trees per stack layout → fewer hits.

### The direct path

```
Source → AST → TypeCheck → NounBuilder → Noun
```

The typed AST carries everything NounBuilder needs:
- Tree structure (from source)
- Types and widths (from TypeChecker's ModuleExports)
- Variable bindings and scopes (from AST Let nodes)
- Control flow (from AST If/For/Match)
- Monomorphized function instances (from TypeChecker)

What NounBuilder replaces:
- TIRBuilder (AST → TIR) — skip entirely
- TreeLowering (TIR → Noun) — skip entirely
- StackManager — replaced by SubjectManager (tree axes, not stack depth)

### Structural correspondence

| Trident source | nox pattern | Direct mapping |
|---------------|-------------|----------------|
| `let x = expr` | cons (3) | Extend subject: `[3 [expr] [rest]]` |
| `x` (variable) | axis (0) | Navigate subject: `[0 axis_of_x]` |
| `a + b` | add (5) | `[5 [0 axis_a] [0 axis_b]]` |
| `a - b` | sub (6) | `[6 [0 axis_a] [0 axis_b]]` |
| `a * b` | mul (7) | `[7 [0 axis_a] [0 axis_b]]` |
| `1 / a` | inv (8) | `[8 [0 axis_a]]` |
| `a == b` | eq (9) | `[9 [0 axis_a] [0 axis_b]]` |
| `a < b` | lt (10) | `[10 [0 axis_a] [0 axis_b]]` |
| `a ^ b` | xor (11) | `[11 [0 axis_a] [0 axis_b]]` |
| `a & b` | and (12) | `[12 [0 axis_a] [0 axis_b]]` |
| `hash(x)` | hash (15) | `[15 [0 axis_x]]` |
| `if c { a } else { b }` | branch (4) | `[4 [c_noun] [a_noun] [b_noun]]` |
| `f(x)` | compose (2) | `[2 [x_subject] [f_formula]]` |
| `42` | quote (1) | `[1 42]` |
| `secret_read()` | hint (16) | `[16 [constraint] [formula]]` |
| `(a, b)` | cons (3) | `[3 [a_noun] [b_noun]]` |
| `s.field` | axis (0) | `[0 axis_of_field]` |

Every source construct maps to exactly one nox pattern. No intermediate
representation needed.

### Gains from direct lowering

| Metric | Through TIR | Direct AST→Noun | Why |
|--------|------------|-----------------|-----|
| Compilation passes | 2 (AST→TIR→Noun) | 1 (AST→Noun) | Skip TIR entirely |
| Noun size | larger | ~20-30% smaller | No Dup/Swap/spill artifacts |
| Parallelism | lost | preserved | cons/compose sub-exprs stay independent |
| Type info | erased | preserved | Domain separation in hemera hashing |
| Memoization | poor cache shape | source-aligned shape | Similar programs share sub-trees |
| Focus prediction | approximate | exact | Pattern costs known at compile time |
| Compiler complexity | 2 modules | 1 module | NounBuilder replaces TIRBuilder + TreeLowering |
| Spill/reload | yes (finite stack) | none | Subject tree has no depth limit |

### nox-specific optimizations (tree-native)

These optimizations are impossible through TIR because TIR destroys
the tree structure they operate on.

1. **Subject sharing.** Multiple functions can share subject sub-trees.
   If `f(a,b)` and `g(a,c)` both need `a`, the subject tree places
   `a` at a shared axis. Through TIR, each function gets its own
   stack layout.

2. **Memoization-aware layout.** Place frequently-hashed sub-expressions
   at known axes so `H(formula, object)` cache keys collide across
   similar programs. The compiler can optimize for cache hits.

3. **Cons tree compaction.** Flatten right-nested cons chains when
   possible. `[3 [3 a b] c]` → `[3 a [3 b c]]` when semantically
   equivalent. Shallower trees = cheaper axis lookups.

4. **Dead axis elimination.** If a variable is bound but never
   accessed by axis, don't cons it into the subject. TIR has dead
   code elimination but it operates on stack ops, not tree shape.

5. **Focus budgeting.** Since every pattern has a known cost (add=1,
   inv=64, hash=200), the compiler can compute exact focus cost of
   any noun tree at compile time. Report this as `trident cost`.

6. **Parallel marking.** Mark cons/compose nodes whose sub-expressions
   are independent (no shared axis reads). A parallel nox executor
   can use these marks. TIR destroys this information.

---

## Phase 1: nebu + hemera (NOW — 2.5 sessions)

Both crates are implemented. Mechanical replacement.

### 1a. Dependencies

```toml
# Cargo.toml — add:
nebu = { path = "../nebu/rs" }
hemera = { package = "cyber-hemera", path = "../hemera/rs" }
# Remove: blake3 = "1"
```

### 1b. Bridge Goldilocks to nebu

`src/field/goldilocks.rs` becomes thin wrapper: delegate ops to
`nebu::Goldilocks`, implement trident's `PrimeField` trait.
BabyBear and Mersenne31 keep their own implementations.

### 1c. Replace hashing with hemera

Delete `src/field/poseidon2.rs` (295 LOC) and
`src/package/poseidon2.rs` (340 LOC).

ContentHash: `[u8; 32]` → `[u8; 64]`. hash_version: 1 → 2.

All call sites (~10 files): `poseidon2::hash_bytes()` → `hemera::hash()`.

### 1d. Tests

- All hash test vectors change (new expected values)
- Snapshot tests: `cargo insta review`
- Benchmark references: rewrite using hemera
- Integration: `trident hash` outputs 128 hex chars
- Trisha rebuild + verify (Tip5 on-chain unchanged)

### 1e. Documentation

6 files: language.md, vm.md, content-addressing.md, src/README.md,
CLAUDE.md, README.md.

### Estimate

| Task | Pomodoros |
|------|----------|
| Cargo.toml + goldilocks bridge | 2 |
| Delete poseidon2 + replace call sites | 2 |
| ContentHash 32→64 ripple (store, manifests, CLI, artifacts) | 3 |
| Fix all tests + update expected values | 3 |
| Update benchmarks/references | 2 |
| Docs (6 files) | 1 |
| Trisha rebuild + verify | 1 |
| Final verification (check, test, bench, grep) | 1 |
| **Total** | **15 pomodoros = 2.5 sessions** |

---

## Phase 2: vm/nox/ target + direct AST→Noun path (NOW — 5 sessions)

This is the central architectural change. Add nox as a new VM target
with a direct compilation path that bypasses TIR.

### 2a. Create vm/nox/ target profile

```
vm/nox/
  target.toml
  README.md
```

```toml
[target]
name = "nox"
display_name = "NOX"
architecture = "tree"
output_extension = ".nox"

[field]
prime = "2^64 - 2^32 + 1"
bits = 64

[hash]
function = "Hemera"
digest_width = 8
rate = 8

[cost]
tables = ["focus"]

[status]
level = 0
lowering = "NounLowering"
lowering_path = "noun"
cost_model = true
tests = false
notes = "Direct AST→Noun. Spec-aligned, awaiting nox crate."
```

### 2b. Register in target system

- `src/config/target/mod.rs` — add nox to VM registry
- `reference/vm.md` — add NOX row
- `reference/targets.md` — add nox entry
- `vm/README.md` — add to table

### 2c. Create NounBuilder (the core work)

New module: `src/ir/noun/` — direct AST→Noun lowering for tree targets.

```
src/ir/noun/
  mod.rs           # NounBuilder, public API
  subject.rs       # SubjectManager (variable → axis mapping)
  lower.rs         # AST node → nox pattern conversion
  focus.rs         # Focus cost calculation
  serialize.rs     # Noun → .nox binary format
  tests.rs         # Unit + property tests
```

**SubjectManager** replaces StackManager:

```rust
struct SubjectManager {
    /// Maps variable name to axis number in subject tree.
    bindings: Vec<(String, u64, Ty)>,
    /// Current subject depth (for axis calculation).
    depth: u32,
}

impl SubjectManager {
    /// Bind a new variable: extends subject with cons.
    fn bind(&mut self, name: &str, ty: &Ty) -> u64 { /* axis */ }
    /// Look up variable axis.
    fn axis_of(&self, name: &str) -> u64 { /* axis number */ }
    /// Enter scope (save state).
    fn push_scope(&mut self) { }
    /// Leave scope (restore).
    fn pop_scope(&mut self) { }
}
```

**NounBuilder** walks typed AST:

```rust
struct NounBuilder {
    subject: SubjectManager,
    exports: ModuleExports,   // from TypeChecker
    focus_cost: u64,          // accumulated focus estimate
}

impl NounBuilder {
    fn build_expr(&mut self, expr: &Expr) -> Noun { ... }
    fn build_stmt(&mut self, stmt: &Stmt) -> Noun { ... }
    fn build_fn(&mut self, func: &FnDef) -> Noun { ... }
    fn build_program(&mut self, file: &File) -> Noun { ... }
}
```

**Key lowering rules:**

```
build_expr(Literal(v))     → Noun::cell(1, Noun::atom(v))      // quote
build_expr(Var(name))      → Noun::cell(0, axis_of(name))      // axis
build_expr(BinOp(Add,a,b)) → Noun::cell(5, [build(a), build(b)]) // add
build_expr(BinOp(Mul,a,b)) → Noun::cell(7, [build(a), build(b)]) // mul
build_expr(Call(f, args))  → Noun::cell(2, [args_subject, f_formula]) // compose

build_stmt(Let(x, init))  → Noun::cell(3, [build(init), ...]) // cons (extend subject)
build_stmt(If(c, t, e))   → Noun::cell(4, [build(c), build(t), build(e)]) // branch
```

### 2d. Nox-specific optimizations

| Optimization | What | Estimated gain |
|-------------|------|----------------|
| Dead axis elimination | Don't cons unused variables | 5-15% smaller nouns |
| Subject sharing | Shared sub-trees for common args | 10-20% smaller for multi-fn |
| Cons compaction | Flatten right-nested cons | ~5% smaller nouns |
| Parallel marking | Tag independent sub-exprs | Future: parallel executor |
| Focus precomputation | Exact focus cost at compile time | 100% accurate cost prediction |

### 2e. Serialization (.nox format)

Binary format for nox nouns. Atoms are 8-byte field elements (not
arbitrary-width naturals like Nock .jam). Three sizes: 8 (atom),
32 (hash), 64 (cell = two hemera hashes).

### 2f. Cost model (focus)

```
src/cost/nox.rs — focus cost tables

Pattern costs (canonical):
  structural (0-4): 1
  field arith (5-7, 9): 1
  inv (8): 64
  lt (10): 1 (exec), ~64 (constraints)
  bitwise (11-14): 1 (exec), ~32 (constraints)
  hash (15): 200
  hint (16): 1 + sub-costs
```

Focus cost is calculable at compile time from the noun tree.
`trident cost --target nox program.tri` → exact focus budget.

### 2g. Pipeline integration

```rust
// src/api/mod.rs — add nox path
pub fn compile_to_noun(file: &File, exports: &ModuleExports) -> Noun {
    let mut builder = NounBuilder::new(exports);
    builder.build_program(file)
}
```

The existing TIR path stays for stack/register/GPU targets.
The noun path is an alternative for tree targets (nox, and
potentially Nock in the future if we want better Nock output).

### 2h. Tests

| Test category | Count | What |
|---------------|-------|------|
| Target loading | 2 | target.toml parses, registers |
| SubjectManager | 8 | bind, lookup, scope push/pop, axis math |
| NounBuilder per-expr | 12 | literal, var, binop (+,-,*,/,==,<), call, field access |
| NounBuilder per-stmt | 8 | let, if/else, for, match, return, assign |
| NounBuilder functions | 4 | single fn, multi fn, generic fn, recursive-like |
| Focus calculation | 6 | each pattern cost, compound expressions |
| Serialization roundtrip | 4 | atom, hash, cell, nested |
| End-to-end | 4 | `trident build --target nox` on test programs |
| Snapshot | 6 | golden noun output for reference programs |
| **Total** | **~54** | |

### 2i. Documentation

| File | Change |
|------|--------|
| `vm/nox/README.md` | Full target profile |
| `vm/README.md` | Add NOX to registry |
| `reference/vm.md` | Add NOX row with all parameters |
| `reference/targets.md` | Add nox integration level |
| `reference/ir.md` | Add noun lowering section (AST→Noun path) |
| `src/README.md` | Add src/ir/noun/ module description |
| `CLAUDE.md` | Add noun path to pipeline contract |

### Estimate

| Task | Pomodoros |
|------|----------|
| vm/nox/ target profile + registration | 2 |
| SubjectManager (variable → axis) | 3 |
| NounBuilder core (expr + stmt lowering) | 6 |
| Function handling (call, multi-fn, generics) | 3 |
| Nox optimizations (dead axis, subject sharing) | 3 |
| Focus cost model | 2 |
| Serialization (.nox format) | 2 |
| Tests (~54 tests) | 5 |
| Pipeline integration (api/mod.rs) | 1 |
| Documentation (7 files) | 2 |
| End-to-end: `trident build --target nox` | 1 |
| **Total** | **30 pomodoros = 5 sessions** |

---

## Phase 3: os/cyber/ target (NOW — 1 session)

New OS target alongside Nockchain, Neptune, etc.

### 3a. Create OS profile

```
os/cyber/
  target.toml
  README.md
  types.tri       # Particle, Neuron, Cyberlink, Token, Focus
  cyberlink.tri   # cyberlink creation/validation
```

```toml
[os]
name = "cyber"
display_name = "Cyber"
vm = "nox"

[runtime]
binding_prefix = "os.cyber"
account_model = "cybergraph"
storage_model = "bbg"
transaction_model = "proof-based"

[status]
level = 0
ext_modules = 0
tests = false
notes = "Types from bbg spec. Awaiting runtime."
```

### 3b. Types from bbg spec

```trident
// os/cyber/types.tri
module os.cyber.types

pub struct Particle {
    hash: Digest,
    energy: Field,
    focus: Field,
}

pub struct Neuron {
    focus: Field,
    karma: Field,
    stake: Field,
}

pub struct Cyberlink {
    from: Digest,
    to: Digest,
    neuron: Digest,
    weight: Field,
}
```

### 3c. Registration + tests + docs

- `src/config/target/os.rs` — add cyber
- `reference/os.md` — add entry
- `os/README.md` — add to table
- Target loading test, type compilation test, resolved target test

### Estimate

| Task | Pomodoros |
|------|----------|
| target.toml + README | 1 |
| Type definitions (.tri files) | 2 |
| Register in OS system + tests | 2 |
| Docs (3 files) | 1 |
| **Total** | **6 pomodoros = 1 session** |

---

## Phase 4: Align runtime with zheng API (NOW — 1 session)

### 4a. Update traits

- `Prover` gains `focus: u64` parameter
- `ProveResult` carries structured `Proof`
- `Verifier` takes `Statement` (program hash + I/O + focus bound)
- New: `Folder` trait for recursive composition
- `ProgramBundle.program_digest` → 64 bytes (hemera)
- New: `focus_bound: u64` field

### 4b. Tests + docs

- Mock implementations for trait compilation tests
- ProgramBundle serialization roundtrip
- Update runtime/ docs

### Estimate

| Task | Pomodoros |
|------|----------|
| Update trait signatures | 2 |
| Update ProgramBundle | 1 |
| Mock tests | 2 |
| Docs | 1 |
| **Total** | **6 pomodoros = 1 session** |

---

## Phase 5: Full pipeline (BLOCKED on nox + zheng)

When nox can execute and zheng can prove:

```
.tri → trident build --target nox → .nox noun
    → nox execute → trace → zheng prove → proof
```

New warrior binary for cyber target.

### Estimate: 4-5 sessions when deps are ready

---

## Total Estimate

| Phase | Status | Pomodoros | Sessions |
|-------|--------|----------|----------|
| 1. nebu + hemera | NOW | 15 | 2.5 |
| 2. vm/nox/ + AST→Noun | NOW | 30 | 5 |
| 3. os/cyber/ | NOW | 6 | 1 |
| 4. Runtime/zheng | NOW | 6 | 1 |
| 5. Full pipeline | BLOCKED | 24-30 | 4-5 |
| **Total** | | **81-87** | **13.5-14.5** |

Phases 1-4 (9.5 sessions) can run without waiting for nox/zheng.
Phase 5 blocked until cyber stack crates have working code.

## Risk

1. **nebu v0.1.0 API churn** — thin wrapper insulates.
2. **ContentHash 64-byte ripple** — biggest Phase 1 change.
3. **NounBuilder complexity** — the SubjectManager (variable→axis)
   is the hard part. Stack machines have linear layout; tree machines
   have exponential branching. Axis calculation must be correct.
4. **nox spec vs implementation** — spec says "frozen" but implementation
   may reveal issues. NounBuilder should be modular.
5. **Multi-target** — TIR stays for 20+ stack/register/GPU targets.
   Noun path is tree-target only. Two paths = more code, but each
   is simpler than a forced universal path.
6. **bbg types draft** — os/cyber/ types will iterate.
7. **Triton VM unaffected** — hemera replaces only off-chain content
   addressing. trisha stays. Tip5 stays on-chain.
