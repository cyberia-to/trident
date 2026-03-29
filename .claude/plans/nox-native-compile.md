# Nox Native Compilation: WASM + ARM64

## Architecture

Nox formula (noun tree) → native code. No Cranelift, no LLVM.
18 patterns × ~10-20 instructions each = hand-emitted machine code.

```
formula.nox → nox compile --target wasm → formula.wasm
formula.nox → nox compile --target arm64 → JIT execute
```

Code lives in `nox/rs/compile/`. Part of nox lib (`no_std + alloc`).

## Memory Model

**Phase 1: Atom-only** — formulas that produce atoms (field elements).
Subject = cons-list of atoms → mapped to function parameters.
Result = single atom → single return value. No heap allocation.

**Phase 2: Cell support** — cons, compose, dynamic subjects.
Requires linear memory (WASM) or heap (ARM64) for cell allocation.

## Pattern → Native Mapping (Phase 1)

| Pattern | WASM | ARM64 |
|---------|------|-------|
| 0 axis | local.get N | ldr xN, [subject, #offset] |
| 1 quote | i64.const V | mov xN, #V |
| 4 branch | if/else | cbz/cbnz + branch |
| 5 add | i64.add + reduce | add + reduce |
| 6 sub | i64.sub + reduce | sub + reduce |
| 7 mul | call $goldilocks_mul | call goldilocks_mul |
| 8 inv | call $goldilocks_inv | call goldilocks_inv |
| 9 eq | i64.eq | cmp + cset |
| 10 lt | i64.lt_u + adjust | cmp + cset + adjust |
| 11 xor | i64.xor | eor |
| 12 and | i64.and | and |
| 13 not | i64.xor 0xFFFFFFFF | mvn + and mask |
| 14 shl | i64.shl | lsl |

Patterns 2 (compose), 3 (cons), 15 (hash), 16 (call), 17 (look) → Phase 2.

## Goldilocks Reduction (inline)

```
p = 2^64 - 2^32 + 1

ADD: sum = a + b (wrapping)
     if overflow: sum += 0xFFFFFFFF  (2^64 ≡ 2^32-1 mod p)
     if sum >= p: sum -= p

SUB: if a >= b: a - b
     else: p - b + a

MUL: u128 = a * b
     reduce via hi*(2^32-1) + lo, repeat if needed
```

## Axis → Parameter Mapping

Subject = `[argN [argN-1 [... [arg1 0]]]]`

| Axis | Position | Param index |
|------|----------|-------------|
| 2 | head | 0 (outermost) |
| 6 | head.tail | 1 |
| 14 | head.tail.tail | 2 |
| 30 | head.tail.tail.tail | 3 |

Formula: `param_index = bits(axis) - 2`, where axis = `2*(2^n) - 2`.
General: axis bits from MSB: 1=head 0=go deeper. Last bit: 0=head 1=tail.

## Module Structure

```
nox/rs/compile/
  mod.rs      — CompileNox trait, dispatch, NoxIR (optional)
  wasm.rs     — WASM binary emitter (~300 LOC)
  arm64.rs    — ARM64 machine code emitter (~300 LOC)
```

CLI addition to `nox/cli/main.rs`:
```
nox compile --target wasm formula.nox -o formula.wasm
nox compile --target arm64 formula.nox    # JIT + execute
```

## Implementation Order

1. `compile/mod.rs` — formula tree walker, axis→param resolver
2. `compile/wasm.rs` — emit valid WASM module binary
3. `compile/arm64.rs` — emit ARM64, mmap+mprotect, call as fn ptr
4. CLI integration — `nox compile` subcommand
5. Tests — round-trip: interpret vs native, same result

## Verification

For every formula: `nox -e formula` (interpreter) must produce
the same result as `nox compile --target wasm formula | wasmtime`
and `nox compile --target arm64 formula` (JIT). If they disagree,
native compilation has a bug.
