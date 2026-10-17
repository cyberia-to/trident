# Intel macOS bootstrap-step timeouts

Both Intel repetitions in run `36359020560`, attempt 1, head
`c17bd0371c11746f46e20222c48cae2ab08be79d`, completed both self-builds and
the fixed-point check. Each actual C2 passed all six corpora: 547 observations
and 1816 commands. GitHub terminated each bootstrap step after 330 minutes
during the actual C3 corpora. The original outer receipts remain `running`;
the direct job API and logs record final failure.

| Repetition | Artifact ID | ZIP bytes | Worker C1→C2 seconds | Worker C2→C3 seconds |
|---|---|---:|---:|---:|
| 1 | `10951539650` | 39909321 | 4730.908499 | 4576.039055 |
| 2 | `10952109432` | 41692736 | 4718.821175 | 3937.556209 |

The times are Joy `execution.execution.elapsed_micros`, converted to seconds.
Separate producer command wall times remain in the validation receipts.
Repetition 1 timed out at `2026-09-28T05:03:45.9538530Z`, during C3 main,
with 373 observations and 1123 commands recorded. Repetition 2 timed out at
`2026-09-28T05:03:27.0713420Z`, during C3 intrinsics, with 10 observations
and 51 commands recorded. Its C3 main (402), constants (31), function imports
(32) and type imports (24) had passed; generated-profile had not started.
Unfinished-corpus progress is retained without accepting the corpus.

Both Intel Cargo builds per repetition have zero Rust warning lines.
`independent/intel-partial-validation-strict.json.gz` records the completed
corpus checks, exact active commands, raw timeout lines and source bindings.
`operations/validation.json.gz` independently rechecks the restored trees:
both completed self-builds match local S1, including all execution fields
except elapsed time.

The bound identities are:

- C1: `5728e07a37e111f88166c471ad338f8947b74dfdc842ed1ffa9fa0886c005fda`.
- Actual C2 and C3: `76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8`.
- Inventory: `d35d263c7f9f27cbe7ea760a34393105ae8140dc6ab84d484151b6fe04960571`.
- C1 JOB1: `a0ee6ef9b9ecaa3007feb4d4e64a9cd7d53228995a13ab75454b47a693a32630`.
- C2 JOB1: `3474e5583e7b3ac36bdd526435bb2ae584691774a009e29ca02407c22589f13d`.

Original ZIP SHA-256 values are
`d59c9c14d54ba5ff8c6d4471582717fdeb34710dd216918ab9d020c02dad2f04`
and `ba20a764af70bc5b830f2a9c2b66c6f342ddc3e6b802798868b668f0680addec`
for repetitions 1 and 2. Immutable downloads remain in
`measurements/ci-split-readonly/artifact-<ID>.zip`; the durable platform store
retains every original uncompressed file and exact API metadata. It cannot
reconstruct the original ZIP container. There are 5770 and 6884 retained
files, respectively, totaling 91708175 and 96205160 raw bytes.

The reviewed archive helper imported the two artifacts sequentially, then
restored all twelve to a fresh directory. Every restored file set and byte
matched the original ZIP. The prior ten entries and all prior stored files
were unchanged. The final index is
`e63844ab6f5e79713f0b073fc70f22c9f5865a71f0ab6a8c6e61f032f3013b84`.
`operations/index-010.json.gz`, `index-011.json.gz` and `index-012.json.gz`
preserve each index. Exact import/restore/matrix command arrays and stdout /
stderr are retained in `operations/`; `source/retain-final.py.gz` records the
archival orchestration. Its invocation records receipt revision
`e9f5b83f929f95b2c36496fc3083605dcc73cddf` and the helper/script identities.

Restore with the reviewed helper and the index above, then run the retained
frozen c17 runner against the twelve restored directories:

```sh
python3 audit/self-hosting/bootstrap-results/archive-tool/archive.py restore \
  --store audit/self-hosting/bootstrap-results/run-36359020560/platform-store \
  --index-sha256 e63844ab6f5e79713f0b073fc70f22c9f5865a71f0ab6a8c6e61f032f3013b84 \
  --output /fresh/restored-platforms
python3 /retained/frozen-bootstrap-runner.py --matrix /fresh/restored-platforms \
  --output /fresh/local-matrix
```

The expected matrix exit code is 1 with
`ValueError: bootstrap not passed/current schema`. Local `ci_origin` remains
null. The explicit original-origin comparison rejects with the same error.
`files.json` binds all compressed/raw evidence; gzip timestamps are zero.
Successful retention does not change the failed CI result or close SH6.
