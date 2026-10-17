# Native source preparation on six platforms

[CI run 36400752982](https://github.com/cyberia-to/trident/actions/runs/36400752982),
attempt 1 at `85ab2d3f33d4e5d5e057600782aeb99bed2dfeb2`, passed the supplied
compiler source-preparation guards on macOS, Linux and Windows, each on ARM64
and x64. The workflow invokes `check-source-transport.py --target TARGET`,
which runs the same 21 named cases in ordinary and optimized Python 3.13.15.
It verifies native interpreter architecture and retains every outcome and skip.

| Native targets | Ordinary passed / skipped | Optimized passed / skipped |
|---|---:|---:|
| macOS ARM64 and x64, each | 21 / 0 | 21 / 0 |
| Windows ARM64 and x64, each | 21 / 0 | 21 / 0 |
| Linux ARM64 and x64, each | 20 / 1 | 20 / 1 |

There are 21 distinct tests and 252 case selections across the six targets
and two modes: 248 passed and four were skipped. All skips are the physical
case-alias test on Linux's case-sensitive filesystems. Both Windows runners
executed the symlink tests; neither needed the explicit WinError1314 skip.
These are byte-transport tests. Full compiler self-builds and their corpora
are accepted through the separate original SH6 matrix.

The independent review retains original final/attempt REST, six direct jobs,
their logs, all original artifact API responses and ZIPs, the full run-log ZIP,
and 101 exact Git source blobs including the 94 compiler modules. It binds
each job to its platform, commit, run/attempt and exact worker arguments, then
compares the JSON outcomes with all named unittest log lines. The root also
compared all retained source blobs directly with Git and replayed the checker
after restoring the retained archive into a fresh directory.

The archive contains 198 files and 5262843 raw bytes, including the original
index. That index lists 197 other files totaling 5234770 bytes. The archive is
4314836 bytes, SHA256
`d4ff213725b9c9605eecb4d2aba5ecdfa7786847ebbca16b5c7b24b3ac3102bb`.
`retention.json` binds every member and the exact original collection command's
source. `collect.py` preserves that one-time local measurement recipe, invoked
as `python3 -B retain-source-transport-ci.py`. Replay uses:

```sh
python3 -B audit/self-hosting/bootstrap-results/source-transport-platforms/replay.py \
  --output /absolute/fresh/replay
```

Choose an existing ordinary parent for the fresh output directory. Replay
checks bounded input sizes, exact archive bytes and every member before
writing, then runs the archived independent checker. It invokes no compiler
or native test. The root's corrected restore and checks passed; oversized
archive/index and physical case-alias output guards rejected before output.

Local evidence under `local/` preserves the initial and corrected ordinary/
optimized runs, native-target rejection, workflow validation, replay guards,
and postcommit install log. Review tightened the test driver to clear inherited
`PYTHONOPTIMIZE`; the corrected local run started with value 2 and still
executed modes 0 and 1. The helper and frozen compiler inputs stayed unchanged.
