# Final matrix replay preparation

The reviewed validator `verify-final-matrix.py` is prepared for the twelve
platform artifacts of run `36359020560`, attempt 1, head
`c17bd0371c11746f46e20222c48cae2ab08be79d`. Its SHA-256 is
`c4e5220a51e08c41df221cdb4cf5950723a4fa0c6edef37d527069d58ca6bfc2`.
At preparation time only ten platform reports are complete. This directory
contains no final matrix acceptance yet.

The validator pins the retained store index supplied on the command line,
the frozen expected-input document and the unchanged c17 runner. It checks
every restored file against the exact original GitHub ZIP and API digest,
then binds each named platform/repetition to its source pins, recorded runner
and both known S1 compiler/result/inventory/JOB identities. It invokes the
unchanged runner's `--matrix` CLI locally with `ci_origin=null`, then calls
its `compare_matrix(reports, expected_origin)` explicitly. Local replay does
not impersonate CI. The original aggregate remains separate from the twelve
platform artifacts; its receipt must name all twelve producer hashes, and
both its original job and final workflow run must have succeeded.

The preparation receipts under `preparation/` retain the exact commands and
stdout/stderr for rejecting the actual incomplete ten-platform store with
normal and optimized Python. No output/acceptance directory is created by
those failed preconditions. `files.json` binds every compressed and raw file.
Source review strengthened three explicit bindings before any successful
matrix replay: producer runner identity, artifact-name target/repetition, and
both known S1 step identities. The earlier helper and its rejection logs are
retained separately; neither helper version has accepted an incomplete matrix.
The frozen-checkout cleanup receipt records removal of only the independently
identified session-generated Python bytecode; tracked c17 source was unchanged.

Actual final replay commands, original aggregate provenance and results will
be retained here when the twelve native producers and aggregate are available.
