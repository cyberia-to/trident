# Portable cryptographic arithmetic

## Standard Poseidon2-HL

`std.crypto.poseidon` protocol version 2 implements the exact Goldilocks
Poseidon2-HL permutation at width eight: eight full rounds, 22 partial rounds,
x^7 S-boxes, the Horizen Labs external matrix and the upstream internal
diagonal. All 86 round constants are fixed in source. Parameters and test
oracles are pinned to `p3-goldilocks = 0.4.2`, upstream revision
`0835481398d2b481bef0c6d0e8188b484ab9a636`.
[Upstream permutation and constants](https://github.com/Plonky3/Plonky3/blob/0835481398d2b481bef0c6d0e8188b484ab9a636/goldilocks/src/poseidon2.rs).

`permute(State) -> State` exposes all eight lanes. `State` has fields `s0`
through `s7`. `hash1_digest` through `hash4_digest` accept the corresponding
number of Field arguments and return `(Field, Field, Field, Field)`. This
fixed four-limb result is independent of the target's `Digest` type width.
Inputs fill lanes zero through three, unused input lanes are zero, lane four
contains the input length, and remaining capacity lanes are zero. The result
is lanes zero through three after one permutation.

The existing `hash1` through `hash4` signatures still return one Field. These
are explicitly truncated hashes with a generic collision ceiling of roughly
32 bits and a preimage ceiling of roughly 64 bits. They cannot serve as
128-bit collision-resistant identifiers. Use the four-limb APIs when a full
result is required; their generic collision ceiling is roughly 128 bits.
These ceilings are output-size bounds, not an independent security audit of
this integration or its application protocol.

Version 2 deliberately changes every old dummy hash output. Protocols using
those outputs must version their commitments and migrate stored values; the
old four-round arithmetic has no compatibility mode. This change does not
replace Hemera, change nox's hash intrinsic, or migrate network state roots.
Hemera's separate parameter set and security review remain separate concerns.

`std.crypto.poseidon2` retains the historical caller-parameter arithmetic API.
Its matrices are custom and its constants are caller inputs. It is not the
standard permutation above and has no independently established cryptographic
security guarantee. New fixed-parameter hashing uses `std.crypto.poseidon`.

The upstream-oracle runtime tests live in Trisha's
`rs/tests/poseidon_standard.rs`; they exercise all lanes, canonical boundary
values, full output and explicit truncation on Triton VM. The independent
hand baseline uses scratch RAM `[2000000,2000016)`; its generator is
`trisha/scripts/generate_poseidon_baseline.py`, with pinned constants and
dense matrix arithmetic. The zero/range benchmark fixtures check all eight
lanes against upstream outputs. Callers composing that hand baseline must
reserve its scratch range. The portable source has no such RAM restriction.

## U256 modular arithmetic

`std.crypto.bigint.U256` stores eight little-endian U32 limbs. `sub256` returns
the low 256 bits of subtraction, including wrapped subtraction. `add_mod`
requires reduced operands `a,b < m`, and retains the 257th addition carry
before reducing. `mul_mod` accepts arbitrary U256 operands with any nonzero
U256 modulus and returns a canonical residue in `[0,m)`. Zero modulus fails;
modulus one returns zero.

Multiplication first reduces the multiplicand with 256 binary steps, then
uses 256 modular double/add steps. Every modular addition uses reduced
operands and handles overflow before subtraction. The implementation avoids
truncating an unreduced 512-bit product. Its control flow depends on operand
bits; it does not promise constant-time host execution.

Independent full-width residue vectors, addition-overflow cases and zero
modulus checks live in Trisha's `rs/tests/bigint_mod.rs`. Compiler regressions
for aggregate call/assignment and branch-local tuple returns are retained
beside these tests. `rs/tests/bigint_boundaries.rs` additionally checks frozen
add/sub/reduction vectors under both source optimization profiles. Runtime acceptance of those tests is a release gate for
this arithmetic implementation.

## ECDSA scalar policy

`std.crypto.ecdsa` provides representation, exact U32 input/output and scalar
policy helpers; it does not implement elliptic-curve signature verification.
`valid_range(sig,order)` requires both scalars in `[1,order-1]`.
`is_low_s(sig,order)` checks `1 <= s <= floor(order/2)` by an exact eight-limb
right shift, including cross-limb carries; it does not check r. Both helpers
are needed when a caller requires valid r and normalized s. The previous
`is_low_s` placeholder accepted high-S values and is intentionally corrected.
Trisha's `crypto_policy` regression checks half-order, half-order+1, zero,
order, invalid r, even order and cross-limb boundaries under both profiles.

## Full Keccak-f[1600]

`std.crypto.keccak256.keccak_f1600(State)` and
`keccak_f1600_in_ram(state_addr,scratch_addr)` compute the same full 24-round,
25-lane permutation. Neither API supplies message padding or Ethereum trie
verification. Independent vectors check all 50 U32 output words against the
[Keccak specification](https://keccak.team/keccak_specs_summary.html).

The RAM API owns no allocator. The caller supplies 50 state words and 70
scratch words, all outside other live data or explicitly reserved runtime memory.
Both exclusive ends must fit U32 and the regions must not overlap; adjacent
regions are allowed. Every initial state word must fit U32. The API rejects
invalid regions and limbs before mutating either buffer, and writes only these
regions. Scratch is temporary and unspecified after return. It requires the
selected target's RAM capability, so a nox entry reaching it is rejected during
capability checking. This capability restriction is independent of arithmetic
correctness on a RAM-capable target.

The Trisha hand reference uses fixed RAM state `[0,50)` and scratch
`[1000,1252)` plus round constants `[1300,1302)`; a composing caller reserves
those regions. Its implementation uses U32 limbs independently of the Python
oracle's 64-bit integers. The benchmark explicitly selects the RAM source API;
`crypto_policy` retains the aggregate API against the same full outputs.

## SHA-256 RAM compression

`std.crypto.sha256.compress_in_ram(state_addr,block_addr,scratch_addr)` consumes
an eight-word chaining state and a sixteen-word block, computes all 64 schedule
words and rounds, and replaces the state with the feed-forward result. It leaves
the block unchanged and uses 72 scratch words. The three regions must be pairwise
disjoint, outside other live or implementation-reserved memory, and have U32
exclusive ends. All initial state/block words must fit U32; validation precedes
mutation. Scratch contents are unspecified afterwards.

`double_sha256_single_block_in_ram(block_addr,out_addr,scratch_addr)` takes a
caller-padded sixteen-word block, returns all eight double-SHA256 digest words in
`out_addr`, and needs 88 scratch words. It applies the standard IV for both
hashes and constructs the exact second-hash padding for the first 256-bit digest.
The input block, output and entire scratch region are pairwise disjoint. It does
not pad arbitrary-length messages: the single-block input padding remains the
caller's responsibility. Original aggregate APIs remain available and are checked
against the same independent `hashlib` vectors as the RAM path.
