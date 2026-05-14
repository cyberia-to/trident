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

Each proposal is a standalone markdown file. Status is tracked in the
frontmatter.

| proposal | status | what |
|----------|--------|------|
| [[noun-types]] | draft | why nox drops cell? and what Trident does instead |
| [[polynomial-target]] | draft | polynomial noun lowering for nox |
| [[five-algebras]] | draft | type-driven regime dispatch: BitVec, RingElement, Tropical, Curve types + 4 new std modules |
| [[warrior-architecture]] | draft | core vs tooling vs warriors: workspace split, nox+native in core, stack/gpu/quantum opt-in |
| [[cyber-stack-adoption]] | draft | nebu + hemera + nox + zheng integration, AST→Noun path, 14 sessions |
| [[switch-to-hemera]] | draft | replace blake3 + custom poseidon2 with hemera, ContentHash 32→64 bytes |
| [[trident-on-evm]] | draft | EvmLowering + os.ethereum.* + verifier-codegen — six capabilities Solidity/Fe can't deliver, 15 sessions |
| [[cyber-warrior]] | draft | warrior-cyber PoC — nox/zheng pipeline, honeycrisp AMX acceleration, hint/witness, trinity parameters, ~12.5 sessions |

## Status values

| Status | Meaning |
|--------|---------|
| `draft` | Idea captured, open for discussion |
| `accepted` | Approved — ready to implement and move to spec |
| `rejected` | Decided against, kept for rationale |
| `implemented` | Done — migrated to the relevant `reference/` spec file |
