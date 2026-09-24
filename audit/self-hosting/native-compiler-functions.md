# Native compiler: reusable typed functions

Source: Trident `5339030b66c8e9835d79f175ef2ac5eb98d1e75e`, Joy `a3dd4c5c7c2f870f6632deace5b137a141796173`,
Trisha `908d5e22a669e4aa0b6c02b5e9134e67c1a25675`, nox `5271961a72a1f1e9922df36e3e4e3c65d1acff81`.
[The pinned receipt](sh3-native-functions-validation.json) records local
validation inputs, commands and failed drafts. This closes the functions slice
of SH3: fresh source with Field/Bool parameters and Field/Bool/Unit results
compiles inside nox into separate ART1 programs executed by Joy.

Headers and body spans are collected before any body is checked. Forward calls
resolve against final function bindings, independently of local variables.
Every declared body is checked against its own result type, including unused
and shadowed declarations. Positional parameters preserve arity and immutable
slots; repeated names select the final parameter. This follows the seed's
existing replacement semantics.

Nested calls own linked arguments; each callee receives their source-order
values in a fresh balanced frame, followed by zero local padding. Checked bodies
are emitted once into a sorted reachable code table. Native apply selects the
body through a table axis without quoting that producer. Callee return flow is
unwrapped at the function boundary. Declaration order and unused well-typed
functions do not alter the extracted executable. Previous arithmetic, locals
and control artifacts keep their identities.

Cycle detection includes all final callable definitions and unselected branches.
Memoized suffix heights preserve the depth ceiling even when a previously
checked tail is reached through a longer path. Recursion uses semantic code5;
JOB1 code4 remains reserved for import cycles. Unit fallthrough/bare return,
local inference, equality and return coverage are checked separately from
Field/Bool. The [subset contract](../../reference/self-hosting.md#native-compiler-subset-contract)
records the exact supported syntax and limits.

## Execution acceptance

The three installed owners were rebuilt after the implementation commit before:

```sh
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-functions-cli.json
```

[The installed CLI receipt](native-functions-cli.json) records 242 commands
and 84 observations. C1 is built once before case files exist. Joy
packs those exact files, executes compiler stages inside nox, publishes the
returned ART1, and executes the result. The corpus covers forward/nested calls,
argument order and frame padding, Bool/Unit, early returns, function/local name
collisions, duplicate definitions/parameters, and code-table identity.
Unknown calls, wrong arity/types, invalid unused bodies and recursive cycles
return bound diagnostics. Rejected jobs preserve existing destination files.

C1 particle: `a2d472f1c8bc2c2016c613b36765d7d3f533c9f1053d704ef4ff269f3e270b30`.
Measurements below use the installed command and source revision above;
allocated nodes are lifetime allocation, separate from reserved physical memory.

| Case | Result | Charged reductions | Allocated nodes | Peak frames |
|---|---:|---:|---:|---:|
| precedence | 14 | 1071209 | 71368 | 500 |
| typed-local | 7 | 1256388 | 68491 | 565 |
| early-return | 7 | 2519616 | 99599 | 653 |
| body-chunks | 9 | 4906212 | 168551 | 731 |
| function-forward | 3 | 1643022 | 84105 | 637 |
| function-nested | 34 | 3400062 | 129089 | 710 |
| function-frame | 338 | 3963617 | 142880 | 742 |
| function-unit | 7 | 2777932 | 111852 | 653 |

Against the [control receipt](native-compiler-control.md) from Trident
`0efff6c1658c6d89218e3c009caeea96457575ab`, measured with the same installed
runner command on that revision, compiler overhead increased:

- precedence: 850440 → 1071209 reductions; 51106 → 71368 allocated nodes.
- body-chunks: 3693956 → 4906212 reductions; 134737 → 168551 allocated nodes.

Header scanning, checked body storage and graph/table planning add work even
when the emitted no-call program retains its identity. SH4 must address actual
compiler-scale allocation; these measurements do not claim improved cost.

The full source/JOB test corpus also compares the Rust seed and independent
expected values. All 970 Trident workspace tests, 120 Joy tests and 380 Trisha CPU
tests passed, with four existing Trisha tests ignored and zero Rust warnings.
Trisha verified 133 fixtures and 43 independent manual baselines; result/cycle
rows match the control delivery. All 37 compiler formal audits, including the
entry, return UNKNOWN. Execution tests do not establish a compiler proof.

## Boundaries and remaining work

Source/JOB calls cover 7/8/9 arguments, uneven frames and nested call ownership.
An actual function-generator probe independently measures emitted DAG depth:
the exact RES1 allowance passes and one less returns diagnostic7. It includes
the sorted table, dynamic dispatch and padded callee frame. This is a component
probe because the same requested depth in full JOB admission also bounds C1.

A prepared compiler-owned graph exercises 127/128/129-function paths with a
memoized shared suffix and both shortcut orders. It uses an explicit 786432-node
component arena and 512 MiB test stack. This validates the graph algorithm's hard
ceiling, separately from the production 196608-node JOB allowance. The original
smaller probe exhausted resources; increasing its inline arena then exceeded
the original 256 MiB test stack before evaluation. Failed drafts and the stack
sample are recorded in the validation receipt.

The actual call-marker push is tested at the delimiter boundary with prepared
frames. Full expression parsing crosses 7/8/9 nested calls and mixed parentheses.
The installed 64-call JOB, 31-assignment workload and 4096-byte admission workload
still exhaust the default execution allowance without replacing the old program.
These failures remain SH4 work; a logical syntax ceiling does not promise that
every input below it fits the independent runtime allowance.

Next is a bounded heap-arena path in nox/Joy, followed by remaining native
language coverage, including aggregates, imports and compiler-scale source
handling. SH3/SH4 remain open. C2/C3 self-build, six-platform release acceptance
and native Zheng compiler proofs are still open. Noun's temperature remains 128K.
