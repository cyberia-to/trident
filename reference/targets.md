# Trident Target Reference

[Language](language.md) · [VM contracts](vm.md) · [Runtime contracts](os.md) · [Warrior API](warrior-api.md)

A target selects a machine ABI and, optionally, a runtime package. An entry
in the discovery catalog is not an implemented compiler or runtime. Current
compilation paths are the reference nox compiler in Trident and the Triton
adapter in Trisha. Other catalog machines remain declarations.

## Ownership and physical layout

| Responsibility | Owner and location |
|---|---|
| Language, frontend, generic intrinsics, portable libraries | Trident `src/`, `lib/vm/`, `lib/std/` |
| Reference nox ABI declaration and lowering | Trident `catalog/vm/nox/target.toml`, `src/ir/tree/lower/nox/` |
| Nox execution and Zheng integration | nox and Joy `rs/` |
| Joy runtime capability declarations | Joy `targets/nox/capabilities.json` |
| Triton ABI, lowering, costs, ISA metadata | Trisha `targets/triton/`, `rs/lower/`, `rs/cost/` |
| Explicit Triton hash and witness ABI modules | Trisha `lib/vm/triton/` |
| Neptune SDK and transaction policy | Trisha `lib/os/neptune/` |
| Neptune protocol and deployment states | Trisha `networks/neptune/` |
| Target discovery and design records | Trident `catalog/vm/`, `catalog/os/` |
| Expert Triton assembly baselines | Trisha `baselines/triton/` |

Trident carries one reference nox ABI consumed by Joy, not a second Joy
mirror. Upstream VM implementation remains nox. Foreign production ABIs
come from their installed owner; Trident's frozen stack ABI is a compiler
unit-test fixture only.

Source namespaces do not follow physical directory names. `vm.*`, `std.*`
and `os.<network>.*` imports resolve through compiler resources or explicit
package modules. Empty future directories, including a portable `lib/os/`
implementation or Joy cyber SDK, are not scaffolded as capabilities.

## Target selection

| CLI name | Alias | Meaning |
|---|---|---|
| `--engine` | `--terrain` | VM contract (`TerrainConfig`) |
| `--network` | `--union` | Runtime contract (`UnionConfig`) |
| `--vimputer` | `--state` | Deployment instance (`StateConfig`) |
| `--target` | — | Combined shorthand; default nox |

The compiler resolves a target once. Nox uses the embedded reference ABI.
Triton and Neptune query Trisha's target package; an absent or incompatible
owner fails rather than falling back to a stale machine literal. A bare
catalog entry cannot authorize compilation. `cyber` in Joy currently aliases
stateless nox execution, not an implemented network connection.

```sh
trident build program.tri --target nox
trident build program.tri --target triton   # requires installed Trisha
trident build program.tri --target neptune # Trisha's Triton + Neptune SDK
joy describe --target nox
trisha describe --target neptune
```

Explicit state selection belongs to the runtime owner. A state preset or
endpoint alone does not establish transaction submission support. In
particular Joy refuses CLI state requests; public Zheng certificates cannot
prove state execution.

## Versioned target packages

`describe --target NAME` emits JSON without running a user program. The
versioned `TargetPackage` contains:

- Schema and compiler API versions, owner and package version.
- Machine ABI, optional runtime and deployment states.
- Dotted module names, embedded source and content hashes.
- Available intrinsic names and editor instruction metadata.
- Separate run, prove, verify and deploy capabilities, proof formats and
  restrictions.

`CompileOptions::with_package(package)` validates the package and supplies
its terrain, modules and intrinsic set together. This prevents a compiler
using one ABI while the typechecker or SDK uses another. Runtime support is
not inferred from an intrinsic list: a VM operation may execute natively
while remaining outside its proof relation.

Compilation identity includes target ABI, package compilation identity,
configuration flags and effective resolved module contents. Deployment
presets are excluded from package compilation identity. Changes to supplied
module content require matching content hashes; ambient checkout files do
not silently replace locked package modules. See [Warrior API](warrior-api.md)
for the exact schema and compatibility rules.

## ABI-dependent libraries

The compiler generates `std.target` constants from the resolved terrain:

| Constant | nox | Triton |
|---|---:|---:|
| `DIGEST_WIDTH` | 4 | 5 |
| `HASH_RATE` | 8 | 10 |
| `STACK_DEPTH` | 0 | 16 |
| `FIELD_LIMBS` | 2 | 2 |

These are compilation parameters, not a promise that every operation of a
matching width exists. Generic I/O declarations are generated from available
intrinsics. Fixed-width tuple reads and native digest reads are distinct.
Explicit Tip5/Triton operations come from Trisha; nox does not silently
substitute Hemera for Tip5.

## Warriors

| Owner | Native execution | Proof contract | Deployment |
|---|---|---|---|
| Joy / nox | Public inputs and sequential secret witnesses; no CLI state loader | Bounded public Zheng execution certificate; full witness, linear verification; no ZK, secrets or state | Unimplemented |
| Trisha / Triton | Triton adapter | Installed package declares proof formats and restrictions | Consult installed capability and Neptune command; packaging is not chain deployment |

Always consult the installed package, not a catalog status level, before
claiming an operation works. A successful compiler build does not imply
proof coverage. Baseline coverage and release test failures must be reported
separately from available commands.

## Packaged artifacts

A `.deploy/` directory contains `manifest.json` and a declared `program_file`:
`program.nox` for nox, `program.tasm` for Triton. The loader reads the manifest's
filename, rejects traversal and files outside the directory, and checks the
compiled bytes against `program_digest`. This content check is not execution
proof verification. Older manifests without `program_file` require repackaging.

For migration decisions and validation gates see the
[target ownership review](../docs/explanation/target-ownership.md).
