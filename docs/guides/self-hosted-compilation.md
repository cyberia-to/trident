# Compile with the self-built nox compiler

The frozen S1 native compiler can compile `.tri` source while running inside
Joy/nox. This guide uses an actual C2 emitted by that compiler, then executes
its output. The supported language subset and larger self-build procedure are
specified in [Self-Hosting](../../reference/self-hosting.md); current platform
acceptance is recorded in [the ledger](../../audit/self-hosting-progress.md).

Use the Joy `release/0.4` implementation with `pack-job` and `run-artifact`
support. The installed `joy` must be on your command path. The accepted kit
and installed/unpacked runtime checks are recorded in the
[actual delivery audit](https://github.com/cyberia-to/trisha/blob/c66c2da3da0d5b1da55f09be533a4d664d585bb2/audit/selfhost-kit/accepted/README.md).

## Prepare the compiler and source package

The current accepted [portable C2 kit](https://github.com/cyberia-to/trisha/raw/c66c2da3da0d5b1da55f09be533a4d664d585bb2/audit/selfhost-kit/accepted/selfhost-kit.tar.gz)
is retained with its original evidence. Its archive SHA256 is
`a3052d95c3de6d622157988a8e74826b2f0140724a634298458c3d75f6b508bd`;
the unpacked `kit.json` SHA256 is
`4096a513d439adda55e62731f461ff0a7a72fa0c85d79292090be048b7f55ae8`.
The kit carries the compiler and provenance; obtain Joy separately from the
compatible `release/0.4` build. Its accepted status covers the frozen S1 compiler
matrix. The [coordinated distribution rehearsal](https://github.com/cyberia-to/trisha/blob/95899e8f4fe32b5d7269d92b5e63ef429fbfafac/audit/final-host-ceiling-package/README.md)
passed for frozen source archive `73b50ebd`. Public versioning and release
promotion remain separate owner-controlled steps.

An accepted coordinated distribution includes `share/trident-selfhost/`.
Verify the distribution and kit archive checksums before using their contents.
Copy `compiler.dag`, `sample.tri`, `package.json` and `zero.dag` from that kit
into a fresh directory. The compiler has SHA-256:

```text
76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8
```

The kit manifest binds every file. `sample.tri` is the source below,
`package.json` provides its explicit module/entry/options/limits, and `zero.dag`
is a complete native atom-zero input for the emitted program. A kit marked
`rehearsal` remains an explicitly unaccepted local input; changing that field
cannot confer distribution acceptance.

```trident
program sample

fn main() -> Field {
    13
}
```

The package manifest includes every source module the guest may import.
`pack-job` serializes their exact bytes and binds the job to `compiler.dag`.
Module resolution, parsing, checking and code generation execute in the guest.

## Compile and execute

Run these commands in the prepared directory:

```sh
joy pack-job --compiler compiler.dag --manifest package.json --output job.dag
joy run-artifact compiler.dag --input job.dag --emit program --output sample.dag
joy run-artifact sample.dag --input zero.dag --output answer.dag
```

The second command publishes the ART1 program produced by C2. The third
publishes its result as a complete NOXDAG01 noun: atom 13 in this example.
Each successful command prints one JSON receipt. Keep those receipts and the
output bytes together. The expected result is atom 13; the distribution
validates this route using its installed Joy binary.

Existing destinations require an explicit `--force`. A rejected package,
runtime failure or compiler diagnostic leaves an existing program intact.
These small commands use ordinary host limits. Larger packages need explicit
job and host limits; the complete compiler bootstrap has its own measured
profile in the self-hosting contract.

The ordinary `joy build` command invokes the Rust seed compiler. Select the
`pack-job` → `run-artifact` sequence above to compile with C2. This guide
demonstrates execution. Joy also implements `prove-artifact` and
`verify-artifact` for public ART1/JOB1/RES1 programs with computed continuations
and variable result shapes. Its [structured certificate contract](https://github.com/cyberia-to/joy/blob/dd61df9128f6da1f97d4698f45f154f05312fe51/specs/structured-certificates.md)
specifies independent Zheng verification, full public witness disclosure and
the resource bounds. Complete self-build proofs have [accepted byte-equivalence retention](https://github.com/cyberia-to/trisha/blob/fbea3cef9a4139075e529c319ff75488ed5df625/audit/whole-retention-byte-closure/README.md).
Final adversarial and independent-checker acceptance remains open under
[SH8](../../reference/self-hosting.md#sh8-proved-self-compilation); follow the
[acceptance ledger](../../audit/self-hosting-progress.md) for workload evidence.


## Rebuild the complete frozen compiler

This path needs Python 3.10 or later, the supplied C2 and its verified kit
metadata, installed Joy and the matching Trident source tree. A standalone
Trident checkout or source archive is sufficient; the Trident directory from
a coordinated source archive is another option. Preparation verifies all 94
frozen source files by their exact bytes. No Rust toolchain, Cargo, seed
compiler, inventory executable, Trisha or Neptune installation is needed.
Joy executes all source-language work inside the supplied compiler.
The preparer reproduces frozen S1 only. Edited-source development uses a
separate explicit JOB1 package manifest with its own declared inputs and limits.

Prepare the matching Trident source tree and unpack the checksum-verified
portable kit. Obtain the
`kit.json` SHA256 from the verified kit/distribution metadata; preserve that
expected digest independently of files being checked. Set `kit_digest` to it,
then prepare a fresh directory (its parent must already exist):

```sh
python3 /absolute/trident-source/scripts/prepare-selfhost-source.py \
  --kit /absolute/trident-selfhost \
  --kit-manifest-sha256 "$kit_digest" \
  --source-root /absolute/trident-source \
  --output /absolute/selfbuild
```

The helper requires an accepted kit by default. For an explicitly local
historical rehearsal, append `--rehearsal`; both its receipt and any later run
must retain that qualification. It never upgrades acceptance. The helper
rejects changed or missing source bytes, altered compiler profiles/limits,
unsafe paths and occupied or overlapping destinations before publishing a
prepared package.

The output contains the exact compiler, all 94 frozen source files,
`package.json` and `receipt.json`. Status `prepared` establishes only byte
preparation. The manifest preserves `native_compiler/main`, the origins,
profile1/1 compiler output and every frozen job limit. Standard-library modules
are explicit package inputs, with no host fallback.

From that directory, pack and compile with the existing Joy commands:

```sh
joy pack-job --compiler compiler.dag --manifest package.json --output job.dag \
  --arena-nodes 1000000000 --budget 20000000000 --frames 65536 \
  --time-ms 7200000 --validation-visits 16777216 \
  --resident-nodes 3145728 --collection-work 10000000000 \
  > pack.stdout 2> pack.stderr
joy run-artifact compiler.dag --input job.dag --emit program --output c3.dag \
  --arena-nodes 1000000000 --budget 20000000000 --frames 65536 \
  --time-ms 7200000 --validation-visits 16777216 \
  --resident-nodes 3145728 --collection-work 10000000000 \
  > compile.stdout 2> compile.stderr
```

Run the second command only after the first exits successfully. Choose fresh
log destinations and retain each exit status. These are the published complete
compiler allowances: 20 billion reductions, 1 billion cumulative nodes,
3145728 resident nodes, 10 billion collection work, 65536 frames, 16777216
validation visits and a two-hour host deadline. They are independent limits;
a larger allowance is not a completion guarantee. No automatic retry raises a
limit after failure.

After the second command exits successfully, compare complete artifact bytes:

```sh
python3 -c 'from pathlib import Path; import sys; sys.exit(Path("compiler.dag").read_bytes() != Path("c3.dag").read_bytes())'
```

Exit zero means the emitted C3 matches the supplied C2 byte for byte. Preserve
both artifacts, the preparation receipt, JOB1 and Joy JSON/raw logs. The
admission's compiler/job particles must match execution's program/input
particles, with compiler-job status `success`. This run establishes its own
self-reproduction result; the native matrix/corpus acceptance and SH7/SH8
compilation proofs have separate gates.

The source-bound preparation and execution evidence is retained in
[the supplied-compiler audit](../../audit/self-hosting/lexer-bootstrap/supplied-compiler/README.md).
Its helper source is commit `98c5897aafde1072c692e0c1373d30f40f917e0d`,
with SHA-256 `4f2381880f7da77265ed562cb99bc0e7bcad4c5f7df28f8c62bf7a37f55c3711`.
Later guide and test portability clarifications do not change that helper or
the frozen compiler sources used by the measurement.
