# Stable nominal bindings in the seed compiler

Compiler source `5d06645b7190a11b5bdab7ed8e3bad7af993c186`; acceptance harness `5d06645b7190a11b5bdab7ed8e3bad7af993c186`.
The [validation receipt](nominal-bindings-validation.json) retains source snapshots,
revisions, exact commands, install logs, binary hashes and local macOS ARM64 evidence.

Earlier seed probes showed that a function signature could retain a Field layout
while its body used a later Bool field, or retain public-field access after the
field became private. The recorded programs executed and produced 1 and 7,
respectively; their sources, commands and binary pins are preserved in the receipt.
These cases now fail with an incompatible-struct-redeclaration diagnostic.

The canonical rule is one resolved ordered layout per defining module and struct
name. Repeated active declarations preserve field names, order, resolved types and
field visibility, including nested nominal identities. Only the struct's own pub
flag may change final export visibility. Equivalent full/short type spellings and
equal checked array extents remain accepted; inactive cfg declarations are ignored.
Overflowed extents are rejected. Ordinary function signatures and struct fields
retain their existing source-order type resolution.

One shared guard checks repeated names before source checker registration and
before direct nox/TIR canonicalization. It compares canonical nested identities;
checking the invariant for every name prevents A/B/A layout mutation without
expanding recursive descriptors. It does not turn direct supplied-AST entrypoints
into full type checkers or add checks for unique malformed declarations.

The new regression file contributes 11 passing tests covering changed field
type/order/name/privacy, nested layouts, cfg, checked extents, imported aliases,
different owners, final visibility, opaque values and preserved direct-API boundaries.
The [installed CLI receipt](nominal-bindings-cli.json) contains 28 commands,
8 cases and 16 warrior observations. Both Joy and Trisha compare complete positive
outputs and reject the five incompatible-layout cases while preserving previously
executed artifact bytes. Joy uses --force; Trisha build has no such flag and uses
its normal replacement mode.

```sh
python3 audit/self-hosting/check-nominal-bindings.py --joy ../install/bin/joy --trident ../install/bin/trident --trisha ../install/bin/trisha --output /tmp/native-nominal-bindings-cli.json
```

The CLI run originally recorded base HEAD `17685e1353e01a2200f28cfc993a4d006d0c0412`
with the uncommitted repair. Every captured seed/source/harness snapshot is matched
to the committed source or harness revision above. Later clean installation of
all three binaries reproduced the tested bytes exactly; original receipts are
retained without rewriting their observed revisions.

All 1154 Trident, 122 Joy and
380 Trisha tests pass without warnings, with
4 existing Trisha tests ignored. All 133 baseline rows
and 43 manual programs match RootG. The 106 formal audits
report UNKNOWN and establish no native execution proof relation.

The full 1195-command / 401-observation guest corpus was not rerun on RootI.
Its prior RootG execution evidence is reused because freshly rebuilt C1 and graph
artifacts are byte-identical and all pinned runtime libraries and guest sources
are unchanged. The reused corpus was executed at compiler source
`17685e1353e01a2200f28cfc993a4d006d0c0412` with the distinct RootG binary
`dd979f4f808433e6aa6d5eb9307636708425d8db55d6d40704ac6e63bad7235b`; no RootI execution is claimed for those commands.
The original 39-command / 12-observation graph execution is reused on the same basis.

Rebuilt C1: `f8b7768f81d8a4966f94a6a6957e8720c7c8453f7c3a4e52052c284751bcece1` (87017 DAG entries).
Rebuilt graph fixture: `7fff69be1bec5170a989c7ecd31c237e2907af3dce22d2f7fce31b00dc7ce74a` (20637 DAG entries).
Their real build commands and the original receipt hashes are recorded under
guest_execution_reuse. No runtime or guest source/resource limit was raised.

This seed repair is a prerequisite for guest imported types. Dependency nominal
and aggregate signatures, opaque constructors/forwarding, intrinsic declarations,
generated compiler profiles, compiler-scale closure, SH4 and C2/C3 remain open.
Six CPU platforms and native Zheng proof gates remain open. Noun stays 128K.
