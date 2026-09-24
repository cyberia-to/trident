# SH3 native compiler locals

Delivery: `feat/0.4-native-compiler-locals` after accepted halting repair; merge into `release/0.4`. Whole compiler/self-build remain open. Follow `reference/self-hosting.md` and `reference/self-hosting-jobs.md`.

Support one Field-returning main: inferred/explicit Field let, let mut, assignment, same-scope shadowing, local reads and existing decimal/+/*/parenthesized expressions. Resolve initializer before binding. Each declaration gets a stable slot; assignment reuses it; RHS executes once. Unknown names/immutable writes use semantic diagnostic5. Diagnostic4 stays import-cycle; unsupported constructs6, capacity7.

Add native postorder Expr Seq (Literal/Local/Add/Mul, backward child IDs), ordered statements, final expression ID and binding table (full identifier source span, slot, mutability, type). Compare all identifier bytes: Token.value is only a keyword aid and is zero beyond seven bytes. Reuse shunting-yard with expression IDs; separate expression parsing from body parsing.

Emit balanced frame `[0 [0 E(h)]]`; slot axis is `7*2^h+slot`. Derive h from actual declarations, never requested limits. Port edit construction from `src/ir/tree/lower/nox/path.rs`: RHS once, siblings against old subject, compose statements in order. Preserve existing arithmetic artifact bytes when there are no locals.

Keep source4096/live expression stacks64. Bound AST/statements/bindings by min(job sequence_limit,SOURCE_CAP), check before growth, bound every search/parser/emitter loop. Account for generated formula and ART1/RES1 depth. Preserve shared guest validation/Joy limits; SH4 source-admission lifetime arena remains open. Files: syntax/ascii/lexer/parser/emit plus native symbols/ast/expression/codegen modules as needed.

Acceptance: build C1 once; fresh JOBs compile inside nox; Joy independently executes generated ART1, compare expected values and Rust seed. Corpus: typed7, dependent25, mutable snapshot709, shadowing8, full long identifiers, unknown/self-read, immutable write/shadow, malformed syntax, parser chunks, exact sequence cap and cap−1, output identity under permissive limits, failure preserving old output. Record committed revisions/reductions/arena/frames. Update canonical grammar before implementation; full gates and installed CLI receipts before integration. No host language stages in driver.
