# Retained guest packages and validated source handles

Source: Trident `473d20c1f6fbb7e2ac7ed93177612856cf6f1a95`. [Pinned validation](guest-package-validation.json)
records the exact commands, sibling revisions and local macOS ARM64 evidence.

The guest reader retains the admitted module table, original package indices,
explicit options and remaining shared validation allowance. Binary lookup compares
complete paths. Source opening checks remaining source capacity before validating
payload bytes. These APIs prepare module discovery; the compiler still compiles
its selected single source and does not yet resolve guest imports.

Opaque persistent BytesTable values retain validated byte handles between stages.
Only typed Bytes can be appended; there is no raw table constructor or serialization
bypass. Reads require a valid index and spend execution resources without resetting
or repeating the admission allowance. Snapshots survive later appends/byte edits.

Terminal tree branches validate their leaf positions together, preserving every
visit charge, arbitrary pair-shaped occupied values and canonical padding. All
previous exact/one-below collection and single-entry admission vectors still pass.
No source, sequence, arena or evaluator-work limit was raised. The corpus explicitly
selects the existing 60000ms host deadline; Joy still defaults to 30000ms. The long-name
probe records cancellation at the default deadline and the expected arena failure
under the selected deadline, both preserving prior output.

```sh
cargo test --release --locked --offline --workspace
cargo test --release --locked --offline --test native_compiler job_package:: -- --nocapture
python3 audit/self-hosting/run-native-compiler.py --joy ../install/bin/joy --time-ms 60000 --output /tmp/guest-package-final-cli.json
```

All 1120 Trident, 122 Joy and 380 Trisha CPU tests pass with zero Rust warnings;
four existing Trisha tests remain ignored. All 133 baseline rows and 43 independent
manual programs match the explicit-import delivery. All 92 formal audits remain
UNKNOWN; this is executable validation, not a new proof claim.

The package component executes a 4097-entry table including index 4096, separately
from the 4096 source/AST sentinel. The unchanged limits are 100000000 reductions,
786432 lifetime arena nodes and 65536 pending frames:

| Case | Charged reductions | Lifetime nodes | Peak frames |
|---|---:|---:|---:|
| admission | 4348689 | 474606 | 1483 |
| found | 4772603 | 490589 | 1483 |
| missing | 4795959 | 490783 | 1483 |

The [fresh installed corpus](guest-package-cli.json) runs 1192 commands and records
401 observations. All 189 previously positive complete ART1 identities remain
unchanged. Two original wide-record source vectors now compile and execute to 3199:
61-bit and 62-bit paths. Their 63/64/65-bit companions still exhaust the same arena
and preserve prior output files. The JSON records actual costs and exact sources.
The original whitespace/long-name/compiler-scale boundaries remain open.

Source and evidence commits are followed by all-three installs; rebuilt binaries
and complete C1 reproduce the executed bytes. C1 particle:
`681c613b8952777de42675bb03941c0af13d80ce255d23a30b2cff6480a0d260`.

The next unit is strict path parsing and the true guest module graph. Complete
SH3/SH4, generated compiler profiles, C2/C3, six platforms and native Zheng compiler
proofs remain open. Noun stays 128K; no temperature gate is newly closed.
