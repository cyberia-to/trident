---
status: draft
date: 2026-05-14
---
# warrior-cyber: PoC execution + proving on nox/zheng

## context

The only working warrior today is trisha — it takes compiled TASM from
trident, executes on Triton VM, and proves with Triton's FRI/STARK prover.
Triton is not the primary target. nox is.

`trident build --target nox` produces Noun trees via NounBuilder. There is
no warrior to execute or prove them. The trident → nox → zheng pipeline is
wired in three separate repos but never end-to-end.

This proposal specifies a PoC warrior that closes the pipeline. One binary.
Four commands. Three proving backends: CPU (acpu AMX/NEON), WebGPU (wgpu,
cross-platform including browser), Metal (aruminium, Apple Silicon native).

## what warrior-cyber does

```
warrior-cyber run   <file.tri>     # compile + execute on nox, print result
warrior-cyber prove <file.tri>     # compile + execute + prove with zheng
warrior-cyber verify <proof.cyb>   # verify proof, print claim
warrior-cyber bench <file.tri>     # run + prove + time each stage
```

The warrior receives a `.tri` source file (PoC), compiles it to a Noun via
NounBuilder, reduces the Noun with nox, traces execution, and proves the
trace with zheng. Optionally: receives a pre-compiled `ProgramBundle` (the
production path once `trident build --target nox` is wired to emit bundles).

## pipeline

```
.tri source   witness tape (.wit)
    │               │
    ▼               ▼
trident::build_noun(src)        # NounBuilder: AST → Noun
    │
    ▼
nox::reduce(noun, input, calls) # 16 patterns + hint (Layer 2)
    │  calls: FifoCallProvider from witness tape
    │  returns (output, ExecutionTrace)
    ▼
zheng::prove(trace, claim)      # SuperSpartan + sumcheck + Brakedown
    │  internally calls lens::brakedown::commit(trace_evals)
    │  field ops via honeycrisp::acpu::field::goldilocks
    ▼
Proof + Claim
    │
    ▼
zheng::verify(proof, claim)     # verifier: < 100ms on M-series
```

## hint — non-deterministic witness

nox Layer 2 is pattern 16 (`call_witness`). The prover injects a witness
atom; the pattern evaluates a `check_formula` against it; if check returns 0
the witness is accepted and written into the trace. zheng proves the check
passed — the verifier never sees the witness itself.

```rust
pub trait CallProvider<const N: usize>: LookProvider {
    fn provide(&self, order: &mut Order<N>, tag: Goldilocks, object: NounId)
        -> Option<NounId>;
}
```

`reduce()` already accepts `hints: &dyn CallProvider<N>`. The PoC passes
`NullCalls` today; warrior-cyber replaces it with `FifoCallProvider` — a
FIFO tape of field elements indexed by call tag:

```rust
struct FifoCallProvider {
    tape: HashMap<u64, VecDeque<Goldilocks>>,  // tag → values in FIFO order
}
```

Witness tape format: newline-separated `tag:value` pairs. The warrior reads
a `.wit` file and populates the tape before reduction begins.

For trinity, the bench reference (`benches/references/std/trinity/inference.rs`)
already computes all required values in `compute_bench_divine()`. The witness
tape is the FIFO of divine values the prover must supply:

| call site | tag | values supplied |
|-----------|-----|-----------------|
| Phase 1b: `lwe.decrypt` per neuron | 0x01 | plaintext m for each ciphertext (32 values) |
| Phase 3: LUT sponge `reduce_mod` | 0x02 | (r, k) pairs — 14 rounds × 8 elems × 2 = 224 values |
| Phase 4: PBS `build_test_poly` | 0x03 | table_idx for each ring position (128 values) |
| Phase 4: PBS `blind_rotate` | 0x04 | rotation, then src+sign pairs (1 + 2×128×2 values) |
| Phase 4: PBS `key_switch` | 0x05 | switched ciphertext components (17 values) |
| Phase 4: PBS `lwe.decrypt` | 0x01 | final plaintext (1 value) |

Privacy property: the secret key `s` and all intermediate ciphertexts stay
in the prover's witness tape. The proof covers only the constraint that each
decryption satisfied the noise bound check. The verifier learns nothing about
`s`.

## proving backends

Three implementations behind one trait. The prover selects at startup via
`--backend cpu|webgpu|metal`. All three produce identical proofs — same
zheng protocol, same Brakedown PCS, different field arithmetic engines.

```rust
trait ProveBackend {
    fn commit(&self, evals: &[Field]) -> Commitment;
    fn batch_mul(&self, a: &[Field], b: &[Field], out: &mut [Field]);
    fn merkle_root(&self, leaves: &[Field]) -> Digest;
}
```

### cpu — acpu (AMX + NEON)

The portable baseline. Runs on any aarch64 machine, no GPU required.
Maps to existing honeycrisp primitives directly:

| zheng stage | operation | acpu call |
|-------------|-----------|-----------|
| Brakedown encoding | linear-code matvec over F_p | `acpu::gemm::matvec_gl` (AMX tiles) |
| sumcheck rounds | batched field multiply-accumulate | `acpu::field::goldilocks::gl_mul_batch` (NEON) |
| witness Merkle | tree hashing over trace | `acpu::field::merkle::merkle_root` |
| permutation | trace column permutation | `acpu::field::permute` |

acpu already has 321 LOC of Goldilocks ops and 2,855 LOC of AMX GEMM.
Integration is bridging, not building from scratch.

Target: < 10s for trinity (18–20K trace).

### webgpu — wgpu crate

Cross-platform GPU. Runs on any device with a WebGPU-capable driver:
Metal (macOS/iOS), Vulkan (Linux/Android), DX12 (Windows), and the browser
via WebAssembly + the WebGPU API.

Hot operations become WGSL compute shaders:
- `brakedown_encode.wgsl` — row-wise field matvec, one thread per row
- `sumcheck_round.wgsl` — batched mul-add for sumcheck polynomial folding
- `merkle_layer.wgsl` — pairwise hemera hashing up the Merkle tree

The wgpu backend is the portability story: the same warrior binary proving
on Apple Silicon, Linux CI, and browser playground. aruminium is Metal-only;
wgpu covers everything else.

Target: < 3s for trinity on discrete GPU; < 1s on M3 via Metal adapter.

### metal — aruminium (Apple Silicon native)

Maximum throughput on Mac. aruminium directly calls Metal APIs via
Objective-C FFI (2,286 LOC of `Gpu`, `Pipeline`, `Buffer`, `Dispatch`).
Same WGSL shaders as the wgpu backend compile to Metal Shading Language at
runtime, but aruminium's `Dispatch` layer skips wgpu's validation overhead
and uses Metal's `MTLComputeCommandEncoder` directly.

Additional Metal-specific optimisations:
- `unimem::IOSurface` for zero-copy transfer between acpu (AMX encoding)
  and aruminium (GPU hashing) — the trace buffer is shared, no memcpy
- AMX for Brakedown linear encoding (CPU-side), Metal for parallel Merkle
  hashing (GPU-side) — the two operations pipeline across PCIe-less UMA

Target: < 1s for trinity (UMA removes transfer overhead entirely).

## trinity parameters

The trinity demo (`std.trinity.inference`) serves as the reference program
for warrior-cyber. Current "pitch parameters" are geometrically inconsistent:
RING_N=64 cannot bootstrap PLAINTEXT_SPACE=1024 (requires RING_N ≥ 2×DOMAIN).

Corrected PoC parameters:

```
LWE_N          = 16     # LWE dimension (80-bit security at Goldilocks prime)
INPUT_DIM      = 16     # feature vector size
NEURONS        = 32     # hidden layer width (32×16 weight matrix = 512 weights)
PLAINTEXT_BITS = 6      # 6-bit message space
DOMAIN         = 64     # LUT domain size (= 2^PLAINTEXT_BITS)
RING_N         = 128    # ring dimension for PBS (≥ 2×DOMAIN, required)
```

Trace estimate:
- Phase 1 LWE matvec: 16 × 32 × 2×16 ≈ 16 K field ops
- Phase 2 dense + LUT: 32×16 + 32 lookups ≈ 550 ops
- Phase 3 Poseidon2 + LUT sponge: ≈ 300 ops
- Phase 4 PBS (NTT): 128 × log₂(128) × 2 ≈ 1.8 K ring ops
- Phase 5 Bell circuit: ≈ 20 field ops

Total: ~18 K–20 K nox pattern applications.

Proof targets (trinity, 18–20K trace):

| metric | cpu (AMX) | webgpu | metal (aruminium) |
|--------|-----------|--------|-------------------|
| nox execution | < 50 ms | < 50 ms | < 50 ms |
| Brakedown commit | < 2 s | < 500 ms | < 200 ms |
| sumcheck prove | < 5 s | < 1 s | < 500 ms |
| total prove time | < 10 s | < 3 s | < 1 s |
| proof size | < 100 KB | < 100 KB | < 100 KB |
| verify time | < 100 ms | < 100 ms | < 100 ms |

Proof size and verify time are backend-independent — same protocol, same
Brakedown PCS, same proof bytes. These are engineering targets, not claimed
results. `warrior-cyber bench` reports actuals on first run.

## crate structure (PoC)

```
warrior-cyber/
├── Cargo.toml
└── src/
    ├── main.rs              # CLI: subcommand dispatch, --backend flag
    ├── compile.rs           # .tri → Noun via trident NounBuilder
    ├── execute.rs           # Noun + witness → (output, ExecutionTrace) via nox::reduce
    ├── witness.rs           # FifoCallProvider: .wit tape → CallProvider<N>
    ├── prove.rs             # ExecutionTrace → Proof (dispatches to backend)
    ├── verify.rs            # (Proof, Claim) → bool via zheng
    └── backend/
        ├── mod.rs           # ProveBackend trait
        ├── cpu.rs           # acpu: AMX GEMM + NEON field ops
        ├── webgpu.rs        # wgpu: WGSL shaders, cross-platform GPU
        └── metal.rs         # aruminium: Metal native + unimem zero-copy
```

Dependencies:
```toml
[dependencies]
trident       = { path = "../trident" }
nox           = { path = "../nox/rs" }
zheng         = { path = "../zheng" }
lens          = { path = "../lens" }
strata        = { path = "../strata" }
hemera        = { package = "cyber-hemera", path = "../hemera/rs" }
honeycrisp    = { path = "../honeycrisp" }
wgpu          = { version = "22", features = ["webgpu"] }
clap          = { version = "4", features = ["derive"] }

[target.'cfg(target_os = "macos")'.dependencies]
honeycrisp    = { path = "../honeycrisp" }   # aruminium + acpu + unimem
```

## what is not in scope (PoC)

- Network deployment (radio, bbg) — local only
- os.cyber.* programs — trident + nox target only, no OS layer
- Recursive proof composition — flat proofs only
- ANE (rane) acceleration — inference runtime path, not prover path

These are 128K items. The PoC proves that the pipeline closes across all
three proving backends, including full hint/witness support for trinity.
Correctness before networked deployment.

## relationship to trisha

trisha is the Triton warrior. warrior-cyber is the nox warrior. They share
no code except the trident ProgramBundle format. warrior-cyber does not
depend on trisha or any Triton VM library.

Long-term: warrior-cyber is the primary warrior. trisha remains for
Triton/Neptune programs. The warrior-architecture.md split (core vs
tooling vs warriors) keeps them independent.

## estimate

| task | sessions |
|------|----------|
| Cargo workspace + deps wired | 0.5 |
| compile.rs: trident → Noun | 0.5 |
| execute.rs: nox::reduce integration | 1 |
| witness.rs: FifoCallProvider + .wit tape format | 1 |
| prove.rs: ProveBackend trait + dispatch | 0.5 |
| zheng + lens::brakedown wiring | 2 |
| backend/cpu.rs: acpu AMX/NEON bridge | 1 |
| backend/webgpu.rs: wgpu + WGSL shaders | 2 |
| backend/metal.rs: aruminium + unimem zero-copy | 1.5 |
| verify.rs | 0.5 |
| CLI + --backend flag + bench command | 0.5 |
| trinity parameters fixed in .tri + reference | 0.5 |
| integration test: trinity proves on all 3 backends | 1 |
| **total** | **~12.5 sessions** |

depends on: nox::reduce stable API, zheng::prove accepting ExecutionTrace,
lens::brakedown::commit callable from zheng. All three exist; API alignment
may add 1 session.
