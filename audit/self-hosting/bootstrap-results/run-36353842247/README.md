# Original v1 native bootstrap: run 36353842247

The original six-job run does not pass SH6. Linux ARM64 completed both clean
repetitions. Intel macOS failed its first compiler execution at the original
one-hour worker deadline. The other four native jobs reached their second
repetition, then GitHub stopped the bootstrap step at its 330-minute limit.
Their outer receipts remain `running`, exactly as uploaded; they are incomplete
results. These failures do not establish an incorrect compiler output.

This is historical evidence from [run 36353842247](https://github.com/cyberia-to/trident/actions/runs/36353842247),
Trident `23691cd2c6885bf25bfc023799552559724dbc2b`, Joy
`2878f4b17dfedf237c6110d7d411bb4824e65103` and nox
`f8047c22cc6075d5171c0fbe520174e78e94ccb6`. The exact original workflow and
runner, all nine pins, direct GitHub job responses/logs and original artifact
metadata are retained. The v1 design runs two repetitions in each native job.
Its compiler profile allows 20B reductions, 1B cumulative nodes, 3145728
resident nodes, 10B collection work, 65536 frames and 3600000 ms per whole build.
Neither these limits nor any uploaded receipt was changed during retention.

The later [twelve-job run](../run-36359020560/README.md) has separate pins,
deadlines, receipts and acceptance. No result here supplies one of its jobs.
The original Intel failure and the original aggregate remain red; successful
archive checks or individual repetitions do not change the matrix result.

The actual aggregate job `108776544043` failed with
`ValueError: bootstrap not passed`. Its artifact `10950680824` retains the
hashes of all six original platform receipts; each matches the downloaded
platform receipt exactly. The original runner's local matrix replay returns
the same error and exit 1. That local replay is separately labeled and does
not replace the original CI receipt. The aggregate ZIP is 694 bytes, SHA-256
`cbb0dde47e76505d0d26057f9a755b8210070ea20de35da598e3b286fc0c9015`.
Its exact restored receipt and job metadata/log are retained under `aggregate/`.
The final run API snapshot and dated timeout/component observations are retained
under `observations/`, each with compressed and raw SHA-256 identities.

## Retained platform results

| Native target | GitHub job / artifact | Original result | Last second-repetition operation when stopped |
|---|---|---|---|
| Linux ARM64 | `108717528179` / `10950491263` | Both repetitions passed | Completed both compiler corpora and final clean-source checks |
| Linux x64 | `108717528128` / `10950695650` | GitHub step timeout; outer receipt `running` | `C2(S) -> C3` |
| Windows x64 | `108717528125` / `10950546387` | GitHub step timeout; outer receipt `running` | Actual C2 constant-linking corpus; C2 main corpus completed |
| Windows ARM64 | `108717528114` / `10950372498` | GitHub step timeout; outer receipt `running` | Actual C3 main corpus; all six C2 corpora completed |
| macOS ARM64 | `108717528157` / `10950293054` | GitHub step timeout; outer receipt `running` | Actual C3 main corpus; all six C2 corpora completed |
| Intel macOS | `108717527925` / `10943644187` | First whole build failed; no C2 | See the unchanged [original deadline receipt](intel-deadline/README.md) |

Each of the four step-timeout bundles records its first repetition as passed.
They are nevertheless excluded from the original runner's successful-component
comparison because the complete two-repetition report did not finish.

For Linux ARM64, the unchanged original v1 `compare()` accepts the restored
report. Each actual C2 and C3 in each repetition passed all six corpora:
547 observations, 1816 commands and 119 separately labeled raw Rust reference
builds. All four compiler artifacts have SHA-256
`76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
Original native Cargo build logs have zero Rust compiler warnings on these
five newly retained targets. GitHub action notices remain in their raw logs.

## Exact archive and replay

The reviewed [archive tool](../archive-tool/README.md) imported the five new
platform ZIPs and failed aggregate into this run's separate `platform-store/`. Every restored file
was compared byte for byte with its original ZIP member, including partial
receipts and outputs. The existing `intel-deadline/` files are unchanged and
retain the complete sixth platform ZIP separately.

| Receipt directory | Restored files | Raw bytes | Original ZIP SHA-256 |
|---|---:|---:|---|
| `linux-arm/` | 14550 | 199779654 | `21484b9da01c96093f810cfaa8dc4700f8d4bd19bca98585e8942453fc2055c6` |
| `linux-x64/` | 7824 | 143468512 | `481d9ce9f052b474768273b260cfd84f0f1f767f280f0fa4a06b1e73a301f26e` |
| `windows-x64/` | 10115 | 175051195 | `343000e4e791a10d03cc523f03e15172b9976ee60a6ddaf04068a2a5446da46d` |
| `windows-arm/` | 13043 | 190470043 | `654d2468c94bb481f3ef5991f8897cfddb1a5c8bbaa11b208ce0128d80991574` |
| `mac-arm/` | 13126 | 190634767 | `0456d2888358d371c91e10653f3ca3136fbe222fa75bf5865a9161da3583607b` |

The store index SHA-256, including the five new platforms and failed aggregate, is
`9f20e7f20437abd0ba7fa4fa521215bafac29ff5a62059ab9454833e15d89860`.
Each platform's `files.json` binds lossless gzip files for its exact import and
restore commands, stdout/stderr, retention checks, direct job metadata/log,
original artifact metadata, frozen runner/workflow and measured collector.
The sources of those operations are `retain-component.py` and `retain-aggregate.py`;
all receipts record their helper source SHA-256, and component receipts also
record the checkout revision. Original ZIP containers remain immutable
under `measurements/ci-readonly/` in the recording workspace; the store retains
their complete raw file trees and metadata, not a reconstruction of ZIP bytes.

From the repository root, restore into a fresh directory:

```sh
python3 audit/self-hosting/bootstrap-results/archive-tool/archive.py restore \
  --store audit/self-hosting/bootstrap-results/run-36353842247/platform-store \
  --output /absolute/fresh-original-results \
  --index-sha256 9f20e7f20437abd0ba7fa4fa521215bafac29ff5a62059ab9454833e15d89860
```

The frozen runner SHA-256 is
`805af512ad48b99fc77b2e476ef6b1e67e7068ca4c4304a59a366c06110cb0f1`;
the original workflow SHA-256 is
`7af25add1b3a94e35bf9964db80cb3bc798ff5d6446e06cb37fc19bf5729b975`.
Their exact bytes were checked against Git commit `23691cd`. Decompress
`linux-arm/original-runner.py.gz` into a separate fresh file to replay its
`compare([(root, load(root / "receipt.json"))])` on the restored Linux ARM64
directory. This is a successful-component comparison only, not `--matrix`
acceptance. No host build or guest compiler execution occurs during replay.

`check-retention.py` verifies the 75 operation/provenance gzip files, measured
collector identities, seven fresh-output/optimized-Python/path guards and the
unchanged Intel receipt. Its ten commands passed; the exact command arrays and
outputs are retained in `observations/retention-checks.json.gz`. These checks
validate this historical retention unit, not the failed bootstrap matrix.
