# Indexed reads preserve compiler arena capacity

Source: `ceaf7c8da5031d55faaabe7a268e928b58fbfbd7`, implementation `f4bbc1c`.
The [receipt](native-indexed-reads-validation.json) records commands, sibling
revisions and local macOS ARM64 evidence.

`std.nox.tree.get` returns the complete leaf at its final branch. This avoids
allocating another loop continuation solely to exit. Canonical tree shape,
index checks, opaque pair leaves and `height + 1` admission charges are preserved.
Independent sparse trees exercise every U32 height and adjacent power boundaries,
including U32::MAX and rejection at the declared length.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --time-ms 60000 --output /tmp/native-indexed-reads-final-cli.json
python3 audit/self-hosting/check-guest-module-graph.py --joy ../install/bin/joy --output /tmp/native-indexed-reads-graph-cli.json
```

All 1131 Trident, 122 Joy and 380 Trisha CPU tests pass without warnings; four
existing Trisha tests remain ignored. All 133 baseline rows/43 manual programs
are unchanged; all 98 formal audits remain UNKNOWN.

The [installed corpus](native-indexed-reads-cli.json) records 1194 commands and
401 observations. All 232 prior successful compilation observations preserve complete ART1 identities,
including programs with expected runtime failures and exact-quota probes. They
represent 200 distinct programs; the
original 63/64-bit record-write cases also compile and execute to 3199. Their
sources and 786432-node arena ceiling are unchanged. The 65-bit case still fails
with Unavailable and preserves the old output. Measured costs for accepted cases:

| Path bits | Charged reductions | Lifetime nodes | Peak frames |
|---|---:|---:|---:|
| 61 | 36604799 | 745472 | 1351 |
| 62 | 37186743 | 755787 | 1351 |
| 63 | 37468590 | 765561 | 1351 |
| 64 | 38373815 | 776311 | 1351 |

The prior 62-bit case used 785399 lifetime nodes; it now uses
755787 under the same limit. Source4096-arena,
assignments31-arena and calls64-arena retain explicit Unavailable failures in
the original 196608-node arena. No source, sequence, arena or evaluator quota
changed. The runner retains the explicit 60000ms host deadline; Joy defaults
remain unchanged.

The [graph corpus](native-indexed-reads-graph-cli.json) adds 39 commands and
12 observations. Complete graph outputs and shared admission allowances match
the accepted graph component. All three post-commit binaries and complete C1
reproduce the executed bytes; C1 particle is `35429c19be900bf8de42e40b33ab5e4a8b697ff94757188bc3a89c44c99e8303`.

Guest import linking, full compiler source/resource scale, generated compiler
profiles, C2/C3, six platforms and native Zheng proofs remain open. Noun stays 128K.
