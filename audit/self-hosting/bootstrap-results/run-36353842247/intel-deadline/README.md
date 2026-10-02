# Intel macOS: unchanged deadline rejects the first clean self-build

[CI run 36353842247](https://github.com/cyberia-to/trident/actions/runs/36353842247),
[job 108717527925](https://github.com/cyberia-to/trident/actions/runs/36353842247/job/108717527925),
Trident `23691cd2c6885bf25bfc023799552559724dbc2b`, Joy
`2878f4b17dfedf237c6110d7d411bb4824e65103`, nox
`f8047c22cc6075d5171c0fbe520174e78e94ccb6`. All nine origin pins, commands,
native toolchain, source inventory and build logs are retained in the exact
GitHub artifact ZIP. Its SHA256 matches the retained GitHub API digest.

The native x64 build succeeds without Rust compiler warnings. Repetition 1's
actual `joy run-artifact` for `C1(S1) -> C2` exits 1 with
`CompactionFailure { kind: Execution(Cancelled) }` after 3600087115845 ns.
Its explicit host deadline is 3600000 ms. No C2 is published. The command and
all counters are in `x86_64-apple-darwin-failure.json` and the original
`repeat-1/c2-step.json` inside the ZIP. This is a failed SH6 platform result.

The C1, JOB1 and source-inventory SHA256 identities exactly match the successful
[local first self-build](../../../lexer-bootstrap/README.md). The failure
therefore demonstrates that the existing wall-clock allowance does not cover
this native CI machine with these inputs. It does not establish a compiler
output mismatch, nor guarantee completion under any larger allowance.

The following delivery declares a new bounded whole-compiler host deadline and
separates the two clean repetitions into independent native CI jobs. Ordinary
corpus quotas and guest computational limits remain fixed. Acceptance requires
new actual successful results; this original failed run is never relabeled.

Retention check, from the repository root:

```sh
python3 audit/self-hosting/bootstrap-results/run-36353842247/intel-deadline/verify.py
```

`files.json` checks every original retained file. The ZIP remains byte-exact;
the raw GitHub job log is losslessly gzip-compressed with a fixed timestamp.
The API status snapshot records the other jobs still running at that observation.
GitHub action Node deprecation notices are preserved separately from Rust
compiler warnings. No later outcome is inferred from that partial snapshot.
