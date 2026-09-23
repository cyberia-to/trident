---
title: Trident
tags: trident, core
crystal-type: entity
crystal-domain: cyber
icon: "🔱"
alias: Tri, tri, the provable language
---
# Trident

> The weapon is their language. They gave it all to us.
> If you learn it, when you really learn it, you begin to perceive time
> the way that they do. So you can see what's to come.

<p align="center">
  <img src="https://raw.githubusercontent.com/cyberia-to/trident/master/media/tri.gif" width="100%" alt="Trident" />
</p>

Trident is a provable programming language.

Every variable, every operation, every function compiles to arithmetic
over the Goldilocks prime field (p = 2^64 - 2^32 + 1). A program is a
formula; running it and proving it ran correctly are the same act.

Trident is the language of the [soft3](https://soft3.org) stack. It
compiles to **nox**, the proof-native VM, and the **joy** warrior runs,
proves and verifies what it emits through **zheng**. Public execution
certificates disclose their witness; private execution uses Trisha's Triton 7
STARK to check Zheng's execution constraints. One algebra
([strata](https://github.com/cyberia-to/strata)), one hash
([hemera](https://github.com/cyberia-to/hemera)), one field, from source
to proof. Twenty other engines and twenty-five unions are declared
behind `--target`, Triton VM with its STARK is the second tested one,
and the in-repo `silicon` crate contains experimental emitters for 25 native
backends — from Cortex-M and CUDA to OpenQASM and Verilog. Catalog entries and
experimental emitters do not establish runtime or proof support.

The coordinated CPU release is **Trident 0.3.0, Trisha 0.3.0 and Joy 0.5.0**.
[Download native macOS, Linux and Windows archives](https://github.com/cyberia-to/trident/releases/tag/v0.3.0)
or build the complete coordinated source archive. Individual registry installation
of this release is not available. See the [release notes](audit/release-notes-v0.3.0.md)
and [source-bound validation](audit/release-2026-09-16.md).

---

## Hello, Proof

```trident
program hello_proof

fn main() -> Field {
    let a: Field = divine()
    let b: Field = divine()
    a + b
}
```

```sh
trident build hello_proof.tri --target nox
trident prove hello_proof.tri --target nox --secret 7,13
trident verify hello_proof.zheng
```

A proof that `a + b = 20` without revealing `a` or `b`, checked
without re-running the program. The prover's witness never leaves the
prover — the artifact carries the statement, the proof and the formula,
nothing else.

---

## The Mental Model

```
.tri source
  |
  |  trident build
  v
.nox formula  +  static cost report          (--target triton: .tasm)
  |
  |  joy runs it on nox                        (trisha on Triton VM)
  v
Execution trace  =  the zheng witness
  |
  |  joy proves
  v
zheng proof + statement                      (STARK on Triton)
  |
  |  joy verifies — no re-execution
  v
true / false
```

Trident owns **source -> formula + cost**. The warrior owns
**execute -> trace -> prove -> verify**. The compiler exists to expose
cost, not hide it:

```
$ trident build hello.tri --costs
  Cost model: reductions (nox)
    reductions:  5
    formula nodes: 15
    by pattern (static, all arms):
      axis     ×3     = 3 reductions
      add      ×1     = 1 reductions
      mul      ×1     = 1 reductions
```

Five reductions billed before running; `joy run` executes in exactly
five. Branch-dependent programs get an honest `min..=max` range.

---

## The Stack

On the default target a Trident program never leaves the soft3 stack:

| step | component | what it does |
|------|-----------|--------------|
| compile | **trident** | `.tri` → `.nox` formula over 18 reduction patterns; cost in reductions |
| execute | [nox](https://github.com/cyberia-to/nox) via [joy](https://github.com/cyberia-to/joy) | reduces the formula; the trace is the witness |
| prove | [zheng](https://github.com/cyberia-to/zheng) via joy | Public execution certificate: full authenticated witness and exact CCS constraints |
| verify | joy | checks the proof against the statement — no re-execution |
| state | Compiler/nox state primitive | State execution certificates are unsupported by the current Joy path |
| algebra · hash | [strata](https://github.com/cyberia-to/strata) · [hemera](https://github.com/cyberia-to/hemera) | Goldilocks arithmetic and Poseidon2 inside the compiler — no parallel implementations |

The default Zheng path verifies the computation/public-result relation for its supported bounded programs without native re-execution. It reveals the witness and has linear certificate size and verification work; it is neither ZK nor succinct. Secret inputs, state proofs, and unsupported execution shapes fail explicitly. Older folded trace-statement artifacts do not establish this relation and require explicit legacy inspection.

---

## Target ownership and installed support

Trident implements the nox compilation path and shared frontend/TIR. Trisha owns Triton lowering, machine metadata, SDK, execution, and STARK proving. Joy owns the nox runtime/proof integration. `catalog/vm/` and `catalog/os/` contain discovery/design records; a catalog entry does not provide a compiler backend or working deployment.

| Target | Implemented path | Runtime/proof scope |
|---|---|---|
| `nox` (default) | Trident → nox formula → Joy | Execution; bounded public Zheng execution certificates |
| `triton` | Trident typed TIR → Trisha TASM | CPU execution and Triton STARK proof/verification |
| `neptune` | Triton machine plus Trisha-owned Neptune SDK | Helpers and proposed standards; no live deployment or complete recursive verifier |
| Other catalog names | Declared metadata and design documentation | Not installed implementations |

Targets resolve a versioned package once: machine ABI, intrinsic capabilities, module contents and hashes, and runtime capabilities. Library imports retain `vm.*`, `std.*`, and `os.*` namespaces independently of repository paths. See [targets](reference/targets.md) and [warrior API](reference/warrior-api.md).

### The same formula, on silicon (sketches, not warriors)

A nox formula is a tree over 18 patterns — small enough to hand-emit
for any machine. `silicon` (`trident/silicon/`, an in-repo, unpublished
crate — build with `cargo install --path silicon`) turns the very
formula joy proves into native code for **25 backends**, and every one
of them emits real output for `hello.nox` today. Honest scope: the
emitters cover the atom-level patterns (axis, quote, branch, field
arithmetic, bitwise — nox patterns 0, 1, 4, 5–14); programs that use
`hash`, `divine`, structs/cons or state reads are refused with
`UnsupportedPattern`, and the emitters produce code, not traces —
execution and proving on this hardware are not wired. The point is that
one program already speaks to all of it; running and proving there is
the next tier — see
[`.claude/plans/warrior-owns-lowering.md`](.claude/plans/warrior-owns-lowering.md)
for the shape that tier takes.

| class | backends |
|-------|----------|
| CPU | `x86-64` · `arm64` (JIT) · `rv64` · `rv32` (ESP32) · `rvv` (RISC-V vector) · `thumb2` (Cortex-M · STM32 · RP2040) · `hexagon` (Qualcomm DSP) |
| GPU | `ptx` (CUDA) · `tensor-cores` (wmma) · `wgsl` (WebGPU) · `spirv` (Vulkan) |
| accelerators | `ane` (Apple Neural Engine) · `amx` (Apple matrix) · `intel-amx` · `xla` (TPU) · `onnx` · `cerebras` (wafer-scale CSL) · `upmem` (processing-in-memory) |
| kernel · web | `ebpf` (Linux kernel) · `wasm` (browser · WASI · every wasm chain) |
| quantum | `qasm` (OpenQASM 3.0 circuits) · `qir` (Quantum IR · Azure Quantum) |
| hardware | `verilog` (FPGA) · `systemverilog` (ASIC) · `vhdl` |

```
$ trident build hello.tri                # hello.nox
$ silicon hello.nox -t qasm -o hello.qasm
$ silicon hello.nox -t verilog -o hello.v
$ silicon hello.nox -t spirv -o hello.spv
```

### Neptune (`--target neptune`)

Trisha owns Neptune bindings and proposed token standards. Selecting `neptune` supplies its SDK; the bare `triton` target supplies only machine bindings. Compilation of these sources does not establish correctness of a live transaction protocol.

| Source | Scope |
|---|---|
| [Coin](../trisha/examples/neptune/standards/coin.tri) | Proposed fungible-token standard |
| [Card](../trisha/examples/neptune/standards/card.tri) | Proposed non-fungible-token standard |
| [Lock scripts](../trisha/examples/neptune/locks/) | Authorization helpers |
| [Type scripts](../trisha/examples/neptune/types/) | Proposed token checks |
| [Experimental proof programs](../trisha/examples/experimental/neptune/) | Unimplemented recursive verification and transaction prototypes, excluded from the SDK |

The old `os.neptune.proof` prototype did not constrain computed verification values. Production imports now fail; it must not authorize transactions. Trisha's CPU Triton STARK prover/verifier remains a separate working path.

See the [Gold Standard](docs/explanation/gold-standard.md) for the full
PLUMB specification and the [Skill Library](docs/explanation/skill-library.md)
for designed token capabilities.

---

## Why a New Language

Provable VMs are not conventional CPUs. Treating them as such leaves
orders of magnitude on the table.

**The machine word is a field element, not a byte.** Trident's
primitives — `Field`, `Digest`, `XField` — map directly to what the
VM computes. Rust compiled to RISC-V wraps field operations in
byte-level emulation.

**On nox every field operation is one reduction.** The program is a
formula over 18 patterns; the cost report before you run equals the
reduction count after; the execution trace *is* the proof witness — no
arithmetization step, no trace-to-circuit translation.

**The gap to byte-emulating zkVMs is not marginal**, in either
machine's own unit — a nox *reduction* is one trace row the prover
folds (a hash is 24 Poseidon2 rounds + 1 squeeze row), a Triton *cycle*
one instruction across six tables; both are exact and static, from
`trident build --costs`:

| Operation | Trident on nox (default) | Trident on Triton VM | Rust on SP1 | Rust on RISC Zero |
|-----------|:---:|:---:|:---:|:---:|
| One hash (Poseidon2 / Tip5 / SHA-256) | 25 reductions | 1 cycle | ~3,000 cycles | ~1,000 cycles |
| Merkle path (depth 32) | 1,906 reductions (825 in hashes) | ~100 cycles | ~96,000 cycles | ~32,000 cycles |

For hash-heavy programs — Merkle trees, content addressing, token
transfers — this is decisive. See
[Comparative Analysis](docs/explanation/provable-computing.md).

**Bounded execution is not a limitation.** It is what makes programs
provable, costs predictable, and (eventually) circuits compilable to
quantum hardware. All loops have explicit bounds. No recursion.
No heap. No halting problem.

**Proofs compose, calls don't.** A proof can verify another proof
inside it. Any chain of proofs collapses into one. Trident is designed
for recursive proof composition — not invocation.

---

## The Rosetta Stone

Three computational revolutions — quantum, privacy, AI — share a
common algebraic root: the prime field.

A single lookup table over Goldilocks simultaneously functions as:

| Reading | Role | What it provides |
|---------|------|------------------|
| Cryptographic S-box | Hash nonlinearity | Security |
| Neural activation | Network expressiveness | Intelligence |
| FHE bootstrap | Encrypted evaluation | Privacy |
| Proof lookup | Proof authentication | Verifiability |

One table. One field. Four purposes. When all systems operate over the
same prime field, four separate mechanisms collapse into one data
structure read four ways.

[Trinity](docs/explanation/trinity-bench.md) demonstrates this: a single
Trident program encrypts input with LWE, runs a neural layer, hashes
with Tip5, performs FHE bootstrapping, and commits via a quantum
circuit — all inside one proof trace.

To our knowledge, no existing system composes FHE, neural inference,
hashing, and quantum circuits in a single proof.

See [Quantum](docs/explanation/quantum.md) ·
[Privacy](docs/explanation/privacy.md) ·
[Verifiable AI](docs/explanation/ai.md) ·
[Vision](docs/explanation/vision.md)

---

## Formal Verification

Annotate. Then prove.

```trident
#[requires(amount > 0)]
#[requires(sender_balance >= amount)]
#[ensures(result == sender_balance - amount)]
fn transfer(sender_balance: Field, amount: Field) -> Field {
    assert(amount > 0)
    assert(sender_balance >= amount)
    sender_balance - amount
}
```

```
$ trident audit transfer.tri
  All 3 properties verified (0.2s)
  No counterexample exists for any input in Field
```

Trident's restrictions — bounded loops, no recursion, finite field
arithmetic — make verification decidable. The compiler proves
correctness automatically. No manual proof construction.
See [Formal Verification](docs/explanation/formal-verification.md).

---

## Content-Addressed Code

Every function has a unique cryptographic identity: the hemera hash of
its normalized AST — the same hash that names particles in the
cybergraph. Names are metadata. The hash is the truth.

```
$ trident deploy transfer.tri
  Hash:     #a7f3b2c1d4e8
  Verified: (audit certificate attached)
  Cost:     47 reductions
  Published to registry
```

Rename a function — the hash doesn't change. Publish independently
from the other side of the planet — same code, same hash. Verification
certificates travel with the identity, not the name. And because nox
memoizes by the hash of (formula, object), a deployed function is
computed once for everyone.
See [Content-Addressed Code](docs/explanation/content-addressing.md).

---

## Verification status

The Rust compiler is the current implementation. `lib/std/compiler/` contains
experimental components, not a complete self-hosted compiler or a proof
that this compiler binary faithfully implements the language.

[Self-hosting on soft3](reference/self-hosting.md) defines the native nox/Joy
milestones and acceptance gates. The [progress ledger](audit/self-hosting-progress.md)
records the current task and implementation evidence.

Triton execution and STARK proving belong to [Trisha](../trisha).
`trisha bench --full` requires explicit input/output fixtures and verifies
proofs for the measured programs. Missing fixtures are reported as
unverified and fail the command. Historical ratios from modified or
non-executed assembly are not release evidence.

The nox compiler and Joy executor support the surface documented in
[reference/nox.md](reference/nox.md). Zheng derives the execution constraints
from the canonical program; verification binds public input, output, cost and
authenticated state roots. Public JOYEXEC2 and JOYST001 certificates disclose
the witness. Private JOYZK003 artifacts prove the same bounded relation using
a real Triton STARK and omit private columns. Private queries select from
bounded fully public state tables. Dynamic continuations, variable noun shapes
and a private database remain unimplemented. Legacy unauthenticated recursive
opening APIs remain disabled. See [Joy's proof contracts](../joy/README.md)
and the [coordinated validation ledger](audit/full-release-preparation.md).

---

## Quick Start

Build the coordinated development checkouts with their locked dependencies:

```sh
nu ../trisha/patches/apply.nu
cargo install --path . --locked
cargo install --path ../joy/cli --locked
cargo install --path ../trisha/cli --locked
trident build main.tri                     # nox output by default
trident run main.tri --input-values 3,5     # Joy execution
trident build main.tri --target triton     # Trisha emits TASM
trident run main.tri --target triton       # Trisha execution
trident prove main.tri --target triton     # Triton STARK proof
trident check main.tri                     # type-check
trident fmt main.tri                       # format source
```

Follow [Trisha's build instructions](../trisha/README.md) to prepare its
patched dependencies before installing. Warriors must be on `PATH`.
An unavailable warrior is an error. An explicit target overrides the
project target; otherwise `trident.toml` selects the target, then nox is
the fallback. Standard compiler resources are embedded in the binaries.

---

## Design Principles

1. **Field elements all the way down.** The machine word is `Field`, not `u64`.
2. **Bounded execution.** Explicit loop bounds. No recursion. No halting problem.
3. **Compile-time everything.** Types, array sizes, and costs known statically.
4. **Constraints are features.** No heap, no dynamic dispatch — safety guarantees.
5. **Provable first.** Designed for proofs. These constraints make great conventional programs too.
6. **One stack.** Nothing outside soft3: strata (algebra), hemera (hash), nox (target) — plus clap, ariadne, tower-lsp, tokio, blake3. No parallel implementations of anything the stack already provides.

---

## Source Tree

```
src/          Compiler in Rust            ~36K lines, soft3-native (strata · hemera · nox)
lib/vm/       Generic intrinsic contracts; target-shaped modules are generated
lib/std/      Portable source libraries; std.target is generated from the ABI
catalog/vm/   Machine discovery records, not backend implementations
catalog/os/   Network discovery and design records
../trisha/    Triton SDK, machine/network manifests, baselines and warrior
tests/        nox_surface.rs, differential.rs — every lowering reduces on the real VM
```

```
vm.*              Compiler intrinsics       hash, sponge, divine, assert
std.*             Standard library          sha256, bigint, ecdsa, poseidon2
os.*              Runtime namespace         os.state.read builtin; portable modules are future work
os.<target>.*     Target-specific APIs      Neptune modules supplied by Trisha
```

The warriors live beside the compiler:
[joy](https://github.com/cyberia-to/joy) (nox, the cyber battlefield) and
[trisha](https://github.com/cyberia-to/trisha) (Triton VM, Neptune).

Development stubs reserve five more warriors: **vitalina** (Ethereum),
**gaw** (Polkadot/DOT), **tolya** (Solana/SOL), **pearla** (Pearl inference),
and **zenda** (Zcash). See [planned warriors](reference/warrior-api.md#planned-warriors)
for their ownership and current stub status.

---

## Standard Library

**Implemented:** `std.field` · `std.crypto` · `std.math` · `std.data` ·
`std.io` · `std.compiler`

**In development:** `std.nn` (field-native neural networks) ·
`std.private` (ZK + FHE + MPC) · `std.quantum` (gates, error
correction)

On nox today the surface is: literals, let, arithmetic, comparison,
if/else, early return, mutable assignment, bounded `for`, function
calls, structs/arrays/tuples, `hash`, the `assert` family, field ops,
`divine()`, `os.state.read`. Sponge/Merkle/RAM/IO builtins, xfield
ops and dynamic array indices are explicit compile errors on nox —
never wrong code — and work on Triton.

---

## Documentation

Organized with the [Diataxis](https://diataxis.fr/) framework.
Full index: [docs/README.md](docs/README.md)

| | Start here |
|---|---|
| **Tutorials** | [The Builder's Journey](docs/tutorials/README.md) — from hello-proof to a DAO |
| **Guides** | [Compiling a Program](docs/guides/compiling-a-program.md) — build, test, deploy |
| **Reference** | [Language Reference](reference/language.md) — types, operators, builtins |
| **Explanation** | [Vision](docs/explanation/vision.md) — why Trident exists |

---

## Editor Support

| Editor | Setup |
|--------|-------|
| [Zed](https://zed.dev/) | Extension in `editor/zed/` |
| [Helix](https://helix-editor.com/) | Config in `editor/helix/languages.toml` |
| Any LSP client | `trident lsp` — diagnostics, completions, hover, go-to-definition |

---

## Status

Trident **0.3.0** is published with Trisha **0.3.0** and Joy **0.5.0**.
The compiler owns the shared frontend, nox lowering and extension interfaces.
Trisha owns Triton lowering, emission, execution, Neptune libraries and
Triton baselines. Joy owns nox execution and Zheng proof integration.
See [the warrior API](reference/warrior-api.md).

All scoped release gates passed: six native CPU targets, 133 execution fixtures
on each target, 198 freshly generated and verified baseline proofs on the
dedicated ARM Mac, all 36 cross-platform proof exchanges, and actual isolated
Neptune transaction admission. The [published validation report](audit/release-2026-09-16.md)
identifies the exact released sources and binaries. Broader nox execution and
live node/database integration remain roadmap work.

---

## License

[Cyber License](https://github.com/mastercyb/cyber/blob/master/graph/cyber/license.md)

Don't trust. Verify. Don't fear. Publish. Don't beg. Build.
