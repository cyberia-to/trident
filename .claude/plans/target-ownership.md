# Target ownership follow-up

Status: proposed after the owner's 2026-09-11 request to recheck the migration
and analyze `vm/` / `os/` architecture.

Detailed audit, layout, ownership map and ordered gates:
`docs/explanation/target-ownership.md`.

Baseline assets already live in Trisha; empty Trident directory and misleading
entrypoint instructions were cleaned up. Runtime/ABI migration is pending.
Implement semantics/provider contracts before moving directories. Preserve the
four source namespaces and keep nox lowering in core for this scope.
