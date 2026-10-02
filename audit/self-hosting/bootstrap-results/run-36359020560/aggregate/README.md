# Original failed aggregate

The actual aggregate for run `36359020560`, attempt 1, head
`c17bd0371c11746f46e20222c48cae2ab08be79d`, failed with
`ValueError: bootstrap not passed/current schema`. Ten platform repetitions
passed; two Intel macOS repetitions were interrupted during C3 corpora.

Original aggregate artifact `10953060170` is retained here as its exact
1061-byte ZIP, SHA-256
`85267763aa8e9e210799df9d1978d01ebe21d13a8158c21941742d24dd6fa88f`.
The raw artifact API JSON, its single original receipt, the direct aggregate
job API response (`108794676918`), raw job log and final run API response are
retained losslessly alongside it. This artifact stays separate from the
twelve-entry platform store.

The actual aggregate receipt has SHA-256
`426889e8fc5f8e80512f04157549985836f22302490560f72a0736f04e77d713`.
Its explicit run/head/attempt and all twelve producer receipt hashes match
the original downloaded platform trees and their independent fresh restore.
The final REST run response reports `completed` / `failure`, with SHA-256
`2fbc6ebc33c691e2368bcab1e809f4195e79b0cdd27c72dd11d6795b813d4923`.

The frozen c17 runner's local `--matrix` CLI returns exit code 1 and the same
error, with local `ci_origin=null`. Calling its unchanged
`compare_matrix(reports, expected_origin)` also rejects. These observations
are recorded in `binding.json.gz`, `aggregate-validation.json.gz` and the
[restored-platform validation](../mac-intel/README.md). No guest stages were
rerun by this verification.

`files.json` binds raw and stored bytes, including the original ZIP. The
archival command source and receipt revision are recorded under
`../mac-intel/`; the measured CI head remains c17. SH6 remains open.
