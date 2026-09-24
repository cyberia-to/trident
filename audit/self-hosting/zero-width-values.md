# SH3 seed values: preserve zero-word layouts

Source: Trident `f11a4320ed2b42be5f4a32789c06c5c5fe4fa1bd`, Trisha
`3140d30ff2a9d4ba57641148e3f227148577cc0f`.

The shared TIR model now retains one logical record per expression result,
including Unit, empty structs and arrays whose elements occupy no machine
words. Bindings, aggregates, argument evaluation, tuple assignments and return
cleanup preserve neighboring values. Ordinary scalar return cleanup keeps its
existing instruction sequence.

Logical array extents resolve through checked arithmetic and defining-module
constants, independently of physical width. Unit and empty-array inference
retain component layouts through mixed tuples and nested arrays. Empty copies
skip selection-tree expansion while retaining index checks and RHS effects.
Entry transport validates nested zero-word layouts once rather than traversing
every logical element. Extents outside U32 reject explicitly. Trisha preserves
the detailed unresolved-layout reason in its entry diagnostic.

[The IR contract](../../reference/ir.md) defines these invariants.
[Validation](sh3-zero-width-validation.json) records exact source revisions,
commands, test totals and log identities. The shared source fixture runs on
actual nox and Triton VMs in debug and release; independent expected values
check neighbor preservation and array boundary failure. Trisha also checks
ordered public effects, early returns, imported generics, assembly word
consumption, inferred Unit arrays, caller-vs-defining-module size constants,
and bounded generated code for a large array of empty elements. Oversized
outer and nested extents fail before entry traversal.

The recorded commands passed 928 Trident workspace tests, 120 Joy workspace
tests and 370 Trisha CPU tests, with four existing ignored cases. The final
focused run covers all seven zero-width test functions, including the added
generic-size shadowing case. Trisha verified 133 fixtures and 43 independent
baselines with unchanged result/cycle rows. Rust checks report no warnings.

After rebuilding committed sources, the installed owners passed all 14 CLI
commands and eight observations in [the receipt](zero-width-cli.json): both
profiles return the independent expected values, out-of-bounds execution fails,
and failed Joy publication preserves the previous output.

```sh
python3 audit/self-hosting/run-zero-width.py \
  --joy ../install/bin/joy --trisha ../install/bin/trisha \
  --output /tmp/zero-width-cli.json
```

## Review findings and limits

The first implementation passed its explicit Empty-value corpus. Independent
review found that Unit-return inference and nested empty-array inference still
lost layout, and that a module constant in an array extent was initially
resolved without its defining scope. Both were repaired and added to actual
execution coverage before acceptance. A subsequent review found the nested
oversized-entry bypass; the final regression covers that case as well.

The earlier parser fixture's unresolved-parameter failure is resolved by this
value invariant. The public native compiler remains the bounded SH2 arithmetic
compiler. Resolved halting calls and the complete native compiler feature set
remain SH3 work; compiler-scale memory, complete self-build, fixed point,
six-platform acceptance and native Zheng proofs remain open. Existing formal
compiler-library analysis remains UNKNOWN. Noun's roadmap milestone remains
128K until the full generator and self-build acceptance are supplied.
