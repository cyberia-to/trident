# Shared typed seed constants

Source: Trident `2f6b0ef0a615c1cfffcce186e5d1b936739ad704`. [Pinned validation](typed-constants-validation.json)
records commands and all sibling revisions on local macOS ARM64.

The Rust seed previously dropped nonliteral initializers, including unused
unknown names and calls. It rejected constant aliases. A public U32 constant
followed by a private Field declaration could export the old type while nox
emitted the new value. Literal scans could also change dimension bindings
between signature checking and lowering. The [prior probes](typed-constants-before.json)
retain original source text, commands and binary identities; their installed
guest library was in development and is identified separately from accepted C1.

One shared resolver now freezes the final active value, declared type and
visibility before signatures. Every active initializer is checked, including
replaced declarations. Integer literals and exact-type references resolve in
the defining scope; forward references work and cycles diagnose. Raw integers
remain available for checked dimensions, while runtime Field values normalize
at emission. Exports, generic specialization and both lowerers share the result.
Private owner layout dimensions become literal extents without exposing private
constants to expressions. TIR's direct builder keeps explicit external Field
bindings and observes its final cfg flags.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/check-typed-constants.py --joy ../install/bin/joy --trisha ../install/bin/trisha --output /tmp/typed-constants-cli.json
```

All 1058 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha cases remain ignored. All 133 fixture rows and 43 manual
baselines match the array delivery. Ten new tests cover raw values/types,
forward/cyclic/long chains, cfg, final export visibility, signature sizes,
direct backend errors, private generic dimensions and native/TIR parity.

The [installed receipt](typed-constants-cli.json) contains 61 commands and 20
case observations. Ten positive programs execute on both nox and Triton,
including imported array-return and record layouts; ten rejected sources
preserve earlier output files on both warriors. The CLI started on the eventual
source commit's unchanged production tree. Post-commit installs reproduce all
three executed binaries byte for byte. Complete C1 remains identical to the
accepted array corpus: `d9c86de1d644e7826a75ccbf264c4df4dba36306c3173f18935798a523f8428e`. That 851-command guest
corpus was not rerun for this Rust-only resolution change; its compiler ART1
input bytes are identical. All 79 formal audits are UNKNOWN and establish no proof.

Guest constants/attributes/asserts/imports, full SH3/SH4, generated profiles,
C2/C3, six platforms and native compiler proofs remain open. Noun stays 128K.
