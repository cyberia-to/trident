# Native compiler: reusable literal-range loops

Source: Trident `7e5a27461b0cbad23db15c249734c3ed9095adf8`, Joy `d662ef9f07b6904783c98fd215f75764dfca5254`,
nox `c9f7486a74fe81bfc194b598da40f6343ecb2ef1`, Trisha `908d5e22a669e4aa0b6c02b5e9134e67c1a25675`.
[The pinned validation](sh3-native-loops-validation.json) records all inputs,
commands and local macOS ARM64 observations.

The compiler executing inside nox now compiles `for i in A..B` from literal
source bounds into reusable loop formulas. Nested loops, immutable U32 indices,
outer mutation, scoped locals, calls and early returns preserve the checked
source semantics. Raw decimal bounds are checked before Field normalization;
the exclusive end 2^32 is valid, and the final U32 index never increments past
its domain. Empty bodies are still type checked. Loop tails are discarded;
explicit returns propagate to their own function boundary.

Sorted reachable functions keep their slots. Generated loops follow in owner
order with source-order IDs, independently of child-block postorder. Each body
is stored once, and native apply reads it from the immutable table. All prior
no-loop ART1 identities remain unchanged. Seed and guest table layouts are
separately deterministic; no byte parity between these compilers is claimed.
The [subset contract](../../reference/self-hosting.md#native-compiler-subset-contract)
records range, scope, coverage and unsupported dynamic-bound behavior.

## Execution acceptance

After rebuilding the three installed owners from the committed implementation:

```sh
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-loops-cli.json
```

[The installed receipt](native-loops-cli.json) records 392 commands and
133 observations. C1 is built before fresh case files. Joy packs each exact
source, runs compilation in nox, publishes the returned ART1 and executes that
separate program. New loop compiler jobs explicitly request 786432 nodes. Loop
program execution requests 65536 frames and 10000000 reductions; previous cases
retain their existing allowances. C1 particle:
`ba442a06eb6fedc3ee8399b00b1d988cdd5f5a438eccdffcac61ac4044c3fa17`.

The following measurements use that command and those source revisions.
Allocated nodes count lifetime allocation; they are not reserved memory or RSS.

| Case | Result | Program bytes | Compile reductions | Compile nodes | Run reductions | Run nodes | Run frames |
|---|---:|---:|---:|---:|---:|---:|---:|
| precedence | 14 | 1536 | 1078047 | 77018 | 5 | 22 | 3 |
| body-chunks | 9 | 5804 | 4938665 | 174656 | 92 | 82 | 15 |
| function-nested | 34 | 5871 | 3449436 | 136302 | 38 | 76 | 10 |
| loop-count-0 | 0 | 6662 | 2240211 | 97845 | 42 | 75 | 11 |
| loop-count-1 | 1 | 15694 | 2260140 | 99217 | 118 | 182 | 20 |
| loop-count-4097 | 4097 | 15832 | 2355249 | 100169 | 274550 | 57514 | 32788 |
| loop-count-5000 | 5000 | 15832 | 2355249 | 100116 | 335051 | 70156 | 40012 |
| loop-nested | 39 | 28050 | 4142757 | 144053 | 781 | 443 | 50 |
| loop-helper | 8 | 22025 | 4224931 | 143207 | 575 | 292 | 53 |
| loop-u32-last | 4294967295 | 11635 | 2472892 | 96989 | 54 | 133 | 11 |

The 4097/5000-candidate programs have equal artifact sizes. Runtime frames still
grow with executed candidates: compact code does not establish constant-memory
execution. The 5000-candidate program rejects the default 16384-frame run and
insufficient reduction/node limits while preserving its previous destination.
The Rust source/JOB test also checks exact and one-below observed runtime
reduction, frame and node allowances on the same emitted program.

Compared with the scalar delivery at `6a1abc2`, compilation overhead changes:

- precedence: 1074092 → 1078047 reductions; 72950 → 77018 nodes.
- body-chunks: 4924137 → 4938665 reductions; 170127 → 174656 nodes.
- function-nested: 3407177 → 3449436 reductions; 130769 → 136302 nodes.

Small-loop outputs agree with the independent expected values and seed compiler.
Large loops agree with an adapted raw seed entry because the legacy flat seed
oracle has its own unrolling ceiling. Reordering function definitions and adding
an unused function containing loops preserves both value and ART1 bytes.
Complete nested-loop table depth passes at the independent DAG-derived limit
and fails one below. Separate exact sequence limits cover index/count slots and
the combined function/loop table. Malformed, wrong-type and unsupported range
forms retain separate diagnostics; the final audit includes the refined parser.

All 985 Trident, 122 Joy and 380 Trisha CPU tests pass, with four
existing Trisha ignores and zero Rust warnings. Trisha verifies 133 fixtures
and 43 manual baselines with unchanged result/cycle rows. Formal audits of the
42 compiler library modules, entry and four fixtures all return UNKNOWN.
These execution results establish no compiler proof.

Noun values and structured raw entry are next, followed by aggregates, constants,
attributes and actual import resolution. Full compiler scale, C2/C3, six-platform
release and native Zheng compiler proofs remain open; Noun stays 128K. The prior
4096-byte whitespace boundary remains an explicit SH4 issue.
