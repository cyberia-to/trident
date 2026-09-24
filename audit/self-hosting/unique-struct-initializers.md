# Reject repeated struct initializers before lowering

Source: Trident `ac2481d62d09a7fe385d4140c388943629861b18`. [Pinned validation](unique-struct-initializers-validation.json)
records every sibling revision and command on the local macOS ARM64 host.

The seed accepted `Point { x:7, x:true, y:9 }` and emitted the first `x`
initializer, silently ignoring the second. The prior-revision witness at
`850525c` builds successfully and its ART1 executes to 7 in the
[CLI receipt](unique-struct-initializers-cli.json). This contradicted the
[exactly-once named-field contract](../../reference/language.md#structs).

Repeated fields now report a diagnostic on the repeated name before lowering,
including shorthand and nested constructors. Required/unknown field checks,
field types and defining-module privacy retain their behavior. Constructor
validation has its own module; expression checking stays within the file limit.

```sh
cargo test --release --locked --offline --workspace
python3 audit/self-hosting/check-struct-initializers.py \
  --joy ../install/bin/joy \
  --before audit/self-hosting/unique-struct-initializers-before.json \
  --output /tmp/struct-initializers-cli.json
```

All 1020 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha cases remain ignored. The 133 fixture result/cycle rows
and 43 manual baselines match the tuple delivery. Three new regressions check
second-name spans, shorthand, nested duplicates and unchanged complete artifacts
for unique reordered literals. The installed receipt runs 9 commands with 5 new
case observations, checks the old invalid artifact and confirms that rejected
`joy build --force` requests preserve the previous program. Post-commit rebuilding
reproduces all three installed binaries byte for byte.

The complete C1 artifact remains identical to the accepted tuple delivery:
`1be92f26a0ed739ff3c2b9fe43dd3bbf6d5ed6b21ae242a8769084535b74b792`. Its 614-command execution corpus therefore has the
same compiler input bytes; that corpus was not rerun for this seed fix.
All 64 formal audits return UNKNOWN. Guest nominal structs/field reads, persistent
writes, imports, full source scale, C2/C3 and native compiler proofs remain open.
