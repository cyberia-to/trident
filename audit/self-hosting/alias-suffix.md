# Direct module alias matching

`qualified_name.matches_owner` replaces three repeated basename scans with a
bounded suffix comparison. Complete canonical equality still wins first. A
shorter prefix must begin immediately after a dot in the owner and its matching
suffix must contain no dot. Thus `field` matches `vm.core.field`, while
`core.field` does not. The function, constant and type readers still inspect
every ordered use, preserving later-public selection and earlier-public
fallback when a later owner lacks the member or keeps it private. No serialized
format, source cap, shared admission charge, or fixture quota changes.

The [receipt](alias-suffix.json) records exact commands, source hashes and logs
from base revision `90ac882`. All 94 source strings were captured once in
[the frozen map](alias-suffix-source-snapshot.json). Before and after use that
same map, replacing only the four owned modules. The complete before C1 retains
the original production sources. For the isolated component, the exact old
private alias body is transplanted into the same helper entry used by the new
algorithm. These observations include admission of both canonical byte inputs.

| Prefix → owner | Before reductions | After reductions | Before nodes | After nodes |
|---|---:|---:|---:|---:|
| `noun` → `vm.nox.noun` | 50,097 | 28,014 | 6,629 | 6,216 |
| `noun` → `std.compiler.nox.qualified_name` | 84,123 | 11,073 | 6,913 | 5,342 |
| `field` → `vm.core.field` | 63,877 | 37,423 | 7,033 | 6,574 |
| `core.field` → `vm.core.field` | 48,919 | 40,770 | 7,288 | 6,914 |
| Complete `std.nox.bytes` equality | 19,903 | 19,903 | 5,736 | 5,609 |
| `x` → 253 `p` bytes followed by `.x` | 852,498 | 37,475 | 24,705 | 7,107 |
| 253 `x` bytes → `p.` followed by that prefix | 2,401,835 | 1,612,564 | 40,633 | 36,678 |

The paired complete C1 artifact shrinks from 9,719,875 to 9,700,617 bytes and
from 100,903 to 100,701 loaded nodes. The ordinary default body-chunks regression
uses 195,823 nodes at the original 196,608-node limit; stack64 uses 194,169.
Their reductions remain 2,808,997 and 2,295,139. These compiler artifact and
component observations do not establish full self-compilation performance.

Differential tests cover all 256 byte values in every packed lane, empty and
trailing-dot strings, partial dotted suffixes, and boundaries through 255 bytes.
All 31 existing import tests pass, including original artifact, visibility,
diagnostic-span and resource-cap checks. Both existing default constants and
locals regressions pass. Ordinary tests capture current production sources;
`TRIDENT_ALIAS_CAPTURED_SOURCES=1` selects frozen audit reproduction.

`cargo check --tests` passes with zero Rust warnings. The symbolic audit returns
unknown for aggregate parameters/returns; it also reports pre-existing unused
`vm.core.convert` imports in unchanged `ascii.tri` and `lexer.tri`. The raw
report and warning output are retained in the receipt; this is no proof claim.

The [static work model](alias-suffix-work-model.json), produced by
[the recorded script](alias-suffix-work-model.py), counts 4,033 imported call
sites and 38,145 ordered-use checks in the frozen compiler source. Their old
basename-location scans require 772,037 owner-byte reads. The suffix model
requires 61,014 owner reads plus 23,400 prefix reads. These are static operation
counts, not measured reductions. Its module-position table is source-derived
inference for interpreting prefix diagnostics, not further guest acceptance.
