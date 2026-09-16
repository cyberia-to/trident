# Checked-U32 reuse hints

H0003 previously stored bare variable names across functions and scopes, treated
`split` as a range proof for its input, and suggested deleting a conversion whose
result has a different nominal type. The editor applied that invalid edit.

The canonical contract is now `reference/errors/hints.md`: a hint names an
existing immutable U32 binding that holds the conversion of the same unchanged
Field. Tracking is deliberately restricted to straight-line lexical scopes.
Entering/leaving a scope clears facts; scalar, aggregate and tuple assignment
invalidate facts, as does opaque assembly. Shadowing either the input or saved
result invalidates the association. Mutable result bindings are not recorded.
Only the unqualified builtin as_u32 is recognized; a locally declared function
of that name disables this hint, and qualified lookalikes are excluded.

`split` does not prove its input fits in U32. A converted Field remains nominally
Field. No LSP code action removes as_u32: the diagnostic offers manual reuse of
its named U32 binding instead. No standard-library conversions were removed.

Regression cases cover unrelated functions, conditional branches, loops, input
mutation (including tuple assignment), mutable results, shadowed inputs/results,
split, and a user-defined same-name function. A positive test checks the precise
suggested binding; replacing the repeated conversion with it typechecks, while
replacing it with the Field input is rejected. The editor regression requires no
unsafe edit for legacy diagnostics either.

Conservative limits: facts are not joined across branches or propagated into
nested scopes, even where a more complete flow analysis could safely reuse them.
This favors missed optimization hints over incorrect suggestions.

Validation: `cargo test --release --locked --lib` passed all **674** tests,
including the range-hint and LSP regressions (`/tmp/h0003-full-lib.log`).

Trisha release rebuild passed without Rust warnings. Re-running `trisha bench`
passed **133/133 fixtures and 43/43 baselines**, with **zero H0003** diagnostics
(`/tmp/h0003-bench.log`). This is execution validation; no proof benchmark or
neural verification is implied.
