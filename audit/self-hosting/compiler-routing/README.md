# Supplied compiler routing

Local validation at Trident `ca9d926b65701be4f9fe0eadac0ac05283b95bb6`, with
uncommitted runner changes identified in [identities.json](identities.json).
That manifest records the exact commands, compiler and Joy file hashes, runner
source hashes, sibling revisions and evidence hashes. Inputs are local frozen
files; this receipt makes no release-source claim.

The provided input is the parent's seed-built full C1. This checks the route that
will accept C2/C3 later. It establishes no C2, C3 or full corpus acceptance.
The existing corpus and its host/package quotas are unchanged.

The [unit run](unit-tests.log) passes nine tests: supplied-path routing,
profile rejection without fallback, default seed routing, missing compiler,
compiler/output alias rejection, Joy/output alias rejection in both modes,
preservation of existing receipts, compiler mutation detection and JOB1/execution
particle mismatch detection. The runner has 480 lines; its unchanged independent
artifact reader and record constructor now live in `native_compiler_artifact.py`.

```sh
python3 -m unittest discover -s audit/self-hosting -p test_native_compiler_runner.py -v
python3 audit/self-hosting/compiler-routing/first-case-probe.py --joy ../install-scaled/bin/joy --compiler ../measurements/scaled-c1.dag --output audit/self-hosting/compiler-routing/provided-c1.json
python3 audit/self-hosting/run-native-compiler.py --joy ../install-scaled/bin/joy --compiler /tmp/trident-sort-specialized.dag --output audit/self-hosting/compiler-routing/profile-rejection.json
```

Choose new output paths when reproducing these commands: the runner preserves
existing evidence. Run from the Trident worktree recorded in the manifest.

The [first-case probe](first-case-probe.py) wraps only process dispatch and stops
externally when the runner reaches the second package. The real `precedence`
case completed all its checks, including execution producing 14. Its three
commands were `pack-job`, compiler `run-artifact`, and generated-program
`run-artifact`; there were zero source builds. Compiler and Joy hashes remained
unchanged. The [partial receipt](provided-c1.partial.json) contains the exact
invocation and command outputs. The runner's own [receipt](provided-c1.json)
retains `running` status because this was intentionally not a complete run.

The final command supplies a canonical raw-profile ART1 ordering fixture. Joy
rejects it at the first `pack-job` with `requires compiler profile(1,1)`. The
[receipt](profile-rejection.json) and [log](profile-rejection.log) retain that
expected failure, unchanged input hashes and the sole command. No seed fallback
runs after rejection.
