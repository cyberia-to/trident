# Compile with the self-built nox compiler

The current native compiler can compile `.tri` source while running inside
Joy/nox. This guide uses an actual C2 emitted by that compiler, then executes
its output. The supported language subset and larger self-build procedure are
specified in [Self-Hosting](../../reference/self-hosting.md); current platform
acceptance is recorded in [the ledger](../../audit/self-hosting-progress.md).

Use the Joy `release/0.4` development implementation with `pack-job` and
`run-artifact` support; this rehearsal used commit `ec83bd8d`. The installed
`joy` must be on your command path.

## Prepare the compiler and source package

Obtain `repeat-1/c2.dag` from a successful native bootstrap artifact, or restore
it from the [retained platform store](../../audit/self-hosting/bootstrap-results/run-36359020560/README.md).
Copy it into a fresh directory as `compiler.dag`. The compiler used here has
SHA-256:

```text
76a07c08265bd2ef525164472b6b53ac3f0e6cbbedce3250c4202f40ffba34c8
```

Copy these three [checked example inputs](../../audit/self-hosting/bootstrap-results/cli-guide/README.md)
into the same directory:

- [sample.tri](../../audit/self-hosting/bootstrap-results/cli-guide/sample.tri):
  the source below.
- [package.json](../../audit/self-hosting/bootstrap-results/cli-guide/package.json):
  explicit module paths, entry, options and resource limits.
- [zero.dag](../../audit/self-hosting/bootstrap-results/cli-guide/zero.dag):
  a complete native atom-zero input for the emitted program.

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
Each successful command prints one JSON receipt; the checked commands,
receipts and output bytes are retained with the example. Compare `answer.dag`
with the [retained expected output](../../audit/self-hosting/bootstrap-results/cli-guide/answer.dag)
to check this exact example.

Existing destinations require an explicit `--force`. A rejected package,
runtime failure or compiler diagnostic leaves an existing program intact.
These small commands use ordinary host limits. Larger packages need explicit
job and host limits; the complete compiler bootstrap has its own measured
profile in the self-hosting contract.

The ordinary `joy build` command invokes the Rust seed compiler. Select the
`pack-job` → `run-artifact` sequence above to compile with C2. This guide
demonstrates execution; proofs of dynamic compilation remain SH7/SH8 work.
