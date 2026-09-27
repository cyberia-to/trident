# Why the Joy package is large

This measures the same installed Joy binary and revisions as [Hello World](README.md).
The [composition receipt](joy-composition.json) records the binary SHA-256,
source revisions, Mach-O sections, approximate code groups and embedded files.
All MB below use decimal bytes. These are observations, not size targets.

## File contents

`xcrun size -m /path/to/joy` reports 4,846,508 bytes of machine instructions
(`__text`), 1,185,156 bytes across constant/string sections, and 825,984 bytes
across exception/unwind sections. Reading the Mach-O `LC_SEGMENT_64` file sizes
shows another 1,492,144 bytes in `__LINKEDIT`; the balance is 40,016 bytes of
headers, alignment, stubs and other data. These sum to 8,389,808 file bytes.
The 4 GiB `__PAGEZERO` mapping occupies zero file bytes. `size -m` reports
virtual segment sizes, so its total is not the executable file length.

The previous measured `strip -x` plus ad-hoc signing removes 1,214,016 bytes.
The remaining file size is 7,175,792 bytes; stripping does not remove features.

## Retained functionality

Joy's `cli/main.rs` exposes build, run, run-artifact, pack-job, prove and verify
from one executable. `load_bundle` can compile `.tri` source at runtime.
`cli/Cargo.toml` directly depends on `trident-lang`; `rs/Cargo.toml` directly
and unconditionally depends on Trident, Zheng and `trisha-rs`. The private
execution path calls `trisha_rs::ccs::Checker`, which uses Triton proving.
Consequently selecting `joy run` does not eliminate the code needed by the
other CLI subcommands from the distributed executable.

`xcrun nm -n --demangle --defined-only /path/to/joy` confirms the retained
functions. Approximate adjacent symbol-address intervals attribute:

| Matched name | Instruction bytes |
|---|---:|
| `triton_vm` | 1,287,288 |
| `trident` | 568,452 |
| `clap_builder` | 325,824 |
| `twenty_first` | 171,680 |
| `serde_json` | 169,880 |
| `zheng` | 145,200 |

This attribution includes alignment and generic type names; it excludes
constants and unwind information. Shared/inlined functions prevent exclusive
crate accounting. These numbers are not predicted savings from removing a
feature. Two Triton `evaluate_transition_constraints` instances alone occupy
449,480 and 366,140 bytes by this method (815,620 bytes combined).

Trident's `build.rs` embeds all `.tri` and `.toml` files under `lib/` and
`catalog/`. The resource scan below finds 176 files with a combined source
length of 904,777 bytes, and every complete file occurs verbatim in Joy.
This source-length sum may include overlapping strings and is not an extra
section to add to the file-contents table.

```python
from pathlib import Path
root = Path('/Users/master/cyber/.worktrees/selfhost-0.4-source-capacity')
binary = (root / 'install-release/bin/joy').read_bytes()
files = [p for name in ('lib', 'catalog')
         for p in (root / 'trident' / name).rglob('*')
         if p.is_file() and p.suffix in ('.tri', '.toml')]
print(len(files), sum(p.stat().st_size for p in files))
assert all(p.read_bytes() in binary for p in files)
```

## Architectural implication

The measured package carries a compiler with its embedded libraries, an
execution engine, proof production/verification, formats and the CLI. A
precompiled Hello World only uses a subset. Runtime selection of a command
does not let the linker discard the other reachable commands.

A smaller application distribution would need an execution-only build with
compiler/resources and proof backends omitted at build time. The complete Joy
development/proving tool can remain available separately. Small-build flags
and stripping can then optimize that execution build. No such production
split or minimum-runtime size was implemented or measured in this audit.
The result does not establish a fundamental multi-megabyte minimum for nox
or Trident, and self-hosting alone does not change this packaging boundary.
