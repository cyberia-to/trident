# Verified Bootstrap execution plan

Active milestone: VB — Verified Bootstrap on soft3; status open, planning only.
The working plan and gates VB0–VB8 live in [roadmap/verified-bootstrap.md](../../roadmap/verified-bootstrap.md).
SH0–SH8 remain accepted for their original frozen-S1 execution scope.
User requires maintained Rust/Rs and Trident implementations of every critical component: both compilers, interpreters, Eidos, nox, full scoped Zheng prover/verifier, Joy and dependencies; missing ports keep VB open.
Next delivery: VB0 + RS0 inventory/claims/trust ledger and operation matrix, then bounded VB1/E0 seed/kernel experiment. Ceremony and D0–D6 live in soft3/docs/verified-bootstrap.md; Rs RS0–RS9 in rs/roadmap/verified-bootstrap.md.
Separate DDC source correspondence, compiler/nox semantics, protocol/verifier soundness, prover completeness/privacy and delivery provenance.
VB1: reviewed native nox + initial Eidos image → admitted Tri interpreter → Trident → admitted interpreter of restricted Rs B → own native Rs → complete pairs. Root logic/models precede admission; final E4 waits for native RS8. No full Rust interpreter or sealed stage0 prerequisite. Language analysis: roadmap/bounded-metaprogramming.md.
Use owning feature branches, release/0.4 for Trident/Joy/Trisha and separate Rs/soft3 owner review. Versions/default branches/tags/publication keep owner policy.
This documentation introduces no accepted VB implementation or new native proof result; preserve original SH receipts and temperatures.
