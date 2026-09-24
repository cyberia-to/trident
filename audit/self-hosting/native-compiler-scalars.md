# Native compiler: checked U32 scalars

Source: Trident `6a1abc25d06810456c217ab2bf120a379952ef48`, Joy `d662ef9f07b6904783c98fd215f75764dfca5254`,
nox `c9f7486a74fe81bfc194b598da40f6343ecb2ef1`, Trisha `908d5e22a669e4aa0b6c02b5e9134e67c1a25675`.
[The pinned validation](sh3-native-scalars-validation.json) records all sibling
inputs, commands, failed drafts and local macOS ARM64 observations.

The compiler running inside nox now recognizes U32 locals, parameters and
results; unsigned comparison and bitwise AND; and unqualified `as_u32`,
`as_field` and `sub`. This closes the scalar increment of SH3.
[The subset contract](../../reference/self-hosting.md#native-compiler-subset-contract)
retains explicit typing, source precedence and final user-function overrides.
Eight-byte `as_field` names compare source bytes rather than the lexer's shorter
packed identifier. Builtins use checked owned arguments without entering the
user call graph or code table.

Conversion from Field evaluates its argument once. A value outside U32 compiles
successfully and traps with InvZero when the emitted program executes. This
also holds for discarded values, unused local initializers and unused function
arguments. An unselected conversion branch does not execute. Widening preserves
the atom; subtraction remains Goldilocks arithmetic. Decimal values normalize
as Field before conversion. The unsupported-type sentinel is distinct from U32.

## Execution acceptance

After rebuilding the three installed owners from the committed implementation:

```sh
python3 audit/self-hosting/run-native-compiler.py \
  --joy ../install/bin/joy --output /tmp/native-scalars-cli.json
```

[The installed receipt](native-scalars-cli.json) records 326 commands and
112 observations. C1 is built before case files exist. Joy packs exact
source bytes, executes the compiler inside nox, extracts its ART1, and runs
that separate program. The new scalar jobs explicitly request 786432 arena
nodes; existing jobs retain their 196608 allowance. Every previous positive
program keeps its artifact identity; C1 itself changes to
`9354f6df23f5f2da39e8f2a784d1c80a9bdec879cbf302358d13462e6ae1eb20`.

The source/JOB corpus compares Rust-seed execution and independent expected
values: zero/maximum U32, Field modulus boundaries, mutation/shadowing, high-bit
masks, operator precedence, typed calls and callable/local name collisions.
Wrong arity/types and unsupported qualified names are rejected. Failed compiler
jobs and failed emitted programs preserve old destination files with force.
A trace check observes one subtraction evaluation inside the conversion guard.

| Case | Program result | Charged reductions | Allocated nodes | Peak frames |
|---|---|---:|---:|---:|
| precedence | 14 | 1074092 | 72950 | 509 |
| typed-local | 7 | 1258174 | 70077 | 565 |
| body-chunks | 9 | 4924137 | 170127 | 731 |
| function-nested | 34 | 3407177 | 130769 | 710 |
| scalar-maximum | 4294967295 | 1586029 | 76699 | 653 |
| scalar-mask | 4278190080 | 2135905 | 89659 | 653 |
| scalar-typed-call | 9 | 2134945 | 93907 | 653 |
| scalar-unused-argument-overflow | InvZero | 1968113 | 89907 | 653 |

Measurements above use the installed command and pinned sources. Allocated
nodes measure lifetime allocation, not reserved memory or RSS. Against the
functions delivery at `5339030`, compiler costs changed:

- precedence: 1071209 → 1074092 reductions; 71368 → 72950 nodes.
- body-chunks: 4906212 → 4924137 reductions; 168551 → 170127 nodes.
- function-nested: 3400062 → 3407177 reductions; 129089 → 130769 nodes.

Added lexical/type checks and builtin call dispatch increase compiler work;
unchanged executable identities do not imply unchanged compilation cost.
Trident 978, Joy 122 and Trisha 380 CPU tests pass, with
4 existing Trisha ignores and zero Rust warnings. Trisha verifies all
133 fixtures and 43 manual baselines; all result/cycle rows match the heap
arena delivery. Formal audits of 38 compiler library modules, the entry and
three fixtures all return UNKNOWN. These runs establish no compiler proof.

## Boundaries and next work

An independent reader measures the emitted conversion guard's complete DAG
depth. Exact and one-above RES1 depth allowances pass; one below returns
resource diagnostic 7, for both shallow and deeper argument formulas. This
component probe uses an explicit larger arena because full JOB depth admission
also bounds C1. Nested builtins separately test exact/insufficient expression
and argument sequence limits.

Bounded loops are next, followed by Noun/aggregates, constants/attributes and
imports. Full compiler source scale, including the previous 4096-byte whitespace
boundary, remains SH4 work. Full SH3/SH4, C2/C3 self-build, six-platform release
acceptance and native Zheng compiler proofs remain open. Noun stays 128K.
