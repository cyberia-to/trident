# Portable retained audit paths

The first native matrix for PR117, run `37000743077` at
`fdeda5fe903c10df4a49b1b247f20f5fa2ae3bdf`, failed checkout in all four Windows
producer jobs. Git reported `Filename too long` before compiler execution.
Nested copies of original evidence produced tracked paths up to 299 characters.
The original logs, job metadata and the subsequent cancellation of the remaining
jobs are retained in [first-run/files.json](first-run/files.json). This run
remains unsuccessful.

The [migration receipt](receipt.json) records the exact command and source
revision. It moves 36 stored files whose repository-relative paths exceeded
200 UTF-16 code units into shallow `objects/` paths. All 327 retained files
were fully decoded and compared before and after the migration. Each original
location, byte count, digest, encoding and stored byte identity remains unchanged
in [the current manifest](../files.json). The [original manifest](original-files.json)
preserves the initial layout exactly.

`test_tracked_paths_fit_native_windows_checkout` in the existing bootstrap
runner tests prevents tracked paths exceeding the same checkout budget. The
native Windows matrix must still run on the corrected commit; this local path
check does not claim Windows execution or SH8 acceptance. Compiler/runtime
sources, proof inputs, active commands and computational limits are unchanged.
