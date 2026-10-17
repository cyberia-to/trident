# Actual alternate-context constructor regression

This audit uses the unchanged accepted SH7 aggregate certificate, unchanged
installed Joy from `6e0ec4d8440e2521df08f442d64f54e667044716`, and the already
reviewed whole-file mutation helper. It consumes no whole self-build certificate
and makes no SH8 completion claim. Exact original inputs, executable and source
identities are in `sources.json`; all actual argv, empty child PATH, limits,
stdout, stderr and sampled resources are retained in `attempts/`.

`run.py` packs six actual alternate JOB1 requests: compiler, entry source,
dependency source, configuration, requested reductions, and evaluator frames.
The dependency edit changes an existing comment with equal length; this tests
source identity binding and does not claim execution of the changed request.
The compiler variant uses the accepted other-generation C1 ART1. Source and
dependency variants each change exactly one module. All packing succeeds.

For each variant the original proof rejects at its outer context; a complete
chain rebuilt under the actual admitted program/formula/JOB/profile/reductions/
frames reaches Zheng and rejects with `semantic record: Key`. The reductions
and frames cases also reproduce the old constructor error: substituting the
unchanged host ceiling produces the outer format/context error instead.

The fresh original and byte-identical rechain controls pass with complete output
and semantic fields equal to the accepted pilot. All thirteen other mutation
constructors also reject in their exact intended error classes, with empty
success stdout and unchanged protected destinations. This gives the original
23-case matrix plus two frame-binding rejects and two bug reproductions, using
58 bounded commands and 29 fresh verifier processes. All proof and mutation
files are retained; none is deleted.

`binding_context.py` is a context extractor for bytes already authenticated by
production `pack-job`, not a second noun authenticator. It validates the ART1
shape/profile and exact admitted source coordinates, requires canonical full
particles and positive u64/u32 limits, and selects the admitted JOB1 reductions
and frames. Its eight focused tests and actual compiler extraction pass. The
helper's source must be hash-bound by any future whole-suite caller.

Run `python3 -B -W error check_result.py` for a read-only replay of every retained
command input/output/log identity and case result. `actual-command.json` retains
the actual bounded fixture invocation. A separate invocation-wrapper observation
records Python file-finalizer warnings emitted only by the outer telemetry
wrapper; neither the tests nor any of the 58 child command logs emitted warnings.

The original full v2 suites stay failed. A later composite acceptance contract
must explicitly retain their nine already completed negative cases, complete
exactly the missing fourteen, and validate the disjoint union. The failed
rebound-job-limit case is not among those nine. Existing failed payloads remain
retained until a separately reviewed classification/reclamation transition.
