# Read each function export row once

The function export bank is opaque and constructs its own bounded records.
Lookup now reads each owner-chain record once, compares its name span, then
decodes the target only for a match. The public `read` API remains unchanged.
Owner selection, newest binding, source-relative names and target zero retain
their meanings. All external package and source admission still occurs.

[The receipt](receipt.json) pins the command, source hashes and base revision
`213b32f`. Its paired measurements use the same saved complete source map and
replace only `std.compiler.nox.function_exports`. The test fixture includes
source admission and bank construction in both measured totals.

| Query in 32 records | Before reductions | After reductions | Before nodes | After nodes |
|---|---:|---:|---:|---:|
| Oldest binding | 1348546 | 1276457 | 50949 | 50842 |
| Middle binding | 863016 | 827341 | 42153 | 42090 |
| Newest binding | 410321 | 410213 | 33410 | 33390 |
| Absent name | 687112 | 612968 | 42152 | 42043 |

The complete seed grows from 9700617 to 9708336 bytes and from 100701 to
100780 loaded nodes. This is a measured footprint cost for less lookup work;
whole-source compilation savings remain unmeasured.

Differential execution covers distinct owners, repeated final bindings, target
zero and 4095, missing and prefix-colliding names, separate source/caller offsets,
empty banks and 511-byte names. All three focused tests and 23 existing function
regressions pass; `cargo check --tests` emits no warnings. The symbolic audit
reports unknown because aggregate parameters/returns are unsupported. It is
retained as unknown. Full self-compilation acceptance remains open.
