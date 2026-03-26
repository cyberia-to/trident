# Design Proposals

Proposals for language and VM design changes. Not spec — these are
desires documented for future consideration.

Each proposal is a standalone markdown file. Status is tracked in the
frontmatter.

## Proposals

| proposal | status | what |
|----------|--------|------|
| [[noun-types]] | draft | why nox drops cell? and what Trident does instead |
| [[polynomial-target]] | draft | polynomial noun lowering for nox |
| [[five-algebras]] | draft | type-driven regime dispatch: BitVec, RingElement, Tropical, Curve types + 4 new std modules |
| [[warrior-architecture]] | draft | core vs tooling vs warriors: workspace split, nox+native in core, stack/gpu/quantum opt-in |
| [[cyber-stack-adoption]] | draft | nebu + hemera + nox + zheng integration, AST→Noun path, 14 sessions |
| [[switch-to-hemera]] | draft | replace blake3 + custom poseidon2 with hemera, ContentHash 32→64 bytes |

## Status values

| Status | Meaning |
|--------|---------|
| `draft` | Idea captured, open for discussion |
| `accepted` | Approved — ready to implement and move to spec |
| `rejected` | Decided against, kept for rationale |
| `implemented` | Done — migrated to the relevant `reference/` spec file |
