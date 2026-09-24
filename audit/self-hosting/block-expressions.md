# SH3 seed parser: expressions before blocks

The seed parser reserves the outer brace for `if`, `for` and `match` bodies.
Uppercase and qualified constants remain ordinary operands even when the body
is empty or contains a single name. Struct literals retain ordinary syntax
inside parentheses, call arguments, array elements and index expressions.
The canonical rule is in [the grammar](../../reference/grammar.md).

Source: Trident `fd64b73f094ff6a611f8473364f7758484c02fd9`, Trisha
`03b6f9dda71f08000265c929ea55dc32642b050f`.

[Validation](sh3-block-expressions-validation.json) pins source revisions,
commands, test totals, log identities and the existing formal-analysis limit.
The AST regressions check all three headers, binary operands, nested delimited
contexts and rejection of an undelimited struct literal. The shared executable
fixture is compiled in debug and release and run on actual nox and Triton VMs.
It covers both condition outcomes, imported constants, empty blocks, discarded
branch values, calls, arrays, indexing and loops. Its expected scalar results
come from the fixture arithmetic, independently of either lowering.

The recorded commands passed 926 Trident workspace tests, 120 Joy tests and
363 Trisha CPU tests (four existing ignored cases). Trisha verified 133 fixtures
and 43 independent baselines with unchanged result/cycle rows. The rebuilt
installed Joy passed the SH2 runner
`python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --output /tmp/trident-04-block-cli.json`: all 60 commands and 22 observations
completed, and C1 retained its SH2 particle
`20f708f4075539e770536c210497d1231b7da4a1cf7e3026eab4264da0ef13f8`.

The change is one SH3 prerequisite. The complete native compiler language
coverage and self-compilation gates remain open. Noun remains at 128K in the
roadmap because full generator and self-build acceptance are still pending.
This is local development validation, not a release candidate.

## Finding carried into the next delivery

An initial execution fixture added `struct Empty {}` and `let empty = Empty {}`
before reading the function parameter. Triton compilation failed with
`ERROR: unresolved variable 'n'`; the raw nox execution passed. The TIR model
omits a zero-width temporary, so the binding renames the previous stack entry.
Aggregate child pops, call arguments and return cleanup need the same value
invariant; changing only the binding would leave related corruption possible.

The minimal reproducer is:

```trident
program empty
struct Empty {}
fn identity(n: Field) -> Field { let empty = Empty {} n }
fn main(n: Field) { pub_write(identity(n)) }
```

The parser regression retains `Empty {}` syntax coverage. Its executable
fixture uses nonempty structs; zero-width execution remains an explicit open
SH3 task in [the ledger](../self-hosting-progress.md), alongside resolved
halting-call semantics. The failed draft and command are retained in the
validation receipt rather than counted as passing coverage.
