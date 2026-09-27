# Extended supplied-compiler routing

Tool readiness on Trident parent `b991d901e6585a40bedd0e0a3d4382c2ad3d89c1`,
with exact changed-file hashes and commands in [identities.json](identities.json).
No actual C2 or full corpus was run for this unit.

The shared constant/function/type import and intrinsic harness now accepts
`--compiler PATH`; the generated-compiler-profile harness accepts the same
option. Each pins the supplied bytes, uses them for every producer JOB1 and
compiler execution, and rejects a compiler seed build or fallback. Default
seed mode retains its single compiler build. Existing source vectors, expected
values, spans, identities and resource limits are unchanged.

Independent Rust oracle builds remain enabled in both modes. Those build and
execution rows carry `reference_only: true`; their raw-profile artifacts are
separate from the supplied compiler and guest-produced outputs. Complete
guest/oracle output comparisons and seed rejection checks are retained.

The shared guard checks compiler and Joy hashes before/after each command,
packed compiler/JOB1 identities, and executed artifact/input identities.
Receipt creation is exclusive and updates use the owned file handle, so a
concurrent path replacement cannot redirect writes into a compiler. Semantic
and finalization failures record failed status, error and end hashes.

[Fourteen focused tests](unit-tests.log) pass with warnings treated as errors.
They cover all shared case providers, default seed routes, retained oracle
comparison, missing/wrong-profile inputs, particle mismatches, input mutation,
output alias/race protection, semantic mismatch, and binary mutation after the
last command. The [nine existing runner tests](existing-route-tests.log) also
pass. These mocked routing checks execute no guest compiler.

The real [focused C1 receipt](provided-c1-final.json) executes the unchanged
`full-short` constant-import case with the saved full C1. All five commands
pass: packing, guest compilation, guest program execution, raw reference build
and reference execution. The complete outputs agree. C1 and Joy hashes stay
unchanged. This is a C1 routing check, not C2 acceptance.

The real [raw-profile rejection](profile-rejection-final.json) fails at the
first Joy pack with `requires compiler profile(1,1)` and never falls back.
The [generated-profile rejection](generated-profile-rejection-final.json)
rejects the same raw ART1 before any Joy command. Its only subprocesses read
Git metadata. Original failed receipts and diagnostics are retained.

[coverage.json](coverage.json), produced by [coverage.py](coverage.py), counts
the retained `source-capacity-*` run receipts and verifies unchanged case-provider
ASTs against the parent revision. It does not infer a new successful run.

| Corpus | Retained observations | Supplied compiler route |
| --- | ---: | --- |
| Main native compiler | 402 | Existing runner |
| Constant imports | 31 | Shared import runner |
| Callable imports | 32 | Shared import runner |
| Type imports | 24 | Shared import runner |
| Exact intrinsics | 37 | Shared import runner |
| Generated compiler profiles | 21 | Generated-profile runner |
| Module graph | 12 | Separate component validation |
| Arena/source capacity | 7 | Separate existing harness |

The main retained receipt has 1198 commands and 402 observations; 401 describes
an older corpus receipt. The graph harness executes a separately built graph
component, not its JOB1-bound compiler, and several vectors have no callable
entry. Relabeling that harness as supplied-C2 execution would overstate coverage.
The separate arena harness remains unchanged. A later actual-C2 acceptance must
record its own complete run results and preserve the existing limits.
