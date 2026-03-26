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

see also: `.claude/plans/cyber-stack-adoption.md` — nox target + NounBuilder (Phase 2 of bootstrap)

## Status values

| Status | Meaning |
|--------|---------|
| `draft` | Idea captured, open for discussion |
| `accepted` | Approved — ready to implement and move to spec |
| `rejected` | Decided against, kept for rationale |
| `implemented` | Done — migrated to the relevant `reference/` spec file |
