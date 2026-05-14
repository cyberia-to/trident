---
tags: cyber, cip
crystal-type: process
crystal-domain: cyber
status: draft
date: 2026-03-25
---
# polynomial target — nox engine for the polynomial proof system

Related: [[noun-types]], [[cyber-stack-adoption]]

## the opportunity

trident compiles to 20 VM targets. none of them is [[nox]]. adding nox as an engine target gives every trident program access to the polynomial proof system: proof-carrying execution, ~2 KiB proofs via Brakedown PCS, O(1) data access via polynomial nouns, 3-5 constraint state operations via state jets, and an 89-constraint decider that verifies all history.

no language change. no new syntax. one new compilation backend (NoxLowering). the proof system — nox + zheng (SuperSpartan IOP + Brakedown PCS) — becomes available to every `.tri` program. see [`../reference/vm.md`](../reference/vm.md) for nox's 16 reduction patterns and 5 jets.

## what changes

### new engine: nox

```
trident → NoxLowering → nox formula (16 patterns) → proof-carrying execution → done
```

NoxLowering is structurally similar to the existing TreeLowering (Nock target). both emit tree-structured combinators. the difference:
- Nock: 12 rules, natural number arithmetic, increment-based
- nox: 16 patterns, [[Goldilocks field]] arithmetic, inverse, hash, hint

the mapping:

| trident concept | Nock lowering | nox lowering |
|---|---|---|
| integer literal | Nock atom (natural number) | nox atom (field element) |
| addition | Nock increment loop | nox pattern 5 (add), O(1) |
| multiplication | Nock repeated addition | nox pattern 7 (mul), O(1) |
| comparison | Nock equals + increment | nox pattern 9 (eq) / 10 (lt), O(1) |
| conditional | Nock 6 (if-then-else) | nox pattern 4 (branch) |
| data access | Nock slot (tree walk) | nox pattern 0 (axis), O(1) via PCS |
| construction | Nock cons | nox pattern 3 (cons) |
| function call | Nock 9 (invoke) | nox pattern 2 (compose) |
| hash | not native | nox pattern 15 (hemera) |
| witness | not native | nox pattern 16 (hint) |

nox is a STRICT UPGRADE over Nock for trident: every Nock operation maps to a nox pattern, plus field arithmetic (O(1) instead of increment loops), cryptographic hash (native), and witness injection (privacy).

### replace Tip5 with hemera

trident's stdlib uses Tip5 (5-round Goldilocks hash). the polynomial proof system uses hemera: `Poseidon2(p=2^64-2^32+1, d=7, t=16, Rf=8, Rp=16, r=8, c=8)`. one hash across the entire stack:

```
current:   trident uses Tip5, cyber stack uses hemera → two hashes, two security analyses
polynomial: everything uses hemera → one hash, one security analysis, ~3 calls per execution
```

change: `std.crypto.hash` implementation switches from Tip5 to hemera. API unchanged. programs recompile without source changes. see also [[switch-to-hemera]] for the migration plan.

### replace Tier 2-3 TIR operations

12 of 54 TIR operations (see [`../reference/ir.md`](../reference/ir.md)) assume FRI/Merkle. replace with polynomial equivalents. zheng uses Brakedown PCS + sumcheck — no FRI, no trusted setup:

**Tier 2 (provable) — hash tree operations → PCS operations:**

| current TIROp | what it does | replacement | cost change |
|---|---|---|---|
| SpongeInit | initialize hash sponge | hemera_init (rare, ~3 per execution) | same |
| SpongeAbsorb | absorb field elements | hemera_absorb (rare) | same |
| SpongeSqueeze | squeeze hash output | hemera_squeeze (rare) | same |
| MerkleStep | verify one tree level | PCS.open (polynomial evaluation) | O(log N) → O(1) |

**Tier 3 (recursion) — FRI operations → folding operations:**

| current TIROp | what it does | replacement | cost change |
|---|---|---|---|
| ExtMul | cubic extension multiply | keep (soundness amplification) | same |
| ExtInvert | cubic extension inverse | keep | same |
| FoldExt | FRI folding (extension) | HyperNova.fold (~30 field ops) | different algorithm |
| FoldBase | FRI folding (base) | HyperNova.fold | different algorithm |
| ProofBlock | recursive verification container | fold_row (proof-carrying) | ~8K → ~89 constraints |

**New TIROps:**

| new TIROp | what it does | tier | cost |
|---|---|---|---|
| PCSCommit | Brakedown.commit(polynomial) | 2 | O(N) field ops |
| PCSOpen | Brakedown.open(polynomial, point) | 2 | O(log N + λ) |
| Fold | HyperNova.fold(accumulator, instance) | 3 | ~30 field ops |
| FoldRow | proof-carrying fold during execution | 3 | ~30 field ops |

### state jet recognition

the NoxLowering backend recognizes common patterns in trident code and emits state jet annotations:

```trident
// trident source
fn transfer(ledger: State, from: Key, to: Key, amount: Field) {
    let balance_from = ledger.read(from);
    assert(balance_from >= amount);
    ledger.write(from, balance_from - amount);
    ledger.write(to, ledger.read(to) + amount);
}
```

the compiler recognizes: 2 reads + range check + 2 writes + conservation = TRANSFER pattern.

```
without state jet:  nox trace of ~200 steps → ~1,600 constraints
with state jet:     TRANSFER pattern recognized → 3 constraints
speedup:            530×
```

the programmer writes normal code. the compiler handles the optimization. state jets fire at the nox level, not the trident level — NoxLowering just needs to emit the pattern that nox's jet registry recognizes.

### cost model update

trident's existing cost model (AIR table sizes for Triton/trisha) doesn't apply to the nox engine. new cost model:

```
nox cost model:
  cost(program) = Σ pattern_costs

  pattern 0 (axis):     1 field op (O(1) PCS opening)
  patterns 1-4:         1 field op each
  patterns 5-10:        1 field op each (field arithmetic)
  patterns 11-14:       1 field op each (bitwise, but ~32 constraints in F_p)
  pattern 15 (hash):    736 constraints (hemera, rare — ~3 per execution)
  pattern 16 (hint):    1 (witness injection)

  fold overhead:        +30 field ops per step (proof-carrying)

  total proof cost:     Σ (pattern_cost + 30) per step
  decider:              +89 constraints (once, at verification)
```

simpler than trisha's multi-table AIR model. in nox/zheng, cost IS the polynomial degree, which IS the Brakedown evaluation table size. the nox execution trace is the STARK witness — no separate witness generation step.

```
trident build main.tri --engine nox --costs

  function        steps    field_ops    fold_cost    hemera    total
  ────────        ─────    ─────────    ─────────    ──────    ─────
  transfer()      12       12           360          0         372
  mint_card()     45       48           1,350        736       2,134
  verify_sig()    200      264          6,000        736       7,000

  total:          257      324          7,710        1,472     9,506

  decider: +89 constraints
  proof size: ~2 KiB
  verify: ~0.1 μs (decider jet) or ~5 μs (generic)
```

## what trident programs gain

```
                        trisha target (Triton VM)    nox target (polynomial)
proof size:             ~200 KiB                    ~2 KiB
verify:                 ~50 ms                      ~5 μs (generic) / ~0.1 μs (decider jet)
prover:                 O(N log N)                  O(N) (Brakedown, no FRI)
recursion:              ~200K constraints/level      ~30 field ops/fold (HyperNova)
data access:            O(log N) Merkle walk         O(1) PCS opening
hash calls:             thousands (sponge-heavy)     ~3 (hemera trust anchor)
state operations:       full trace                   3-5 constraints (state jets)
proving latency:        separate phase (trisha)      zero (proof-carrying)
identity:               Tip5 hash                    hemera(PCS.commit ‖ tag)
DAS:                    separate infrastructure       native (polynomial extension)
```

the lowering path: trident TIR (54 ops, 4 tiers) → NoxLowering → nox formula (16 reduction patterns + 5 jets) → zheng proof (SuperSpartan IOP + Brakedown PCS). see [[noun-types]] for how Trident's type system enables this lowering without runtime type overhead.

every `.tri` program recompiled with `--engine nox` gets these improvements. source code unchanged.

## std library impact

### std.nn (neural networks)

```
current:   neural ops in Goldilocks → prove via Triton STARK
polynomial: neural ops → polynomial nouns (model = polynomial)
           quantized inference → Binius F₂ jets (1,400×)
           model weights → PCS-committed, O(1) layer access
           proof: carried during inference (zero latency)
```

### std.private (FHE)

```
current:   FHE primitives planned, no proving integration
polynomial: R_q = F_p[x]/(x^n+1) where p = Goldilocks
           ring operations = nebu NTT (zero conversion)
           proving FHE = proving polynomial ops = native
           ~1% proving overhead on top of FHE cost
           trinity: quantum-safe + private + provable = enabled
```

### std.quantum (quantum simulation)

```
current:   quantum gates over Goldilocks extension fields
polynomial: same gates, but proofs are ~2 KiB instead of ~200 KiB
           recursive simulation proofs fold via HyperNova
           quantum error correction circuits → state jets
```

## implementation plan

```
phase 1: NoxLowering backend                          ~4 sessions
  add vm/nox/ engine config
  implement NoxLowering trait (similar to TreeLowering)
  map TIR operations to nox 16 patterns
  emit nox formulas from trident AST

phase 2: hemera migration                              ~2 sessions
  replace Tip5 with hemera in std.crypto.hash
  update SpongeInit/Absorb/Squeeze TIROps for hemera parameters
  update content addressing in package system

phase 3: Tier 2-3 TIROp update                        ~3 sessions
  replace MerkleStep with PCSOpen
  replace FoldExt/FoldBase with HyperNova.fold
  replace ProofBlock with FoldRow
  add PCSCommit, PCSOpen, Fold, FoldRow TIROps

phase 4: cost model                                    ~2 sessions
  implement NoxCostModel (polynomial degree-based)
  integrate with trident build --costs
  update trident bench for nox baselines

phase 5: state jet annotations                         ~2 sessions
  pattern recognition in NoxLowering (TRANSFER, INSERT, UPDATE)
  emit jet-recognizable nox formulas
  test: trident transfer() → 3 constraints via state jet

total: ~13 sessions (~39 hours)
```

## the 14 languages through polynomial proofs

trident's 14 algebraically irreducible languages ALL compile through nox:

| language | algebra | nox field | PCS | what polynomial proofs enable |
|---|---|---|---|---|
| Tri | $\mathbb{F}_{p^n}$ tower | Goldilocks | Brakedown | native field tower proofs |
| Tok | UTXO conservation | Goldilocks | Brakedown | state jets for transfers (3 constraints) |
| Arc | category theory | Goldilocks | Brakedown | graph schema as polynomial |
| Seq | partial order | Goldilocks | Brakedown | causal ordering via PCS |
| Inf | Horn clauses | Goldilocks | Brakedown | logic as polynomial evaluation |
| Bel | distributions | Goldilocks | Brakedown | probability as polynomial coefficients |
| Ren | Clifford algebra | Goldilocks | Brakedown | geometric product as field ops |
| Dif | manifolds | Goldilocks | Brakedown | continuous dynamics discretized |
| Sym | Hamiltonian | Goldilocks | Brakedown | physics simulation proved |
| Wav | $R_q$ convolution | Goldilocks | Brakedown | FHE native (R_q = Goldilocks NTT) |
| Ten | tensor contraction | Goldilocks/F₂ | Brakedown/Binius | neural inference 1,400× |
| Bt | F₂ tower | F₂ | Binius | binary ops native |
| Rs | Z/2ⁿ words | split | split | systems programming proved |

14 languages → 16 nox patterns → 1 polynomial proof system → ~2 KiB proofs, ~5 μs verify.

see [[polynomial proof system]] for the proof architecture, [[nox]] for the 16 patterns, [[polynomial nouns]] for the data model, [[recursive brakedown]] for the PCS, [[state jets|state-operations]] for CCS jets, [[hemera]] for the hash
