# SH3 native compiler control

Delivery `feat/0.4-native-compiler-control` after accepted locals, base/PR `release/0.4`; keep master unchanged. Canonical `reference/self-hosting.md` first; sync stale Field-condition prose in language/nox/runtime refs to the existing formal-audit contract (nox0 selects then; Triton nonzero). Bool literals/locals/mutation, equality, scoped if/else/else-if and early return; full self-build remains open.

Match seed: if is a statement, never an initializer expression. Normalize only function-tail and terminal-branch tails to Return; intermediate branch values are evaluated/discarded. Native true0/false1 and condition0 selects then. Support existing Field conditions too; test0/nonzero. Defer U32 `<` until checked as_u32/as_field support.

Separate AST op tags from lexer kinds; explicit precedence equality<add<multiply. Add Field/Bool type tags and spans, typed bindings and semantic5 errors for mismatched assignment/equality/return. Branch-local binding scopes restore on exit; runtime slot allocation stays monotonic.

Keep postorder expression Seq; add explicit statement/block arenas with owned lists (Write/If/Return/Eval), never assume nested blocks occupy contiguous statement ranges. Use bounded explicit parser continuation frames, with scoped symbol state. Codegen block flow follows native/layout.rs: Continue[0 subject]/Return[1 value], one continuation, return skips it; prepare same convention for calls/loops.

Acceptance: full source JOB through Joy, differential Rust seed and independent expectations; Bool mutation/equality precedence, terminal branches, intermediate tails, outer mutation vs branch shadowing, nested/else-if and early returns, Field condition truthiness, type errors/unknown names in unselected arms/missing return/unreachable direct-return/malformed braces. Exact nesting/Seq caps, generator depth component gate, resource receipts; previous arithmetic/local artifact identities unchanged. Next: typed reusable functions with forward signatures, balanced code table, arguments once left-to-right, fresh frames and recursion rejection.
