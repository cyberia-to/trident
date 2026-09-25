# Explicit module imports across seed compiler stages

Source: Trident `ef4ea23f75446a4f87a12714653f37e1315b9e44`. [Pinned validation](explicit-imports-validation.json)
records commands, sibling revisions and local macOS ARM64 evidence.

Only direct `use` declarations expose public symbols, in source order. Repeating
an import restores its conflicting bindings while retaining other symbols.
Discovery uses parsed headers and validates the requested canonical owner;
legacy spellings resolve once. Supplied AST collections enforce the same boundary.
The final active struct declaration controls export visibility.

Canonical layouts remain available for opaque returned values. Source aliases
are resolved once in their declaring scope; a caller's same-basename import
cannot alter a returned record's layout. Nox distinguishes absent qualified
bindings from local names, including the builtin `os.state.read`. Direct TIR
returns lowering errors instead of exporting error comments as usable IR.
Editor hover, completion, signature help and navigation use exact visible
bindings from the live buffer, including incomplete bodies. References/rename
remain lexical and are outside this change.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/check-explicit-imports.py --joy ../install/bin/joy --trisha ../install/bin/trisha --trident ../install/bin/trident --output /tmp/explicit-imports-final-cli.json
python3 audit/self-hosting/check-callable-tir.py --output /tmp/explicit-imports-final-direct-tir.json
```

All 1114 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha tests remain ignored. All 133 fixture rows and 43 manual
baselines match the callable-ownership receipt. All 120 packaged library files
(106 Trident and 14 Trisha) pass [installed target checks](explicit-imports-libraries.json).
The full suite exposed stale Trisha owner-path/error-boundary fixtures and missing
field imports in PBS/RLWE; those are corrected, with execution assertions retained.

The [installed CLI receipt](explicit-imports-cli.json) records 79 commands and 27
observations: 12 positive projects execute on Joy and Trisha, and 15 rejected
projects preserve prior outputs. The [direct TIR receipt](explicit-imports-direct-tir.json)
records 15 actual Triton VM executions from checker-produced generic bindings.

Post-source-commit installs reproduce all three executed binaries. Entire C1 is
identical to the accepted attributes compiler. Its complete 1190-command installed
guest corpus was not rerun for this seed-only change; that evidence remains
applicable by exact artifact identity. C1 particle:
`6ee70c5d760a2c01844ac738b08e3ce6977f5ca525d14f9b20829cf26e1e0048`.

All 90 formal audits remain UNKNOWN. True guest imports, full SH3/SH4 scale,
generated profiles, C2/C3, six platforms and native Zheng compiler proofs remain
open. Noun stays 128K; no roadmap temperature gate is newly closed by this fix.
