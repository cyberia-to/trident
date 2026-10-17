# Packed module-path comparison

`job.before` shares each packed-word lookup across its byte lanes. Whole equal
words advance together only when both names contain every lane; unequal or
partial words retain byte-order comparison through the first difference or end.
The first byte pair is charged before either tree read. An equal full word then
charges the remaining byte pairs together. Scalar fallback charges each later
pair before comparing it. The Package handle, record reads, candidate validation,
shared allowance and binary-search protocol stay unchanged.

The [receipt](job-word.json) records commands, revisions, fixed allowances, final
source SHA256 values and raw logs. Baseline `ca9d926` supplies the private
comparator through an immutable source override. The fixture compiles that exact
private module source as a program with a probe entry; production visibility is
unchanged. The following observations include both canonical input admissions.

| Equal prefix bytes | Exact visits, both versions | Before reductions | After reductions | Before nodes | After nodes | Before frames | After frames |
|---|---|---|---|---|---|---|---|
| 0 | 2 | 11186 | 11872 | 5525 | 6095 | 155 | 155 |
| 3 | 8 | 17382 | 15803 | 5693 | 6220 | 155 | 155 |
| 4 | 20 | 31902 | 21233 | 6301 | 6727 | 176 | 176 |
| 15 | 96 | 92592 | 43910 | 7795 | 7900 | 235 | 208 |
| 31 | 256 | 165968 | 54346 | 7973 | 7477 | 364 | 208 |
| 127 | 1536 | 807966 | 223856 | 19249 | 15152 | 1150 | 388 |
| 254 | 3570 | 1681386 | 453929 | 38680 | 29934 | 2174 | 652 |

The first-byte case regresses in reductions and nodes; the table retains it.
Common-prefix cases improve without increasing any limit. These component
results do not establish complete module-discovery performance.

The comparator suite checks exact and one-short allowance boundaries, empty and
prefix ordering, each packed-word/tree-height boundary through the name cap,
and every byte value in every lane. All comparator tests and existing package
lookup/open tests pass. Existing package tests retain their original limits and
cover shared remaining allowance, prefix names, the last permitted name byte,
and package indices beyond the source-table sentinel. `cargo check --tests`
passes with zero warnings. Every result's command and revision is in the receipt.

The [symbolic audit](job-word-audit.json) returns unknown because aggregate/Noun
expressions are unsupported. It reports no compiler warnings. The unused convert
import was removed after the complete suites; the final probe reproduces identical
resource values.

The frozen combined working tree based on `ca9d926` also passes the full Trident
gate: 42 suites, 1,190 passed, zero failed, two ignored, and zero warnings.
All 188 native compiler tests pass with their unchanged quotas. The receipt
records the exact command, working source hashes, Nox input revision, and
[full gate log](job-word-full-gate.log). This integration result includes the
concurrent sorting and byte-binding changes; the isolated comparison table above
retains its original source hashes and commands.
