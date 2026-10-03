# Verified Bootstrap execution plan

Active milestone: VB — Verified Bootstrap on soft3; status open, planning only.
The working plan and gates VB0–VB8 live in [roadmap/verified-bootstrap.md](../../roadmap/verified-bootstrap.md).
SH0–SH8 remain accepted for their original frozen-S1 execution scope.
User requires maintained Rust and Trident implementations of every critical component: compiler, Eidos, nox, full scoped Zheng prover/verifier, Joy and dependencies; missing ports keep VB open.
Next delivery: VB0 inventory/claims/trust ledger and Rust/Trident operation matrix; Eidos E0–E4 are mandatory. Then the bounded VB1 independent checker/proof-bridge experiment.
Separate DDC source correspondence, compiler/nox semantics, protocol/verifier soundness, prover completeness/privacy and delivery provenance.
Proposed VB1: independent Eidos kernel checker in CakeML justified through HOL4; VB2 separately adds direct Trident-source interpretation/comparison; compiled DDC also needs an independent executor. Both remain open. Language analysis: roadmap/bounded-metaprogramming.md.
Use owning feature branches and PRs to release/0.4. Versions/default branches/tags/publication keep owner policy.
This documentation introduces no accepted VB implementation or new native proof result; preserve original SH receipts and temperatures.
