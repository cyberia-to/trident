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
proves and verifies what it emits — a **zheng** proof, hash-based,
post-quantum, no trusted setup, no elliptic curves. One algebra
([strata](https://github.com/cyberia-to/strata)), one hash
([hemera](https://github.com/cyberia-to/hemera)), one field, from source
to proof. Twenty other engines and twenty-five unions are declared
behind `--target`, Triton VM with its STARK is the second tested one,
and the same formula compiles to native code for 28 backends — from
Cortex-M and CUDA to OpenQASM and Verilog.

```
cargo install trident-lang cyber-joy
```

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

```
$ trident build hello_proof.tri
Compiled -> hello_proof.nox
$ trident prove hello_proof.tri --secret 7,13
Proved in 8 ms: 17 reductions, 1 accumulator groups, 1387 bytes
Output: [20]
hello_proof.zheng
$ trident verify hello_proof.zheng
Verification: PASS (zheng proof)
  program: hello_proof
  output:  [20]
  cycles:  17
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
| prove | [zheng](https://github.com/cyberia-to/zheng) via joy | SuperSpartan + Brakedown + HyperNova folding; one universal step CCS — constant-size proofs |
| verify | joy | checks the proof against the statement — no re-execution |
| state | [bbg](https://github.com/cyberia-to/bbg) | `os.state.read` lowers to the look pattern; reads carry proofs, the public root is in the statement |
| algebra · hash | [strata](https://github.com/cyberia-to/strata) · [hemera](https://github.com/cyberia-to/hemera) | Goldilocks arithmetic and Poseidon2 inside the compiler — no parallel implementations |

Measured on a laptop, release build, zheng 0.3.1: `(a+b)*a` proves in
4 ms into a 1.3 KB artifact; two `divine()` secrets in 8 ms / 1.4 KB;
one `hash` builtin in 55 ms / 2.4 KB; a depth-32 Merkle path (1,906
reductions) in 1.9 s / 2.6 KB. The proof is constant-size: one universal
step CCS folds every row into one accumulator, the opening bindings into
a second — any program is ≤ 2 groups, and every byte on the wire is
verifier-read.

---

## One Source, 21 Targets

A Trident program is written once against field elements; the target is
a config in `vm/`, not a rewrite. `trident build --target <engine>`
picks one. Levels are what exists, not what is planned — see
[reference/targets.md](reference/targets.md).

| level | engines |
|-------|---------|
| **tested** — end to end, proofs land | **nox** (default · zheng via joy) · **triton** (Triton VM · STARK via trisha) · **miden** |
| **costed** — scaffold backend + cost model, not yet tested | sp1 · openvm · cairo |
| **lowering** — pipeline wired, emission still stubbed | arm64 · x86-64 · riscv · nock |
| **declared + documented** — config and reference, no lowering | risczero · jolt · aztec · avm · evm · wasm · sbpf · movevm · polkavm · ckb · tvm |

Above the engines sit **25 unions** — the operating systems and chains
a program deploys into, each binding one engine (`os/`): neptune
(bound — runtime bindings in Trident, on triton); linux, macos,
android, browser, wasi on the native and wasm engines; ethereum,
arbitrum, solana, polkadot, ton, near, cosmwasm, icp, sui, aptos,
starknet, aztec, aleo, miden, nervos, nockchain, succinct, boundless,
openvm-network — declared and documented, awaiting bindings.

### The same formula, on silicon

A nox formula is a tree over 18 patterns — small enough to hand-emit
for any machine. `trident compile -t <backend>` turns the very formula
joy proves into native code for **28 backends**, and every one of them
emits real output for `hello.nox` today. These are emitters: execution
and proving on this hardware are not wired yet; the point is that one
program already speaks to all of it.

| class | backends |
|-------|----------|
| CPU | `x86-64` · `arm64` (JIT) · `rv64` · `rv32` (ESP32) · `rvv` (RISC-V vector) · `thumb2` (Cortex-M · STM32 · RP2040) · `hexagon` (Qualcomm DSP) |
| GPU | `ptx` (CUDA) · `tensor-cores` (wmma) · `wgsl` (WebGPU) · `spirv` (Vulkan) |
| accelerators | `ane` (Apple Neural Engine) · `amx` (Apple matrix) · `intel-amx` · `xla` (TPU) · `onnx` · `cerebras` (wafer-scale CSL) · `upmem` (processing-in-memory) |
| kernel · web | `ebpf` (Linux kernel) · `wasm` (browser · WASI · every wasm chain) |
| quantum | `qasm` (OpenQASM 3.0 circuits) · `qir` (Quantum IR · Azure Quantum) |
| hardware | `verilog` (FPGA) · `systemverilog` (ASIC) · `vhdl` |

```
$ trident build hello.tri                      # hello.nox
$ trident compile hello.nox -t qasm -o hello.qasm
$ trident compile hello.nox -t verilog -o hello.v
$ trident compile hello.nox -t spirv -o hello.spv
```

### Neptune (`--target triton`)

[Neptune Cash](https://neptune.cash/) is the only blockchain with
recursive STARK proofs in production — a proof verifies another proof
inside itself, so any chain of transactions collapses into a single
cryptographic check. Trident targets its [Triton VM](https://triton-vm.org/)
through the [trisha](https://github.com/cyberia-to/trisha) warrior, and
these programs are proposed Neptune standards — written in Trident,
compiling to TASM today, under validation before deployment:

| Program | What it proposes |
|---------|-----------------|
| [Coin](os/neptune/standards/coin.tri) | Fungible token (TSP-1) — pay, lock, mint, burn, composable hooks |
| [Card](os/neptune/standards/card.tri) | Non-fungible token (TSP-2) — royalties, creator immutability |
| [Lock scripts](os/neptune/locks/) | Multisig, timelock, symmetric spending authorization |
| [Type scripts](os/neptune/types/) | Token conservation laws verified in every transaction |
| [Programs](os/neptune/programs/) | Recursive verification, proof aggregation, relay |

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

## Trusting, Not Trust

You download a compiler binary. Someone compiled it — you trust them.
They used a compiler too — you trust that one as well. The trust chain
stretches back to the first hand-assembled binary, and every link is
opaque. Ken Thompson showed in 1984 that a compiler can inject
backdoors invisible in the source.

Trident breaks the chain. The compiler self-hosts: Trident source
compiles Trident source, and the execution produces a proof that
compilation was faithful. Not "we audited the binary." Not "we
reproduced the build." A cryptographic proof, from the mathematics
itself, that the output corresponds to the input.

Three producers compete on the same scoreboard:

```
$ trident bench baselines/triton/std/compiler

Module                       Tri   Hand Neural   Ratio
-------------------------------------------------------
std::compiler::lexer         288      8      -  36.00x
std::compiler::parser        358      8      -  44.75x
std::compiler::pipeline        0      1      -   0.00x
```

`Tri` — compiler output. `Hand` — expert-written assembly (the floor).
`Neural` — a [13M-parameter GNN+Transformer](reference/neural.md)
learning to emit better assembly than the compiler. The dashes mean the
model is training. When it beats the compiler, the number appears.
`trident bench` also reports a `Nox(r)` column — reductions, for the
modules inside the nox surface.

`src/` is the Rust bootstrap — it shrinks.
`std/compiler/` is the Trident replacement — it grows.

---

## Quick Start

```
cargo install trident-lang cyber-joy       # the compiler + the nox warrior
trident build main.tri                     # compile to .nox (--target triton for TASM)
trident run main.tri --input-values 3,5    # execute on nox (via joy)
trident prove main.tri                     # zheng proof -> main.zheng
trident verify main.zheng                  # verify, no re-execution
trident check main.tri                     # type-check only
trident test main.tri                      # run #[test] functions
trident fmt main.tri                       # format source
trident audit main.tri                     # formal verification
trident bench main.tri                     # cost: reductions (nox), instructions (triton)
```

The Triton path needs [trisha](https://github.com/cyberia-to/trisha)
on `PATH` the same way the nox path needs `joy`.

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
vm/           Engines + intrinsics        21 target.toml profiles; intrinsics in Trident
std/          Standard library in Trident Crypto, math, neural networks, compiler
os/           Unions in Trident           25 per-OS configs, programs, and extensions
tests/        nox_surface.rs, differential.rs — every lowering reduces on the real VM
```

```
vm.*              Compiler intrinsics       hash, sponge, divine, assert
std.*             Standard library          sha256, bigint, ecdsa, poseidon2
os.*              Portable runtime          os.signal, os.neuron, os.state, os.time
os.<target>.*     Target-specific APIs      os.neptune.xfield, os.solana.pda
```

The warriors live beside the compiler:
[joy](https://github.com/cyberia-to/joy) (nox, the cyber battlefield) and
[trisha](https://github.com/cyberia-to/trisha) (Triton VM, Neptune).

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

0.2.0 / 500K — **Cast**. The language poured into the soft3 mold: nox
by default, a real prover, one algebra, one hash. Hot, not production
ready. Kelvin versioning counts down toward 0K; the
[roadmap](reference/roadmap.md) says what cools next.

Treat it as experimental unless you already understand the constraints
you are adopting. The architecture is built to expand targets over time,
without changing what a Trident program is.

---

## License

[Cyber License](https://github.com/mastercyb/cyber/blob/master/graph/cyber/license.md)

Don't trust. Verify. Don't fear. Publish. Don't beg. Build.
