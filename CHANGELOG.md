# Changelog

Kelvin versioning: versions count down toward 0K (frozen forever).
Lower is colder. Colder is more stable.

## 0.2.0 — Unreleased

### BREAKING: content hashes are now cyber-hemera

One Poseidon2 on the planet, and it is hemera's. The compiler's in-tree
Poseidon2 (`src/package/poseidon2.rs`, BLAKE3-derived constants,
t=8/RF=8/RP=22) is deleted; every content identifier — `trident deploy`
digests, package hashes, function identity from normalized AST — is now
`cyber-hemera`'s hash. All previously computed content hashes change
value. Representative old → new digests:

| input | old (in-tree Poseidon2) | new (cyber-hemera) |
|---|---|---|
| `""` | `0c3c1eaef3b8f8d5…8de91b59` | `a67a71b221e6bdd6…806d563c` |
| `"trident"` | `0027d32685465f66…26026a86` | `5386a7e49142b577…abc9df71` |
| `"fn main() { pub_write(42) }"` | `5f4ccf1c8d38936c…5864e2dd` | `1c315c3651929e24…cd2013bd` |

The API `trident::poseidon2::hash_bytes` is gone; use
`trident::hash::content_hash_bytes`. The Trident-language Poseidon2
library (`std/crypto/poseidon2.tri`) and its Rust ground-truth fixture
are unchanged — they are a language-level pair, not compiler
infrastructure.

The compiler's Goldilocks arithmetic is now `strata-nebu`'s (bit-exact
migration — field arithmetic values are unchanged; only the
implementation moved).

## 0.1.0 / 512K — Smelt (2026-02-26)

First public release. Hot, experimental, not production ready.

### Compiler

- Full pipeline: source → lexer → parser → AST → typecheck → TIR → optimizer → lowering → TASM
- 912 tests passing, zero warnings
- 54-operation intermediate representation mapping ~1:1 to target instructions
- Static cost analysis: exact proving cost from source, before execution

### CLI

`build`, `check`, `test`, `fmt`, `audit`, `bench`, `lsp`, `package`, `deploy`

### Standard Library

- `std.crypto`: poseidon2, poseidon, sha256, keccak256, ecdsa, secp256k1, ed25519, merkle, bigint, auth
- `std.nn`: tensor, dense, attention, convolution, lookup-table activations
- `std.private`: polynomial ops, NTT for FHE
- `std.quantum`: gate set (H, X, Y, Z, S, T, CNOT, CZ, SWAP)
- `std.compiler`: lexer, parser, typechecker, codegen, optimizer, lowering, pipeline (9,195 lines of self-hosted Trident)

### Neptune Programs

- Coin (TSP-1): fungible token — pay, lock, update, mint, burn
- Card (TSP-2): non-fungible token — royalties, creator immutability
- Lock scripts: generation, symmetric, timelock, multisig
- Type scripts: native currency, custom token conservation
- Programs: transaction validation, recursive verification, proof aggregation

### Tooling

- Language Server Protocol (`trident-lsp`): diagnostics, completions, hover, go-to-definition
- Editor support: Zed extension, Helix config
- Formal verification: `trident audit` with `#[requires]`/`#[ensures]` contracts
- Neural optimizer: 13M-parameter GNN+Transformer learning to emit optimized TASM
- Benchmark scoreboard: `trident bench` comparing compiler vs hand-written vs neural output

### Install

```
cargo install trident-lang
```

### License

Cyber License: Don't trust. Don't fear. Don't beg.
