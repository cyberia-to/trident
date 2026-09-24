# Native compiler: Noun values and structured entry

Source: Trident `c5d1901f5be44caf097cc33e7e09132538ce7fa7`, Joy `d662ef9f07b6904783c98fd215f75764dfca5254`,
nox `c9f7486a74fe81bfc194b598da40f6343ecb2ef1`, Trisha `908d5e22a669e4aa0b6c02b5e9134e67c1a25675`.
[The pinned validation](sh3-native-nouns-validation.json) records all inputs,
commands and local macOS ARM64 observations.

The compiler executing inside nox now compiles opaque Noun values and
`main(input:Noun)->Noun`. Its emitted program receives the original complete
runtime subject in slot0, including in singleton, function-table and loop paths.
The six existing direct seed builtins construct atoms/pairs, project head/tail,
check atom conversion and compare native identities. Scalar-entry programs may
also use Noun locals and helpers. Previous successful ART1 identities remain
unchanged. The [subset contract](../../reference/self-hosting.md#native-compiler-subset-contract)
keeps operator restrictions, actual import resolution and generated profile
`(1,1)` separate; this increment emits raw `(0,0)`.

The original subject is composed into a balanced frame with quoted zero padding
before dispatch. Noun values remain whole immutable subtrees. Head/tail on atoms
and atom conversion on pairs compile when well typed, then trap at execution.
Discarded expressions and unused arguments retain those checks. Unselected
branches do not execute them, and arguments execute once in source order.

## Execution acceptance

After rebuilding the three installed owners from the committed implementation:

```sh
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-nouns-cli.json
```

[The installed receipt](native-nouns-cli.json) records 476 commands and
161 observations. C1 is built before fresh case files. Joy packs each exact
source, runs compilation in nox, publishes the returned ART1 and executes that
separate program. New Noun compiler jobs request 786432 nodes. Their emitted
programs run under Joy's default execution allowances. The independent output
reader checks complete nested values; identity entry additionally reproduces
its input's canonical bytes exactly. Runtime failures preserve previous output.
C1 particle: `7f5d9ee04ca7bf505c7d982cf2ff547288af18365e3e1712d5675968fd8602b2`.

The following measurements use that command and those source revisions.
Allocated nodes count lifetime allocation; they are not reserved memory or RSS.

| Case | Program bytes | Compile reductions | Compile nodes | Run reductions | Run nodes | Run frames |
|---|---:|---:|---:|---:|---:|---:|
| precedence | 1536 | 1079517 | 78314 | 5 | 22 | 3 |
| body-chunks | 5804 | 4943536 | 176330 | 92 | 82 | 15 |
| function-nested | 5871 | 3455329 | 138125 | 38 | 76 | 10 |
| loop-count-5000 | 15832 | 2357556 | 101550 | 335051 | 70156 | 40012 |
| noun-input | 1745 | 1020783 | 70697 | 8 | 25 | 4 |
| noun-padded-frame | 9374 | 2521437 | 99509 | 66 | 129 | 11 |
| noun-share | 1939 | 1748525 | 82653 | 10 | 28 | 4 |
| noun-helper | 3088 | 1977727 | 92777 | 18 | 40 | 6 |
| noun-loop | 16373 | 3672741 | 122185 | 260 | 228 | 36 |

Compared with the loop delivery at `7e5a274`, compilation overhead changes:

- precedence: 1078047 → 1079517 reductions; 77018 → 78314 nodes.
- body-chunks: 4938665 → 4943536 reductions; 174656 → 176330 nodes.
- function-nested: 3449436 → 3455329 reductions; 136302 → 138125 nodes.
- loop-count-5000: 2355249 → 2357556 reductions; 100116 → 101550 nodes.

Full source/JOB tests compare complete output with independently constructed
nested/shared values and the Rust raw seed. Cases cover padded input frames,
helper and loop transport, persistence, final callable replacement, local name
collisions and both directions of the final main signature change. Distinct
runtime traps establish evaluation order; trace rows establish once-only
computed arguments. Exact and one-below runtime reduction/frame/node limits
and sequence capacities execute. Independent DAG-derived depth boundaries
cover frame heights0/1/2/3 in standalone and deep quoted-table paths.

All 993 Trident, 122 Joy and 380 Trisha CPU tests pass, with four
existing Trisha ignores and zero Rust warnings. Trisha verifies 133 fixtures
and 43 manual baselines with unchanged result/cycle rows. Formal audits of the
43 compiler library modules, entry and five fixtures all return UNKNOWN.
These execution results establish no compiler proof.

The first CLI attempt stopped on an incorrect acceptance assertion expecting
Rust Outcome Debug formatting. Joy had produced the correct AxisError. Runner
commit `c5d1901` asserts exact AxisError/TypeError diagnostics; the receipt above
comes from the repeated complete acceptance. Production source is `4dbd03e`.

Next are bounded aggregate types, tuples and Digest identity, then nominal
structs/fixed arrays, constants, attributes and actual import resolution.
Full compiler scale, C2/C3, six-platform release and native Zheng compiler proofs
remain open; Noun stays 128K. The prior valid 4096-byte whitespace boundary
remains an explicit SH4 issue.
