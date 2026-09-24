# SH3 seed: resolved failure paths and scoped bindings

Source: Trident `d5f5c3761a745eb717def822fb02184e6aa01bad`, Trisha `981d502105879b26e93b0cfa884aa45ffb377318`.

A direct resolved `assert(false)` can end a typed function or branch without a
continuing result. Shared TIR compares result widths only for continuing paths;
scalar and aggregate results retain their caller's live values. Explicit returns,
nested conditionals and match arms execute the actual failing assertion. Ordinary
functions, generics and imported wrappers retain their own return semantics.

Intrinsic identity comes from active public declarations, independently of
transitive target requirements. Both native owners retain each module's callable
bindings; later siblings and private declarations cannot redirect them. Ordinary
and generic definitions replace one another consistently. Visible constants keep
their defining bindings, and a local struct root shadows module constants in
both type checking and execution. Return coverage shares target-aware constant
conditions and conservative U32 nonemptiness; defensive loop fallbacks keep their
previous acceptance.

Validation used clean detached worktrees and a freshly regenerated vendor tree.
The receipt retains the superseded dirty-input draft and its provenance findings.

The [language contract](../../reference/language.md#return) and
[IR contract](../../reference/ir.md) specify these rules.
[The receipt](sh3-halting-validation.json) records exact inputs, commands, log
identities, failed drafts and final results. Those commands passed
941 Trident workspace tests, 120 Joy workspace tests and
380 Trisha CPU tests, with 4 existing ignored cases. Rust checks
report no warnings. Trisha verified 133 fixtures and 43 independent baselines;
all result/cycle rows match the previous accepted value-layout delivery.

Actual nox execution covers raw and legacy struct-root shadowing, direct and
qualified assertion identity, typed failures and imported callable ownership.
Triton checks both profiles, preserving neighboring words and requiring
`InstructionError::AssertionFailed` for negative cases. It also exercises
constant-taken nested branches, field-wrapped loop endpoints, pattern binding
shadowing, cfg-selected intrinsics, partial short-alias overlaps and earlier
helper scopes.

After rebuilding committed sources, the installed owners passed
10 CLI commands and 6 observations in [this receipt](halting-cli.json).
Joy executes the continuing branch and preserves its previous output when the
failure branch is selected; Trisha preserves the scalar result and caller word,
and emits no output on assertion failure. Both profiles run the same source
library with an imported assertion inside a statically taken nested condition.

```sh
python3 audit/self-hosting/run-halting.py \
  --joy ../install/bin/joy --trisha ../install/bin/trisha \
  --output /tmp/halting-cli.json
```

## Review findings and remaining work

Review found and repaired constant-taken branch divergence, noncanonical loop
bounds, inactive/private intrinsic leakage, global alias maps overriding earlier
modules, stale ordinary/generic bindings, private constant capture and local-root
shadowing. The complete CPU suite caught an overly broad unreachable-code
rejection; the existing defensive fallback behavior was restored. Failed drafts
remain identified in the receipt.

This closes the seed halting prerequisite. The native compiler is still the SH2
arithmetic pilot. Next is native Field locals and assignments, followed by full
compiler language coverage. Compiler-scale allocation, complete self-build,
fixed point, six-platform acceptance and native Zheng proofs remain open. The
compiler entry and seven native libraries retain UNKNOWN formal verdicts.
Noun's roadmap temperature remains 128K until full-generator/self-build evidence.
