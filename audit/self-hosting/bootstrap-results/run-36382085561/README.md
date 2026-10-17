# Native bootstrap phase run 36382085561

Status: accepted for frozen S1. The [original CI run](https://github.com/cyberia-to/trident/actions/runs/36382085561),
attempt 1 at Trident `57491633fbccb58ae44dca2da438ee31430be1bc`, passed all
twelve producers, twenty-four native corpus jobs and its original aggregate.
A fresh restore and independent byte-exact replay of those same artifacts also
passed. Earlier runs supply none of these phases.

## Accepted matrix

| Target | Bootstrap repetitions | C2 corpus jobs | C3 corpus jobs |
|---|---:|---:|---:|
| macOS ARM64 | 2/2 | 2/2 | 2/2 |
| macOS x64 | 2/2 | 2/2 | 2/2 |
| Linux glibc ARM64 | 2/2 | 2/2 | 2/2 |
| Linux glibc x64 | 2/2 | 2/2 | 2/2 |
| Windows MSVC ARM64 | 2/2 | 2/2 | 2/2 |
| Windows MSVC x64 | 2/2 | 2/2 | 2/2 |

Each producer executes both complete S1 self-builds. Every C2 and C3 has SHA256
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
The source inventory, JOB bytes and every non-time worker field match the
original S1 reference. Both native Cargo builds per producer have zero warnings.
Each corpus job executes the same six corpora: 547 observations and 1816 commands,
including 119 explicitly identified Rust reference builds. These are repeated
native checks of the same cases, not additional unique language features.

The 36 expanded artifact trees contain 88488 files and 1621286190 raw bytes,
counting shared copies separately. The producer trees account for 7128 files
and 838706734 bytes; the 24 corpus trees account for 81360 files and 782579456
bytes. Commands, revisions, source pins and exact output identities are retained
in [accepted/retention.json](accepted/retention.json) and its compressed originals.

Original aggregate artifact `10961755660` belongs to successful job
`108868600539`. The original final REST response is successful. Independent
replay restores all three stores into a fresh 36-directory tree, compares every
file with its original ZIP bytes, checks store/API/job/source/runtime bindings,
runs the frozen matrix CLI and compares all 36 receipt hashes with the original
CI aggregate. Its final `verification.json` SHA256 is
`be6d76f37c0daf9694e75dad83e200adc25a32b05e593fdbeec11f4d67a69f81`.

The exact driver is retained as
`accepted/commands/complete-native-acceptance.py.gz`; all four command outcomes
and raw stdout/stderr are in `accepted/replay/`. The
[validator](final-validation/README.md) has its own earlier guard evidence.
An independent second review repeats every original ZIP comparison and frozen
original-origin matrix check; its raw report and command are retained in
`accepted/review/`. Original job logs, the aggregate ZIP/API, final REST response, frozen runners
and expected-input contract are retained byte for byte. The 36 original phase
ZIP containers remain at the immutable local paths recorded in the manifest
and in the original GitHub artifacts; their expanded contents are retained in
the stores. Repeating the full original-container comparison requires those
ZIP containers as well as the stores. The store restore alone verifies the
retained expanded bytes and does not recreate original ZIP container bytes.

## Restore accepted phase trees

These are the final accepted store indices:

| Store | Entries | Index SHA256 |
|---|---:|---|
| `producer` | 12 | `51f791f67822bd9ebaf4ee77f04806d10dff592e64a7528dfc985b1d84c0c13f` |
| `c2` | 12 | `2dcca0ec19a6007f49a453627c353e67706c54bb30f0f5a18bba081b9bcdfa4c` |
| `c3` | 12 | `56345f761f211932f58deb72429379b160159a8064b5886d548d833d228ffe4f` |

For example, restore the producer trees with:

```sh
python3 -B audit/self-hosting/bootstrap-results/archive-tool/archive.py restore \
  --store audit/self-hosting/bootstrap-results/run-36382085561/phase-stores/producer \
  --output /absolute/fresh/producer-results \
  --index-sha256 51f791f67822bd9ebaf4ee77f04806d10dff592e64a7528dfc985b1d84c0c13f
```

Use each role's store and hash, with a separate fresh destination. For strict
final validation, put all 36 restored artifact directories beneath one fresh
matrix directory and supply the original inputs described by
[final-validation/README.md](final-validation/README.md).

## Original producer identities and history

| Native producer | Repeat | Original artifact |
|---|---:|---:|
| macOS ARM64 | 1 | 10955342077 |
| macOS ARM64 | 2 | 10955337661 |
| Windows ARM64 | 1 | 10955053500 |
| Windows ARM64 | 2 | 10955530212 |
| Linux ARM64 | 1 | 10954908883 |
| Linux ARM64 | 2 | 10955242264 |
| macOS Intel | 1 | 10957689633 |
| macOS Intel | 2 | 10956721881 |
| Windows x64 | 1 | 10955124998 |
| Windows x64 | 2 | 10955297375 |
| Linux x64 | 1 | 10956472096 |
| Linux x64 | 2 | 10956667089 |


`components-034/` and its linked snapshots preserve the intermediate component
checks and indices. Their partial verdicts remain unchanged. The final accepted
manifest binds that prior snapshot and both last Intel macOS corpus imports.
Historical failed runs remain in their own directories.

This closes SH6 for S1's declared compiler subset, sources and resource profile.
Native Zheng proofs of the complete compilations remain SH7/SH8 work. Native
compiler acceptance does not complete the separate distribution release gates
or establish full Rust frontend/tooling parity.
