# Trident

> The weapon is their language. They gave it all to us.
> If you learn it, when you really learn it, you begin to perceive time
> the way that they do. So you can see what's to come.

<p align="center">
  <img src="media/tri.gif" width="100%" alt="Trident" />
</p>

Trident is a provable programming language.

Every variable, every operation, every function compiles to arithmetic
over the Goldilocks prime field (p = 2^64 - 2^32 + 1). Programs produce
STARK proofs — hash-based, post-quantum secure, no trusted setup.

---

## Hello, Proof

```trident
program hello_proof

fn main() {
    let a: Field = secret_read()
    let b: Field = secret_read()
    pub_write(a + b)
}
```

```
$ trident build hello.tri
$ trident prove hello --secret 7 --secret 13
  Proof generated (924 cycles, 11 KB)
$ trident verify hello
  Valid: output = 20, inputs hidden
```

A cryptographic proof that `a + b = 20` without revealing `a` or `b`.
Quantum-safe. Zero-knowledge. No trusted setup. No elliptic curves.

---

## The Mental Model

```
.tri source
  |
  |  trident build
  v
Target assembly (TASM)  +  static cost report
  |
  |  target VM executes
  v
Execution trace
  |
  |  target VM proves
  v
STARK proof + claim
  |
  |  target VM verifies
  v
true / false
```

Trident owns **source -> assembly + cost**. The backend owns
**execute -> trace -> prove -> verify**. The compiler exists to expose
cost, not hide it.

```
$ trident build coin.tri --cost
  Total: 14,832 cycles
  Hash:   8,440 (57%)
  Field:  4,192 (28%)
  Stack:  2,200 (15%)
```

You know the proving bill before you run.

---

## Neptune

[Neptune Cash](https://neptune.cash/) is where Trident programs run.
It is the only blockchain with recursive STARK proofs in production —
a proof verifies another proof inside itself, so any chain of
transactions collapses into a single cryptographic check. No other
chain does this today.

Neptune is programmable, private, mineable, and quantum-safe. Trident
is its native language, targeting [Triton VM](https://triton-vm.org/).

The following programs are proposed standards — specifications written
in Trident, compiling to TASM today, under validation before deployment:

| Program | What it proposes |
|---------|-----------------|
| [Coin](os/neptune/standards/coin.tri) | Fungible token (TSP-1) — pay, lock, mint, burn, composable hooks |
| [Card](os/neptune/standards/card.tri) | Non-fungible token (TSP-2) — royalties, creator immutability |
| [Lock scripts](os/neptune/locks/) | Multisig, timelock, symmetric spending authorization |
| [Type scripts](os/neptune/types/) | Token conservation laws verified in every transaction |
| [Programs](os/neptune/programs/) | Recursive verification, proof aggregation, relay |

These compile and pass tests. They are not yet deployed. The
interesting thing is not the tokens — it's that every transaction on
Neptune already carries a recursive proof, and these programs extend
what those proofs can express.

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

The gap is not marginal:

| Operation | Trident on Triton VM | Rust on SP1 | Rust on RISC Zero |
|-----------|:---:|:---:|:---:|
| One hash (Tip5 / SHA-256) | 1 cycle | ~3,000 cycles | ~1,000 cycles |
| Merkle proof (depth 32) | ~100 cycles | ~96,000 cycles | ~32,000 cycles |

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
| STARK lookup | Proof authentication | Verifiability |

One table. One field. Four purposes. When all systems operate over the
same prime field, four separate mechanisms collapse into one data
structure read four ways.

[Trinity](docs/explanation/trinity-bench.md) demonstrates this: a single
Trident program encrypts input with LWE, runs a neural layer, hashes
with Tip5, performs FHE bootstrapping, and commits via a quantum
circuit — all inside one STARK trace.

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

Every function has a unique cryptographic identity derived from its
normalized AST. Names are metadata. The hash is the truth.

```
$ trident deploy transfer.tri
  Hash:     #a7f3b2c1d4e8
  Verified: (audit certificate attached)
  Cost:     47 cycles
  Published to registry
```

Rename a function — the hash doesn't change. Publish independently
from the other side of the planet — same code, same hash. Verification
certificates travel with the identity, not the name.
See [Content-Addressed Code](docs/explanation/content-addressing.md).

---

## Trusting, Not Trust

You download a compiler binary. Someone compiled it — you trust them.
They used a compiler too — you trust that one as well. The trust chain
stretches back to the first hand-assembled binary, and every link is
opaque. Ken Thompson showed in 1984 that a compiler can inject
backdoors invisible in the source.

Trident breaks the chain. The compiler self-hosts: Trident source
compiles Trident source, and the execution produces a STARK proof that
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

`src/` is the Rust bootstrap — it shrinks.
`std/compiler/` is the Trident replacement — it grows.

---

## Quick Start

```
cargo build --release
trident build main.tri           # compile to TASM
trident check main.tri           # type-check only
trident test main.tri            # run #[test] functions
trident fmt main.tri             # format source
trident audit main.tri           # formal verification
trident bench main.tri           # instruction count + cost
```

---

## Design Principles

1. **Field elements all the way down.** The machine word is `Field`, not `u64`.
2. **Bounded execution.** Explicit loop bounds. No recursion. No halting problem.
3. **Compile-time everything.** Types, array sizes, and costs known statically.
4. **Constraints are features.** No heap, no dynamic dispatch — safety guarantees.
5. **Provable first.** Designed for ZK. These constraints make great conventional programs too.
6. **Minimal dependencies.** 5 runtime crates: clap, ariadne, blake3, tower-lsp, tokio.

---

## Source Tree

```
src/          Compiler in Rust            ~36K lines, 5 runtime dependencies
vm/           VM intrinsics in Trident    Compiler primitives (hash, I/O, field ops)
std/          Standard library in Trident Crypto, math, neural networks, compiler
os/           OS bindings in Trident      Per-OS config, programs, and extensions
```

```
vm.*              Compiler intrinsics       hash, sponge, pub_read, assert
std.*             Standard library          sha256, bigint, ecdsa, poseidon2
os.*              Portable runtime          os.signal, os.neuron, os.state, os.time
os.<target>.*     Target-specific APIs      os.neptune.xfield, os.solana.pda
```

---

## Standard Library

**Implemented:** `std.field` · `std.crypto` · `std.math` · `std.data` ·
`std.io` · `std.compiler`

**In development:** `std.nn` (field-native neural networks) ·
`std.private` (ZK + FHE + MPC) · `std.quantum` (gates, error
correction)

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

This is a language from the future, under construction in the present.

Treat it as experimental unless you already understand the constraints
you are adopting. The architecture is built to expand targets over time,
without changing what a Trident program is.

---

## License

[Cyber License](https://github.com/mastercyb/cyber/blob/master/license.md)

Don't trust. Verify. Don't fear. Publish. Don't beg. Build.
