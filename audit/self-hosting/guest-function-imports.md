# Guest ordinary function imports through C1

Compiler source `4acc73021497950154ec6b2e68e1e6f2536a2ba3`; acceptance harness `cdf640a5714c05ab36e2124e5a6dc1f0fb168428`.
The [validation receipt](guest-function-imports-validation.json) pins sibling
revisions, source/harness files, exact commands, logs and local macOS ARM64 evidence.

C1 now resolves ordinary functions across direct JOB1 module imports entirely on
nox. Dependency functions admit Field/Bool/U32 parameters and Field/Bool/U32/Unit
results; the existing entry language is preserved. Private helpers and forward
calls keep their defining owner. Every reached private or replaced body is checked
before final public exports are published. Per-symbol alias replacement,
nontransitivity, final arity, purity and ordinary functions named assert are checked.

Global declaration IDs remain separate from runtime slots. Runtime ordering
compares owner bytes and member bytes independently, preserving generated ART1
when declarations are reordered or unused definitions are added. Diagnostics keep
the original package module index and byte spans. Imported type descriptors,
constructors and intrinsic declarations remain a subsequent slice.

| Installed corpus | Commands | Observations |
| --- | ---: | ---: |
| [full](guest-function-imports-full-cli.json) | 1195 | 401 |
| [imports](guest-function-imports-imports-cli.json) | 133 | 30 |
| [return](guest-function-imports-return-cli.json) | 7 | 2 |
| [constants](guest-function-imports-constants-cli.json) | 139 | 31 |
| [graph](guest-function-imports-graph-cli.json) | 39 | 12 |

Every row links through `installed_cli` to its complete receipt and reproducible
command. Function imports and return rejection receipts partition the final
32-case function harness into 30 and 2 selected cases. Positive imports compare
complete execution output bytes with the seed; rejection cases verify diagnostics
and preservation of the existing output under `--force`.

The full corpus preserves all 235 previous successful compilation observations,
representing 203 distinct ART1 particles. All 227 previously recorded source_hex
values are identical. The remaining 8 resource/cap observations omit source_hex;
their complete JOB1 module identities, source particles and byte lengths match.
All 12 module-graph results and exact admission allowances remain unchanged across
39 commands. All three installed binaries reproduce byte-for-byte, with real
installation commands and log hashes retained. The acceptance host deadline is
60000 ms; the public 30000 ms default is unchanged.

The owner gates report 1154 Trident, 122
Joy and 380 Trisha tests passed without warnings;
4 existing Trisha tests remain ignored. All 133 baseline
rows and 43 manual programs are unchanged. All 111 formal
audits return UNKNOWN; they establish no native execution proof relation.

The original 61–64-bit record-write cases still return 3199 under the same 786432-node
ceiling. The 64-bit case uses 771882 nodes,
36976900 reductions and 1346 peak frames.
The 65-bit case still exhausts that arena and preserves the previous program.
The unchanged 2158-byte long-name source returns 79 using 781466
nodes, 74158193 reductions and 2637 peak frames
under that same ceiling. These are measured costs of the named full-corpus cases,
not a promise that every admitted source fits its requested arena.

Read-only module reviews found no remaining correctness blockers. Complete
dependency types and intrinsics, canonical legacy remaps, generated 1/1 compiler
profiles, compiler-scale source/resource closure and SH4 remain open. C2/C3,
six CPU platforms and native Zheng proof gates remain open. Noun stays 128K.
