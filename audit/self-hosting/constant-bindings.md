# Native qualified names and frozen constant bindings

Source: `8f22f12c06a92a4ae5ac139a04126dc931dbf3ab`, implementation `d8e4887`.
The [receipt](constant-bindings-validation.json) pins commands, sibling revisions
and local macOS ARM64 results.

Qualified-name parsing keeps the caller's complete byte span and final member
token. Only the normalized module prefix uses the 255-byte package-name limit;
long member identifiers retain their complete source span. Comments/whitespace
are handled by the lexer. Incomplete paths, lexical failures and prefix-capacity
errors retain distinct diagnostics.

Frozen exports carry stable IDs, defining owner/name, exact type, normalized
value and a separate terminal literal owner/span. Lookup compares the caller's
member against the defining source. Direct uses are visited in source order;
a later same-basename module replaces only members it exports. Full aliases,
short aliases, repeated uses and foreign literal provenance are tested.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/check-constant-bindings.py --joy ../install/bin/joy --output /tmp/native-constant-bindings-final-cli.json
```

All 1135 Trident, 122 Joy and 380 Trisha CPU tests pass without warnings; four
existing Trisha tests remain ignored. All 133 baseline rows/43 manual programs
are unchanged. All 103 formal audits remain UNKNOWN.

The [installed component corpus](constant-bindings-cli.json) runs 27 commands
and checks eight complete input/output trees. Input producers construct explicit
Nouns which an independent decoder checks before component execution. Fixtures
supply frozen bindings; they do not parse dependency declarations or compile an
importing program. Exact 255-byte owners, long members, source-order replacement,
non-transitive visibility and terminal origin are included.

All three final binaries and complete component artifacts reproduce after
commit. C1 and graph artifacts are unchanged from the indexed-read source
`f4bbc1c` and its accepted graph integration `8688524`; new helpers are not yet
linked into `compiler/nox/main.tri`. Existing validation charges and public
quotas remain unchanged.

The next step checks all dependency declarations, publishes final public bindings,
and connects qualified constants to C1. Guest imports, whole compiler scale,
C2/C3, six platforms and native Zheng proofs remain open; Noun stays 128K.
