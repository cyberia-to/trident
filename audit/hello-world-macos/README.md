# Hello World size on macOS

Measured 2026-09-27 on macOS 26.4.1 (25E253), arm64. This is a local
measurement of the accepted release/0.4 inputs, not a release candidate.
All sizes are uncompressed file bytes from `Path.stat().st_size`.
The [receipt](measurements.json) preserves exact commands, outputs, source
snapshots, file SHA-256 values, compiler versions and revisions.

## Inputs and reproduction

- Trident: `9d52ee8fbb68833a65b52ebe9254bf24bbcd52d0`.
- Joy: `a15adb7310db5b159b4e72ce0dfd0fea2ff01c7d`.
- nox: `c9f7486a74fe81bfc194b598da40f6343ecb2ef1`.
- Installed executables: `../selfhost-0.4-source-capacity/install-release/bin/`
  relative to this worktree's parent. Exact absolute paths are in the receipt.

```trident
program hello

fn main() -> [Field; 14] {
    [72, 101, 108, 108, 111, 44, 32, 119, 111, 114, 108, 100, 33, 10]
}
```

```sh
trident build hello.tri --target nox --profile release -o hello.nox
joy build hello.tri --profile release --emit bundle -o hello.bundle.json
joy run hello.nox
joy run hello.bundle.json
```

## Results

| Measured file or set | Bytes | Meaning |
|---|---:|---|
| `hello.nox` | 292 | Text nox formula from `trident build` |
| `hello.bundle.json` | 691 | Formula and metadata from `joy build --emit bundle` |
| Installed `joy` | 8,389,808 | Complete macOS arm64 CLI; includes more than execution |
| `hello.sh` | 110 | Shell launcher and ASCII renderer for this fixture |
| Joy + formula + launcher | 8,390,210 | Tested package printing the exact greeting |
| Copied Joy, `strip -x` then ad-hoc signed | 7,175,792 | Tested experiment; installed binary remains unchanged |
| Stripped Joy + formula + launcher | 7,176,194 | Sum of the measured components |
| Installed `trident` | 8,646,064 | Compiler; not required to run the precompiled formula |
| C executable | 33,432 | Apple clang 21.0.0, `clang -Os hello.c -o hello-c` |
| Rust executable | 285,904 | rustc 1.95.0, command below |

```sh
rustc -C opt-level=z -C strip=symbols -C panic=abort -C lto=yes hello.rs -o hello-rust
cp /path/to/installed/joy joy.stripped
strip -x joy.stripped
codesign --force --sign - joy.stripped
```

The C source calls `puts("Hello, world!")`; Rust calls
`println!("Hello, world!")`. Both execute successfully and print exactly
`Hello, world!\n`. These are ordinary optimized builds, not a search for the
smallest possible native executable. Their sizes do not independently
validate the tweet's other language claims.

## Output and scope checks

The Trident source returns fourteen ASCII values. Both the raw formula and
bundle execute in 44 reductions and Joy prints each numeric leaf on its own
line, including the final list terminator zero. The first exploratory
assertion expected only the character values and failed on that terminator;
the final checks explicitly require all character values plus the zero.
Joy currently exposes numeric output, so literal terminal text requires a
renderer. The measured launcher uses macOS `/bin/sh` and `/usr/bin/awk`:

```sh
#!/bin/sh
set -eu
cd -- "$(dirname -- "$0")"
./joy run hello.nox | /usr/bin/awk '$0 != 0 { printf "%c", $0 }'
```

Its stdout was checked byte-for-byte. Zero removal is specific to this
fixture's final list terminator, not a general serialization API. The shell
pipeline does not propagate a failing Joy exit status; independent direct
Joy invocations in the receipt check successful execution. The launcher is
measurement scaffolding, not a proposed production application interface.

`otool -L joy` lists system libiconv and libSystem. Package totals exclude
macOS, its shell/awk and system libraries, filesystem allocation overhead,
archives, installer metadata and proofs. The application runs without the
Trident or Trisha executables alongside Joy. A minimum execution-only
runtime has not been measured.

A separate structured `main(input: Noun) -> Noun` fixture builds via
`joy build hello_raw.tri --profile release --emit artifact` to an 8,113-byte
ART1/NOXDAG01 file. `joy run-artifact` accepts it with the canonical zero
input and produces a result independently decoded to the exact greeting.
That fixture uses explicit noun construction; its size includes different
lowering as well as container overhead, so it is not an encoding-only
comparison with the 292-byte formula.

Current implemented compilation paths are nox and the external Triton
adapter. A direct arm64/x86 Mach-O executable backend is absent. The small
formula therefore measures a program for an installed runtime. Distributing
the tested macOS application currently costs about 8.39 MB, or about 7.18 MB
with the tested stripping experiment. Self-hosting the compiler on soft3
does not by itself introduce native Mach-O compilation or terminal I/O.

## Validation of this report

The receipt checks real execution, exact decoded bytes, program sizes and
binary identities. Only audit documents change on this branch. Accepted
compiler/runtime validation at the pinned revision is recorded in
`../self-hosting/source-capacity-validation.json`; the measured executables
match its accepted reproduction receipt. No implementation, target package,
release pin or self-hosting milestone changes in this report.
