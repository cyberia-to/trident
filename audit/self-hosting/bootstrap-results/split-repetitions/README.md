# Independent native bootstrap repetitions

The original Intel macOS job in CI run `36353842247`, Trident
`23691cd2c6885bf25bfc023799552559724dbc2b`, exhausted its explicit one-hour
deadline during the first complete self-build. Its original failed artifact
is retained in [Joy's deadline audit](https://github.com/cyberia-to/joy/blob/ec83bd8d85b20a8bd20d2d14b0f25aab0f75e9fe/audit/compiler-deadline/README.md)
and on Trident's separate receipt branch. No compiler output was produced by
that failed step; the larger allowance requires new execution evidence.

Runner commit `0699069ec774188971d6d797f0159f6a4903f366` separates the two
required clean repetitions into independent native jobs. This integration
pins Joy `ec83bd8d85b20a8bd20d2d14b0f25aab0f75e9fe` and updates the canonical
SH6 instructions. `validation.json` records the exact input hashes before
the integration commit, its base, commands and losslessly compressed logs.

The new explicit whole-compiler host deadline is 7200000 ms; its subprocess
deadline is 7500 seconds. All guest work, allocation, resident storage, frame
and collection limits, and every existing corpus quota, remain unchanged.
The workflow keeps a bounded 330-minute bootstrap step in each 350-minute
job. These ceilings provide bounded execution policy, not a completion promise.

The v2 collector requires twelve separate native platform/repetition results,
both numbered repetitions for each platform, identical source pins and
execution profiles, actual C2/C3 producer chains, both complete supplied-compiler
corpora, and exact compiler bytes. CI run, attempt and head identities bind
the platform results and aggregate. Attempt-specific artifact names retain
older evidence and exclude earlier attempts from the current comparison.

The retained command run passes 46 distinct orchestration/fixed-point unit
tests. A separate optimized-Python invocation passes the 28 orchestration
tests again. These tests use synthetic evidence to exercise rejection of
missing, duplicate, relabeled, mixed-run, mutated and incomplete inputs;
their fixture results supply no native compiler acceptance. Workflow
actionlint and `git diff --check` also pass without diagnostics.

SH6 remains open until actual complete native execution and the aggregate
comparison succeed. The complete local S1 fixed point and both semantic
corpora remain separately retained on `test/0.4-selfhost-acceptance`.
