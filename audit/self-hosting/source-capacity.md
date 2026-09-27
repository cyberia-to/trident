# Independent native source byte capacity

Source `40a86dee44e8ea5132c510950a0e4876a96bbadb`; Joy `a15adb7310db5b159b4e72ce0dfd0fea2ff01c7d`; Trisha `f5c94f5ec5d6d52943c8f416f343513b9d33c85e`.
The [receipt](source-capacity-validation.json) pins commands, exact source hashes,
all sibling revisions and gate logs. These are local macOS ARM64 measurements.
Frozen pre-commit inputs match the named commit and its clean reinstall reproduces
all three binaries, C1 and the graph artifact. Reinstalls use the existing build
cache; fresh-build determinism belongs to the accepted codec prerequisite.

The guest separates a 65536-byte per-file ceiling from 4096-entry internal tables
and absent-ID sentinels. Reached bytes share the explicit JOB1 source allowance.
Complete token/name/UTF-8 scans and original diagnostic offsets cross the old
4096-byte boundary; parser work bounds and runtime limits stay independent.
See the [source review](source-capacity-review.md) and [job contract](../../reference/self-hosting-jobs.md).

| Installed corpus | Commands | Observations |
| --- | ---: | ---: |
| [full](source-capacity-full-cli.json) | 1198 | 402 |
| [capacity](source-capacity-capacity-cli.json) | 25 | 7 |
| [intrinsic](source-capacity-intrinsic-cli.json) | 158 | 37 |
| [profile](source-capacity-profile-cli.json) | 65 | 21 |
| [types](source-capacity-types-cli.json) | 111 | 24 |
| [callable](source-capacity-callable-cli.json) | 149 | 32 |
| [constants](source-capacity-constants-cli.json) | 141 | 31 |
| [graph](source-capacity-graph-cli.json) | 39 | 12 |

All 237 previously successful observations retain their 204 distinct ART1
identities. Ordinary fixtures keep their original public job quotas. Exact
resource boundary probes recalibrate and retain one-below rejection. The original
4097-byte negative fixture now crosses source admission but exhausts its unchanged 196608-node
arena; no RES1 or program is published. The module-graph 4097-byte fixture
completes under its original allowance. All other graph outputs are unchanged.

A long comment prefix and a dependency over 4096 bytes compile to the exact
baseline program and Joy executes result 13. Error spans point to the original
bytes. Exact 65536-byte component scans succeed under explicitly larger test
limits (1B reductions, 12582912 nodes); the complete exact-size JOB1 exhausts the
supported 3145728-node arena. Excess 65537 returns capacity 7. Component limits do
not establish worker support. The fixed nested-64 CLI workload retains 196608
nodes: compact exact ASCII whitespace classification recovers the footprint
regression without raising its quota. Every byte and wide U32 values are checked.

All seven owner gates pass: 1203 Trident, 123 Joy, 380 Trisha tests; four existing
ignored Trisha tests and zero Rust warnings. All 133 baseline rows / 43 manual
programs are unchanged. All 120 formal verdicts remain UNKNOWN. C1 has 95435 DAG
entries and SHA-256 `4aed7fc83be96156fcb65c3bbb369c192ad27f894ab78a030e3588056a66d112`.

The [exact closure](source-capacity-closure.json) contains 94 modules, 455 functions
and 345791 bytes, including comments. `probe-native-closure.py` checks the
inventory, packages exact files, and executes C1 through Joy while preserving
rejection evidence. Its full-source JOB1 is admitted by the host; execution
exhausts the 100M reduction budget without RES1 or C2. Full guest discovery and
compiler-scale execution remain open. The receipt retains prefix/resource probes,
initial failures, discarded source variants and the invalidated mid-install CLI
run; only the final frozen run qualifies this delivery.

Reproduce from these pinned sibling checkouts (the probe exits 1 at the recorded
runtime boundary):

```sh
cargo run --release --locked --offline --example selfhost_inventory -- --root . --entry compiler/nox/main.tri --output /tmp/native-closure.json
joy build compiler/nox/main.tri --emit artifact --artifact-profile compiler-job -o /tmp/c1.dag
python3 audit/self-hosting/probe-native-closure.py --joy /absolute/path/to/joy --compiler /tmp/c1.dag --inventory /tmp/native-closure.json --output /tmp/native-closure-probe.json
```

Diagnostic prefix compilers localize the barrier: entry admission completes,
while module discovery alone exhausts the same reduction budget. These smaller
formulas are diagnostic components; their absolute costs are not C1 costs.

Next: reduce repeated byte/tree work during complete source admission and UTF-8
scanning, then measure discovery and each later compiler stage again. C2/C3,
six CPU platforms and native Zheng proof gates remain open. Noun stays 128K.
