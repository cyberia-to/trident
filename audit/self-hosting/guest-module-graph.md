# Native guest module graph components

Source: Trident `096f68cc2d26b05b1af82e73f922de3ca1b9d3bd`, component implementation `0c4a72b`.
[Pinned receipt](guest-module-graph-validation.json) records exact commands,
sibling revisions and local macOS ARM64 results.

The graph executes inside nox over an admitted JOB1 table. It opens and checks
UTF-8 once per reached source, retains typed source/name handles and original
package indices, and preserves every repeated use with its original byte spans.
Missing dependencies and cycles report the caller; reached name/encoding errors
report the dependency. Unused malformed source bytes remain unparsed.

Discovery uses bounded iterative DFS. A separate lexical-root, source-order DFS
reproduces seed ordering; vectors distinguish it from entry-only traversal and
lexical-ready Kahn ordering. Headers require an identifier after every dot,
normalize comments/whitespace and reject invalid lexical boundary tokens.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/check-guest-module-graph.py --joy ../install/bin/joy --output /tmp/guest-module-graph-final-cli.json
python3 audit/self-hosting/check-strict-module-paths.py --trident ../install/bin/trident --joy ../install/bin/joy --trisha ../install/bin/trisha --output /tmp/guest-module-graph-strict-cli.json
```

All 1130 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha tests remain ignored. All 133 baseline rows and 43 manual
programs are unchanged; all 98 formal audits remain UNKNOWN.

The [graph CLI corpus](guest-module-graph-cli.json) runs 39 commands/12 observations;
the [strict-path corpus](guest-module-graph-strict-cli.json) adds 35 commands/12
observations on both warriors. The graph corpus packs sources through Joy, wraps
the admitted JOB1 as raw fixture input with a separately executed Noun adapter,
and runs the graph component through Joy. It checks complete typed outputs and
explicit options. This is component execution; it does not emit a linked program.

The guest retains its common 4096 reached-source byte cap and unchanged
100000000 reductions, 786432 lifetime nodes and 65536 frames. The runner explicitly
selects supported host caps --modules 65536 and --time-ms 60000; Joy defaults remain
4096 modules and 30000ms. The 4097-entry package reaches index 4096 while compact IDs
remain 0/1. Source 4096 succeeds; source 4097 reports capacity 7 at the target index.

| Case | Charged reductions | Lifetime nodes | Peak frames |
|---|---:|---:|---:|
| diamond-repeated | 1628229 | 67397 | 591 |
| source 4096 | 61789492 | 746648 | 32938 |
| source 4097 | 358946 | 30856 | 333 |
| index 4096 | 5064110 | 512346 | 1482 |

The complete C1 artifact remains identical to the accepted 1192-command
[guest corpus](guest-package-cli.json), which is reused by byte identity.
The graph is not yet linked into compiler/nox/main.tri. Complete body checking,
qualified constants/functions/types, legacy import remaps and generated compiler
profiles remain open. Full compiler scale, C2/C3, six-platform and native Zheng
proof gates stay open; Noun stays 128K.
