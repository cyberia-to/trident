# Native nominal descriptors and field visibility

Source: Trident `1e36ceb13de342aa5f7bcff4536b9fe3d6252a78`. [Pinned validation](native-compiler-nominal-validation.json)
records all sibling revisions, commands and local macOS ARM64 observations.

`nominal.tri` owns complete module/name identity, ordered typed fields and field
visibility. Its descriptor shares the derived metadata envelope with tuples.
Empty records and up to 32 fields are supported; tuple arity remains 16. Logical
node counting includes repeated child occurrences. Private fields are visible
only from their exact defining owner. Source declarations, constructors and
field reads are the next delivery; this internal API does not close SH3.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --output /tmp/native-nominal-cli.json
python3 audit/self-hosting/check-native-nominal.py --joy ../install/bin/joy --output /tmp/native-nominal-components-cli.json
```

All 1026 Trident, 122 Joy and 380 Trisha CPU tests pass, with zero Rust warnings.
Four existing Trisha cases remain ignored. All 133 fixture result/cycle rows
and 43 manual baselines match the previous delivery. Six new tests check full
canonical descriptor identity, round-trip accessors, exact owner/name lookup,
visibility, invalid fields, 32/33 fields, nested nominal/tuple/Noun metadata,
depth 64/65 and logical-node boundaries 4094/4095/4096.

The [full installed corpus](native-nominal-cli.json) runs 614
commands with 207 observations and retains all 105 earlier positive ART1
identities. The [component runner](native-nominal-components-cli.json) runs 52
commands and checks 17 complete outputs through installed Joy, including
private access from a different owner and the 32nd field. Execution uses the
ordinary 196608-node host allowance for every nominal component case.

The full CLI corpus began on the eventual source commit's unchanged tree and
finished against identical installed binary bytes. After the source commit,
all three owner installs reproduce those binaries byte for byte; rebuilding C1
reproduces the complete executed artifact. C1 particle:
`2b156ea2b8d27818fc5e550b77ad6c5e5f34b8609a5f15b8aa76af07d0296579`.

Costs from the two pinned CLI receipts (old tuple source `850525c`, new source
`1e36ceb`) are charged reductions and lifetime arena allocations:

| Case | Reductions old → new | Allocated nodes old → new |
|---|---:|---:|
| precedence | 1077593 → 1077593 | 88346 → 88395 |
| stack64 | 4727716 → 4727716 | 183836 → 183885 |
| body-chunks | 4947310 → 4947310 | 187857 → 187906 |
| tuple-whole-noun | 2912070 → 2912141 | 126752 → 126807 |
| tuple-nested-types | 6076378 → 6076620 | 199582 → 199643 |
| tuple-digest-helper | 7592228 → 7592228 | 207670 → 207719 |

All 67 formal audits return UNKNOWN. They establish no formal proof. Source
structs/imports, full SH3/SH4, generated compiler profiles, C2/C3, six-platform
acceptance and native Zheng compilation proofs remain open. The Noun layer's
128K roadmap temperature is unchanged.
