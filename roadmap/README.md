# Trident Roadmap

Trident exists to write [CORE](https://cyber.page/core-spec/) — Conserved Observable Reduction
Equilibrium, a self-verifying substrate for planetary collective
intelligence. 16 reduction patterns, field-first arithmetic, BBG
state, focus dynamics — all written in Trident, all provable.

Kelvin versioning: versions count down toward 0K (frozen forever).
Lower layers freeze first.

512K released 2026-02-26. Hot, not production ready.
Developer preview and request for comment.
`cargo install trident-lang` · [GitHub](https://github.com/cyberia-to/trident/releases/tag/v0.1.0)

The primary proving target is self-hosting on the native cyber stack:
nox + zheng (SuperSpartan + Brakedown), all field arithmetic through
strata + honeycrisp, hashing through hemera. trisha remains for
Neptune programs. warrior-cyber is the new primary warrior.

Two milestones define the path:

1. Pipeline closes (256K) — a `.tri` program executes on nox and produces
   a zheng proof, verified locally. Trinity runs end-to-end.
2. Compiler proves itself (128K) — the .tri compiler runs on nox, warrior-cyber
   proves the compilation, recursive proof composition makes stage proofs composable.

## Current Temperature

```
Layer           Current   First Release
───────────────────────────────────────
CORE            256K         64K
vm spec          32K         16K
language         64K         32K
TIR              64K         64K
Noun            256K        128K      ← AST→Noun path (tree targets)
compiler         32K         32K
std.*           128K         64K
os.*            128K         64K
cyber stack     256K        128K      ← strata, hemera, nox, zheng, lens, bbg
warrior         256K        128K      ← warrior-cyber PoC
tooling          64K         32K
AI              256K        128K
Privacy         256K        128K
Quantum         256K        128K
```

## Milestones

| release | what |
|---------|------|
| [256K](256k.md) | pipeline closes — trinity proves end-to-end |
| [128K](128k.md) | the machine assembles — self-hosting via recursive proofs |
| [64K](64k.md) | proof of concept — full .tri→nox→zheng→bbg pipeline |
| [32K](32k.md) | first release — compiler compiles itself |
| [16K](16k.md) | the industries fall |
| [8K](8k.md) | proven everything |
| [4K](4k.md) | hardware era |
| [2K](2k.md) | last mile |
| [0K](0k.md) | sealed |

## Proposals

Proposals for language and VM design changes. Not spec — these are
desires documented for future consideration.

Each proposal is a standalone markdown file. Status and target release
are tracked in the frontmatter.

| proposal | area | status | planned | what |
|----------|------|--------|---------|------|
| [[noun-types]] | type system | draft | 256K | why nox drops cell? and what Trident does instead |
| [[polynomial-target]] | compiler | draft | 256K | polynomial noun lowering for nox |
| [[five-algebras]] | type system | draft | 64K | type-driven regime dispatch: BitVec, RingElement, Tropical, Curve types + 4 new std modules |
| [[warrior-architecture]] | compiler | draft | 256K | core vs tooling vs warriors: workspace split, nox+native in core, stack/gpu/quantum opt-in |
| [[cyber-stack-adoption]] | compiler | draft | 256K | nebu + hemera + nox + zheng integration, AST→Noun path, 14 sessions |
| [[switch-to-hemera]] | compiler | draft | 256K | replace blake3 + custom poseidon2 with hemera, ContentHash 32→64 bytes |
| [[trident-on-evm]] | interop | draft | 64K | EvmLowering + os.ethereum.* + verifier-codegen — six capabilities Solidity/Fe can't deliver, 15 sessions |
| [[cyber-warrior]] | compiler | draft | 256K | warrior-cyber PoC — nox/zheng pipeline, honeycrisp AMX acceleration, hint/witness, trinity parameters, ~12.5 sessions |
| [[field-arithmetic-passes]] | compiler | draft | 32K | Fermat reduction, strength reduction via roots-of-unity, batch inversion, addition chain optimization |
| [[polynomial-optimization-passes]] | compiler | draft | 32K | Schwartz-Zippel equivalence, NTT auto-vectorization, multi-exp fusion, vanishing polynomial, Lagrange caching |
| [[compiler-analysis-passes]] | compiler | draft | 32K | extension field strength reduction, constant folding, dead field ops, algebraic CSE |
| [[supercompilation]] | compiler | draft | 16K | driving + folding over field arithmetic, loop-to-closed-form, partial evaluation via specialize |
| [[proof-cost-types]] | type system | draft | 32K | cost [processor: N..M] bounds on function signatures, compile-time AET verification |
| [[table-aware-types]] | type system | draft | 32K | HashFree<T>, ArithOnly<T> — table constraint types enabling parallel proving |
| [[linear-types-crypto]] | type system | draft | 32K | Linear<Field>, Affine<Field> — prevent nonce reuse and witness leakage at compile time |
| [[refinement-types]] | type system | draft | 32K | field predicates (Positive, NonZero) that compile to STARK constraints |
| [[dependent-types]] | type system | draft | 32K | Vector<N>, Matrix<R,C> — dimension types over field values, compile-time bounds |
| [[contracts]] | verification | draft | 32K | requires/ensures clauses compiled to STARK constraints — execution proof IS verification proof |
| [[loop-invariants]] | verification | draft | 32K | invariant-carrying loops as inductive STARK constraints checked every iteration |
| [[termination-proofs]] | verification | draft | 32K | exact step-count termination embedded in the STARK proof |
| [[private-public-types]] | cryptography | draft | 64K | zk fn, Private<T>/Public<T> — compiler generates witness/public-input split automatically |
| [[commitment-syntax]] | cryptography | draft | 64K | commit(), reveal(), verify() as language primitives with cross-boundary batching |
| [[merkle-iterators]] | cryptography | draft | 64K | verified_walk(root) — compiler generates merkle_step instructions, developer just iterates |
| [[lazy-proving]] | runtime | draft | 64K | defer_proof {} — one STARK for an entire block, amortizes commitment and grinding overhead |
| [[incremental-proving]] | runtime | draft | 64K | prove_delta() — re-prove only affected AET rows when program changes slightly |
| [[speculative-execution]] | runtime | draft | 64K | speculate { fast } fallback { safe } — proof system catches the rare failures |
| [[proof-carrying-code]] | interop | draft | 64K | distribute (TASM + STARK_proof) bundles — recipient verifies without re-execution |
| [[cross-vm-proofs]] | interop | draft | 128K | current_proof() intrinsic — recursive proof composition across heterogeneous VMs |
| [[foreign-function-proofs]] | interop | draft | 64K | extern verified fn — Trident proof transitively covers foreign function execution |
| [[proof-cost-ide]] | tooling | draft | 32K | per-line AET cost highlighting, CI/CD gates via trident-ci.yml |
| [[proof-explorer]] | tooling | draft | 32K | interactive STARK visualizer — table fill, click-to-source, hot zone detection |
| [[trident-repl]] | tooling | draft | 32K | REPL showing proof cost and table breakdown per expression |
| [[algebraic-identity-explorer]] | AI | draft | 128K | GFlowNet mining Goldilocks field theory for compiler optimization rules — compounding flywheel |
| [[nn-trd]] | AI | draft | 128K | field-native neural network library: signed convention, fixed-point, provable inference |
| [[evolutionary-training]] | AI | draft | 128K | train NNs entirely in Goldilocks field arithmetic via evolutionary optimization |
| [[trace-predictor]] | AI | draft | 128K | small NN predicting 9 AET table heights from TIR features before compilation |
| [[cost-surrogate]] | AI | draft | 128K | differentiable STARK cost approximation enabling gradient-based TIR optimization |
| [[instruction-scheduling-nn]] | AI | draft | 128K | GNN on TASM dependency DAG — learned priority scheduling, correctness by construction |
| [[compiler-ensemble]] | AI | draft | 128K | 8-16 specialist optimizers per AET table + meta-selector, 800μs total |
| [[learned-peephole]] | AI | draft | 128K | 1D CNN detecting TASM windows where local substitution reduces proving cost |
| [[neural-decompilation]] | AI | draft | 128K | sequence-to-graph model reconstructing TIR from optimized TASM for cross-pollination |
| [[nn-prover-config]] | AI | draft | 128K | RL agent selecting FRI folding, grinding bits, blowup per program — 10-30% proving speedup |
| [[neural-proof-compression]] | AI | draft | 128K | autoregressive predictor over proof elements — transport-layer ~5× compression |
| [[neural-theorem-prover]] | AI | draft | 128K | GNN finding TASM equivalence rewrite chains; feeds peephole database, consumes identity explorer rules |
| [[adversarial-hardening]] | AI | draft | 128K | GAN loop generating programs to defeat neural compiler, equilibrium IS the quality gate |
| [[backend-transfer-learning]] | AI | draft | 128K | shared TIR encoder + per-target decoder, new backend needs ~10% of original training data |
| [[neural-developer-tools]] | AI | draft | 128K | neural type inference, incremental recompile via diff GNN, fuzzing-guided program synthesis |
| [[categorical-compiler]] | math | draft | 8K | compiler as a functor preserving equivalences — correctness as a categorical theorem |
| [[galois-optimization]] | math | draft | 16K | Frobenius automorphisms, norm/trace maps, subfield detection for extension field optimization |
| [[algebraic-geometry-constraints]] | math | draft | 8K | Gröbner basis constraint minimization, singular point detection, Krull dimension for proof size |

## Status values

| Status | Meaning |
|--------|---------|
| `draft` | Idea captured, open for discussion |
| `accepted` | Approved — ready to implement and move to spec |
| `rejected` | Decided against, kept for rationale |
| `implemented` | Done — migrated to the relevant `reference/` spec file |
