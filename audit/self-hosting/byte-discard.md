# Discarded checked-cast bindings

`bytes.validate_eight` now evaluates its first seven checked U32 casts without
binding their unused results. All eight conversions still execute. Seed and
native compiler lowerers retain the effects of discarded expressions, including
out-of-range conversion traps. No admission charge, noun shape, padding check,
or resource limit changes.

The [receipt](byte-discard.json) identifies revision `ca9d926`, exact commands,
source hashes, and logs. The paired diagnostic captures all 94 compiler source
strings once, then builds both C1 variants from that same map. Only the bytes
module differs; its saved before source is in
[byte-discard-before.json](byte-discard-before.json). Each resulting C1 compiles
the same nine-assignment body, and the generated ordinary program executes to 9. These are
local working-tree measurements.

| Paired measurement | Before | After | Saved |
|---|---:|---:|---:|
| C1 artifact bytes | 9,733,067 | 9,719,875 | 13,192 |
| C1 loaded arena nodes | 101,039 | 100,903 | 136 |
| Body-case total arena nodes | 196,252 | 196,025 | 227 |
| Body-case reductions | 2,809,228 | 2,808,997 | 231 |
| Body-case peak frames | 629 | 629 | 0 |

Both paired body cases pass the original 196,608-node limit. A prior combined
compiler version failed at 196,920 nodes, but the concurrent function-sorting
change also reduced its footprint. This paired experiment attributes only the
savings above to discarded byte-validation bindings.

The existing default-limit regressions pass: body chunks use 196,025 nodes and
stack64 uses 194,371. Seven, eight, and nine local assignments use 176,703,
187,383, and 196,025 nodes respectively. Their commands and logs are in the
receipt. The all-slot byte corruption test, all 19 collection tests, and the
existing seed/guest discarded-conversion trap test pass. `cargo check --tests`
passes with zero warnings. Complete self-compilation remains a separate gate.

The frozen combined working tree based on `ca9d926` passes the full Trident gate:
42 suites, 1,190 passed, zero failed, two ignored, and zero warnings, including
all 188 native compiler tests at unchanged quotas. The receipt records its exact
command, working source hashes, Nox input revision, and
[full gate log](job-word-full-gate.log). This integration result includes the
concurrent comparator and sorting changes; the paired measurements above retain
their captured source map.
