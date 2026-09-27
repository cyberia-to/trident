# Bind the fixed-point source snapshot to the executed job

The previous checker verified retained source hashes and the inventory, but did
not bind those bytes to the retained JOB1. Updating both a source copy and its
inventory could leave an internally consistent source report beside a different
executed package. No successful C2/C3 pair had been accepted with that checker.

The checker now requires an explicit `--joy` binary whose SHA256 matches both
run receipts. After inventory validation it asks that binary to `pack-job` from
the same private temporary source copies, the hash-checked compiler, recorded
origins, options and limits. Exact bytes must equal the saved JOB1. Allowed host
arguments are a bounded list of numeric resource flags. The checker performs
metadata and transport operations; it never executes a guest compiler.

[Identities and commands](identities.json) pin base revision `b991d90`, changed
helper hashes, prebuilt tools and evidence. All 18 focused receipt tests pass,
including a same-length source change with updated metadata, repack failure,
binary mutation and command-option injection. Existing compiler chaining,
byte equality, output preservation and failed inventory checks remain covered.
The historical metadata-only helper retains its original scope; the full
fixed-point entry always requires the additional job binding.

Independent review also found that a late final tool-hash rejection could retain
the already assembled `fixed_point` field. Every failure now removes that field;
regressions cover changes to both tools after both steps finish. This final
reporting correction leaves the measured snapshot/repack logic unchanged.

The [real component probe](real-snapshot-final.json) uses the complete retained
94-module, 369820-byte snapshot and actual Joy `2878f4b`. The unchanged snapshot
reproduces JOB1 byte-for-byte. Replacing one newline with a space in a private
copy, updating its SHA256 and regenerating the complete inventory still passes
the inventory check, then fails the JOB1 byte comparison. Original run inputs
remain unchanged. The input receipt was still running; its exact bytes are
retained in [the input copy](real-snapshot-final.input.json). This component
does not establish C2 generation or a fixed point.

An initial probe rejected its temporary `/var` alias before the intended job
comparison. The probe now resolves its temporary directory; the final run above
passes both intended conditions. The initial failure remains in
`initial-probe-failure.log.gz`. Gzip logs retain exact raw bytes, with compressed
and decompressed SHA256 identities in the manifest.
