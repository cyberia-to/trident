---
status: draft
date: 2026-03-26
---
# five algebras: type-driven regime dispatch for nox

**Related proposals:** [[dependent-types]], [[refinement-types]], [[noun-types]]
**Reference:** [stdlib.md](../reference/stdlib.md), [language.md §2 — Types](../reference/language.md)

## problem

Trident v0.5 has four primitive types: `Field`, `Bool`, `U32`, `Digest` (+ `XField` for extensions). all are Goldilocks scalars. this covers ONE of five execution algebras.

the [[cyber]] stack requires five algebras (nebu, kuro, jali, trop, genies) with eight nox instantiations. four algebras have no primitive types in Trident, no std library modules, and no compilation support. programs that need binary computation, ring arithmetic, tropical optimization, or isogeny privacy cannot be expressed natively.

## proposal

add four new primitive type families to Trident. each family maps to one algebra. the type system IS the dispatch mechanism — types determine nox regime, regime determines [[lens]], zero annotations needed.

### two levels of types

**primitive types** change proving execution: they determine regime, lens, constraints. adding or removing a primitive type changes how the zheng proof works.

**semantic types** (structs) prevent programming errors within a regime. they don't change proving. a `Distribution` and a `Matrix` are both `[Field; N]` at the proof level — the struct prevents accidental confusion, not algebraic errors.

test: "if I replace this type with Field, does the proof change?" yes → primitive. no → struct.

### new primitive types

```
// kuro algebra (F₂ tower)
Bit             F₂¹²⁸ packed tower element (128 bits per machine word, SIMD native)
Nibble          F₂⁴ (lookup tables, activation functions)
Byte            F₂⁸ (symmetric crypto, byte-level operations)
F2<N>           F₂ⁿ generic fallback for non-standard tower widths

// jali algebra (R_q)
Lattice         R_q = F_p[x]/(x^n+1) ring element, degree n compile-time const
Eval            NTT-domain representation of Lattice (lazy conversion)

// trop algebra (min,+)
Cost            tropical (min, +): Cost + Cost = min(a,b), Cost * Cost = a + b
Gain            tropical (max, +): Gain + Gain = max(a,b), Gain * Gain = a + b

// genies algebra (F_q)
Iso             supersingular curve (Montgomery form over F_q)
Shade           F_q field element (8 × 64-bit limbs, 512 bits)
```

16 named primitives + 1 generic across 5 algebras:

| algebra | primitives | count | capability |
|---------|-----------|-------|-----------|
| nebu | Field, bool, u32, Digest, Fp2, Fp3, Fp4 | 7 | truth, depth |
| kuro | Bit, Nibble, Byte (+ F2<N>) | 3+generic | speed |
| jali | Lattice, Eval | 2 | veil |
| trop | Cost, Gain | 2 | choice |
| genies | Iso, Shade | 2 | shadow |

### trop: Cost vs Gain

two tropical semirings are common enough to warrant separate types:

```trident
// Cost: shortest/cheapest — add = min
let route: Cost = opt::shortest_path(&distances, source);

// Gain: longest/most profitable — add = max
let critical: Gain = opt::critical_path(&durations, start);
```

Cost + Cost = min(a, b). Gain + Gain = max(a, b). the patterns differ:
```
Cost add: branch(lt(a, b), a, b)     // min: take smaller
Gain add: branch(lt(a, b), b, a)     // max: take larger
```

if a function expects Cost and receives Gain, the optimization is WRONG (minimizes instead of maximizes). type safety catches this at compile time.

both types use the same trop regime and Tropical lens. the difference is pattern emission only.

### jali: 2 types for FHE + signals

Lattice and Eval are sufficient. everything else composes:

```
FHE ciphertext  = struct { a: Lattice, b: Lattice }     → std.wav
FHE secret key  = struct { s: Lattice }                  → std.wav
FHE public key  = struct { parts: [Lattice; K] }         → std.wav
Signal          = Lattice (time domain)                  → std.wav
Spectrum        = Eval (frequency domain)                → std.wav
Convolution     = Eval * Eval                            → Lattice * Lattice
Filter          = Eval (frequency-domain coefficients)   → std.wav
```

like Field: one type, but Matrix = [Field; N*M]. Lattice: one type, but Ciphertext = struct { Lattice, Lattice }.

### genies: primitives vs protocol types

only Iso and Shade are algebraic primitives (they change proving — F_q regime). protocol types are structs in std.sec:

```trident
// std.sec — protocol types (structs over Iso, Shade, Digest)
pub struct Walk { exponents: [u32; 74] }       // secret isogeny path (nebu regime, enters via hint)
pub struct Phantom { address: Digest }          // stealth address (derived from group action)
pub struct Mask { responses: [Shade], c: Digest } // ring signature
pub struct Blind { curve: Iso, s: Shade }       // blind signature
pub struct Fate { output: Digest, proof: Iso }  // VRF output + proof
pub struct Pact { secret: Digest }              // agreed shared secret
```

Walk is a struct, not a primitive: exponents are small integers ([u32; 74] in nebu regime), not F_q elements. group_action jet takes Walk + Iso → Iso. the cross-regime boundary (nebu → genies) happens INSIDE the jet. Walk enters via hint (Layer 2) because it is secret.

### type → regime mapping

the compiler infers regime from operand types. no `#[algebra(...)]` annotation.

```
expression          type of operands    regime      lens
──────────          ────────────────    ──────      ────
a + b               Field               nebu        Brakedown
a + b               Fp2/Fp3/Fp4         nebu²/³/⁴   Brakedown (wider)
a ^ b               Bit                 kuro        Binius
a * b               Lattice             jali        Ring-aware
a + b               Cost                trop        Tropical (min)
a + b               Gain                trop        Tropical (max)
curve_op(iso, shade) Iso, Shade         genies      Isogeny
```

### cross-type boundaries

when operand types change between sub-expressions, the compiler inserts hemera commitments at the boundary. the programmer never sees this.

```trident
fn mixed(x: Field, bits: Bit, ring: Lattice) -> Field {
    let q = bt::quantize(x);          // Field → Bit: boundary inserted
    let r = bt::xor(bits, q);         // Bit ops: kuro regime
    let f = bt::dequantize(r);        // Bit → Field: boundary inserted
    let w = wav::ntt_multiply(ring, ring); // Lattice: jali regime
    f + wav::extract(w)               // back to Field: boundary inserted
}
// compiler inserts 3 hemera commitments at type transitions
// cost: 3 × ~766 F_p constraints
// programmer sees: typed function, nothing else
```

### NounBuilder changes

the NounBuilder (AST → nox noun) gains type-aware lowering:

```
build_expr(BinOp(Add, a, b)):
  type_a = typeof(a)
  match type_a:
    Field, Fp2, ... → emit [5 [build(a) build(b)]]     // nebu add
    Bit              → emit [11 [build(a) build(b)]]    // kuro xor (add in F₂)
    Cost             → emit [4 [10 [build(a) build(b)]] [build(a)] [build(b)]]  // min via branch+lt
    Lattice          → emit jet_recognized composition  // jali ring add
    Iso              → emit [5 ...]                     // genies F_q add (multi-limb)
```

### new std modules

```
trident/std/
├── (existing: field, math, crypto, data, io, graph, nn, private, quantum, ...)
├── bt/           primitives: Bit, Nibble, Byte. fns: quantize, dequantize, popcount
├── wav/          primitives: Lattice, Eval. structs: Cipher, Signal. fns: ntt_multiply, automorphism, bootstrap
├── opt/          primitives: Cost, Gain. fns: shortest_path, critical_path, hungarian, viterbi, transport
└── sec/          primitives: Iso, Shade. structs: Walk, Phantom, Mask, Blind, Fate, Pact. fns: group_action, stealth_send, vrf_eval
```

each module: ~500-2000 LOC Trident. type definitions + functions using new primitive types.

## what changes in the compiler

| component | change | scope |
|-----------|--------|-------|
| parser | new primitive keywords: `Bit`, `Nibble`, `Byte`, `Lattice`, `Eval`, `Cost`, `Gain`, `Iso`, `Shade`, `F2<N>` (10 + generic) | grammar.md |
| type checker | regime inference from types, cross-type boundary detection | typecheck/ |
| NounBuilder | type-aware pattern emission, boundary insertion | ir/noun/ |
| cost model | per-regime costs (from nox vm.md cost table) | cost/ |
| target config | vm/nox/target.toml gains type→regime mapping | vm/nox/ |
| std library | 4 new modules (bt, wav, opt, sec) | std/ |

### what does NOT change

- 16 nox patterns (frozen, checkpoint 0)
- Trident syntax (no new keywords beyond type names)
- TIR path for stack/register/GPU targets (unchanged)
- existing Field/bool/u32/Digest types (unchanged)
- existing std modules (unchanged)

## interaction with nox target

the nox target (vm/nox/target.toml) from cyber-stack-adoption.md gains:

```toml
[types]
regimes = ["nebu", "kuro", "jali", "trop", "genies"]

[types.nebu]
primitives = ["Field", "bool", "u32", "Digest", "Fp2", "Fp3", "Fp4"]

[types.kuro]
primitives = ["Bit", "Nibble", "Byte"]
generic = "F2<N>"

[types.jali]
primitives = ["Lattice", "Eval"]

[types.trop]
primitives = ["Cost", "Gain"]

[types.genies]
primitives = ["Iso", "Shade"]

[boundary]
cost = 766   # F_p constraints per type transition
mechanism = "hemera_commitment"
```

## interaction with 16 languages

the 16 [[cyber]] computation languages map 1:1 to Trident std modules:

| language | std module | primary types |
|----------|-----------|---------------|
| Tri | std.field (existing) | Fp2, Fp3, Fp4 |
| Tok | std.token (existing) | Field (conservation) |
| Arc | std.graph (existing) | Field (category) |
| Ten | std.nn (existing) | Field (tensor) |
| Bt | std.bt (NEW) | Bit, Nibble, Byte |
| Wav | std.wav (NEW) | Lattice, Eval |
| Opt | std.opt (NEW) | Cost, Gain |
| Sec | std.sec (NEW) | Iso, Shade + structs: Walk, Phantom, Mask, Blind, Fate, Pact |
| ... | ... | ... |

existing std modules already serve as languages — they just need the new types to reach all five algebras.

## estimate

| task | sessions |
|------|----------|
| parser: new type keywords | 0.5 |
| type checker: regime inference + boundary detection | 2 |
| NounBuilder: type-aware emission | 2 |
| cost model: per-regime tables | 0.5 |
| target config: type→regime mapping | 0.5 |
| std.bt module | 1 |
| std.wav module | 1.5 |
| std.opt module | 1 |
| std.sec module | 1 |
| tests + docs | 2 |
| **total** | **~12 sessions** |

depends on: nox target (cyber-stack-adoption Phase 2), nox VM implementation (Phase 1 of bootstrap plan).

## relationship to other proposals

- **cyber-stack-adoption.md**: nox target + NounBuilder. THIS proposal extends it with type-driven regime dispatch
- **noun-types.md**: why nox drops cell?. compatible — this proposal adds new PRIMITIVE types, not runtime type tests
- **polynomial-target.md**: polynomial noun lowering. compatible — polynomial nouns work across all regimes

## risk

1. **type proliferation**: 4 new primitive type families = more compiler complexity. mitigation: each family is small (2-3 types) and maps directly to one algebra
2. **cross-type overhead**: ~766 constraints per boundary. mitigation: most programs stay within one algebra. mixed programs pay explicit cost
3. **nox target not yet built**: this proposal extends a target that doesn't exist yet. order: build nox target first (cyber-stack-adoption), then extend with five algebras (this proposal)
