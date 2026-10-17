# Constant export row lookup

Status: measured local component optimization; no self-build acceptance.
Base Trident revision: `213b32fbdfa7a9393afb8c547480b0d47726a965`.

## Pre-edit plan

`constant_exports.find` originally reconstructed a complete Binding through
`read(bank, id)` solely to compare the name, then read the same sequence row
again to advance. The private bank is constructed by `push`; this lookup does
not admit external serialized records. The planned change was to load each row once,
project its complete name span for the unchanged two-source comparator, and
reuse the row for its predecessor. Public `read`, `push`, type/value/literal
provenance and all name/source/capacity checks remain unchanged.

Before editing production source, the original module was retained in
[before.tri](before.tri). A single complete source map was captured after
the edit; paired measurements override only this module with the old source.
Normal regression tests follow the live source closure. Planned checks:

- Paired native component results for hit/miss chains, duplicate final binding,
  foreign literal provenance, distinct source offsets and long source names.
- Exact canonical output bytes plus successful reductions, allocations and
  frames for the same cases and fixed quotas.
- Existing constant imports/linking/provenance and default arena fixtures.
- Full C1 artifact footprint from both immutable source variants, component only.

No full C1(S), C2 corpus or C2/C3 result is established by this unit.

## Results

[receipt.json](receipt.json) pins exact commands, source/test/binary hashes and
logs. [sources.json](sources.json) contains all 94 captured source strings
(369820 bytes). The capture used base `213b32f` plus in-progress alias/function
lookup changes; these exact bytes are shared by both variants. Only the
constant-export module differs. The captured sources also matched the live
closure after function-export commit `12a164b`.

Each paired component run includes public source/sequence admission and bank
construction, uses 20000000 reductions, 196608 arena nodes and 65536 frames, and
compares the complete canonical output bytes against independent expected
metadata. All 17 cases pass: newest duplicate binding, distinct module owners,
misses/empty owner, foreign literal origin, separate caller/source offsets,
7/8/255/256/300-byte names at source offset 5000, and 32-record search chains.

| Query | Before reductions | After reductions | Before nodes | After nodes |
|---|---:|---:|---:|---:|
| Oldest binding in 32 rows | 1220284 | 1135000 | 68164 | 66953 |
| Missing name in 32 rows | 916033 | 828769 | 64117 | 62869 |
| Final repeated binding in five rows | 124218 | 121515 | 16781 | 16775 |
| Empty owner chain | 106385 | 106385 | 16332 | 16353 |
| 300-byte matching name | 3272564 | 3271971 | 92924 | 92952 |

Small cases can allocate more: the component fixture grows from 8111 to 8132
loaded nodes, and the longest-name case increases peak frames from 2571 to
2574. The complete C1 footprint instead shrinks from 9708336 to 9706396 bytes
and from 100780 to 100760 loaded nodes. These are two different artifact
closures; neither component timing nor footprint proves full compilation cost.

All 15 existing constant-related regressions pass, including declaration
admission, private/replaced bindings, literal provenance, diagnostics and old
default arena behavior. The separate unchanged locals boundary passes at
195882 nodes under its original 196608-node cap. `cargo check --tests` passes
with zero Rust warnings. The symbolic audit exits 2 with UNKNOWN because its
aggregate parameters/returns are unsupported; that is no proof claim.

The initial component test failed only because it demanded a strict reduction
saving even for an empty owner chain, where no row is read. The
[original failure log](component-initial.log) is retained. The test now asserts
strict savings for nonempty searches and complete semantic equivalence for
every case; production source was unchanged after that correction.

The production bank remains private and constructed only through typed `push`.
Skipping reconstruction of already-owned metadata during name search removes
no external admission. `read` and `push` are unchanged; a matched row still
returns all owner/type/value/literal fields exactly. No fixture quota increased.

For frozen reproduction use the paired command in the receipt with
`TRIDENT_CONSTANT_EXPORT_CAPTURED_SOURCES=1`; ordinary tests capture current
production sources. Full C1(S), supplied-C2 corpus and C2/C3 acceptance remain
separate gates.
