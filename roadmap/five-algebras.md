---
status: draft
date: 2026-03-26
---
# five algebras: type-driven regime dispatch for nox

## problem

Trident v0.5 has four primitive types: `Field`, `Bool`, `U32`, `Digest` (+ `XField` for extensions). all are Goldilocks scalars. this covers ONE of five execution algebras.

the [[cyber]] stack requires five algebras (nebu, kuro, jali, trop, genies) with eight nox instantiations. four algebras have no primitive types in Trident, no std library modules, and no compilation support. programs that need binary computation, ring arithmetic, tropical optimization, or isogeny privacy cannot be expressed natively.

## proposal

add four new primitive type families to Trident. each family maps to one algebra. the type system IS the dispatch mechanism — types determine nox regime, regime determines [[lens]], zero annotations needed.

### new primitive types

```
// kuro algebra (F₂)
BitVec          1-bit packed boolean vector (128 bits per machine word)
BitMatrix       packed boolean matrix for quantized inference

// jali algebra (R_q)
RingElement     polynomial over F_p[x]/(x^n+1), degree n compile-time const
NTTForm         NTT-domain representation of RingElement (lazy conversion)

// trop algebra (min,+)
Tropical        field element with (min, +) semantics: a + b = min(a,b), a * b = a + b

// genies algebra (F_q)
Curve           Montgomery curve over F_q (supersingular)
FqElement       element of F_q (8 × 64-bit limbs for CSIDH-512)
```

### type → regime mapping

the compiler infers regime from operand types. no `#[algebra(...)]` annotation.

```
expression          type of operands    regime      lens
──────────          ────────────────    ──────      ────
a + b               Field               nebu        Brakedown
a + b               Fp2/Fp3/Fp4         nebu²/³/⁴   Brakedown (wider)
a ^ b               BitVec              kuro        Binius
a * b               RingElement         jali        Ring-aware
a + b               Tropical            trop        Tropical
group_action(s, c)  Secret, Curve       genies      Isogeny
```

### cross-type boundaries

when operand types change between sub-expressions, the compiler inserts hemera commitments at the boundary. the programmer never sees this.

```trident
fn mixed(x: Field, bits: BitVec, ring: RingElement) -> Field {
    let q = bt::quantize(x);          // Field → BitVec: boundary inserted
    let r = bt::xor(bits, q);         // BitVec ops: kuro regime
    let f = bt::dequantize(r);        // BitVec → Field: boundary inserted
    let w = wav::ntt_multiply(ring, ring); // RingElement: jali regime
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
    BitVec           → emit [11 [build(a) build(b)]]    // kuro xor (add in F₂)
    Tropical         → emit [4 [10 [build(a) build(b)]] [build(a)] [build(b)]]  // min via branch+lt
    RingElement      → emit jet_recognized composition  // jali ring add
    Curve            → emit [5 ...]                     // genies F_q add (multi-limb)
```

### new std modules

```
trident/std/
├── (existing: field, math, crypto, data, io, graph, nn, private, quantum, ...)
├── bt/           BitVec, BitMatrix, quantize, dequantize, popcount
├── wav/          RingElement, NTTForm, ntt_multiply, automorphism
├── opt/          Tropical, CostMatrix, shortest_path, hungarian, viterbi
└── sec/          Curve, FqElement, group_action, stealth_send, vrf_eval
```

each module: ~500-2000 LOC Trident. type definitions + functions using new primitive types.

## what changes in the compiler

| component | change | scope |
|-----------|--------|-------|
| parser | new type keywords: `BitVec`, `RingElement`, `Tropical`, `Curve`, `FqElement` | grammar.md |
| type checker | regime inference from types, cross-type boundary detection | typecheck/ |
| NounBuilder | type-aware pattern emission, boundary insertion | ir/noun/ |
| cost model | per-regime costs (from nox vm.md cost table) | cost/ |
| target config | vm/nox/target.toml gains type→regime mapping | vm/nox/ |
| std library | 4 new modules (bt, wav, opt, sec) | std/ |

### what does NOT change

- 16 nox patterns (frozen, checkpoint 0)
- Trident syntax (no new keywords beyond type names)
- TIR path for stack/register/GPU targets (unchanged)
- existing Field/Bool/U32/Digest types (unchanged)
- existing std modules (unchanged)

## interaction with nox target

the nox target (vm/nox/target.toml) from cyber-stack-adoption.md gains:

```toml
[types]
regimes = ["nebu", "kuro", "jali", "trop", "genies"]

[types.nebu]
primitives = ["Field", "Bool", "U32", "Digest", "Fp2", "Fp3", "Fp4"]

[types.kuro]
primitives = ["BitVec", "BitMatrix"]

[types.jali]
primitives = ["RingElement", "NTTForm"]

[types.trop]
primitives = ["Tropical"]

[types.genies]
primitives = ["Curve", "FqElement"]

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
| Bt | std.bt (NEW) | BitVec, BitMatrix |
| Wav | std.wav (NEW) | RingElement, NTTForm |
| Opt | std.opt (NEW) | Tropical, CostMatrix |
| Sec | std.sec (NEW) | Curve, FqElement |
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
