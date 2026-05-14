// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use std::time::Instant;
use trident::field::{poseidon2, Goldilocks, PrimeField};

type F = Goldilocks;

const HALF_P: u64 = (0xFFFF_FFFF_0000_0001u64 - 1) / 2;

// PoC parameters (corrected for PBS feasibility):
//   LWE_N=16, INPUT_DIM=16, NEURONS=32, PLAINTEXT_BITS=6,
//   DOMAIN=64, RING_N=128 (≥ 2×DOMAIN, required for PBS).
const LWE_N: usize = 16;
const INPUT_DIM: usize = 16;
const NEURONS: usize = 32;
const DOMAIN: u64 = 64;
const RING_N: usize = 128;

fn delta() -> F {
    let p = 0xFFFF_FFFF_0000_0001u64;
    F::from_u64(p / DOMAIN)
}

// ===========================================================================
// Phase 1: LWE encryption
// ===========================================================================

fn inner_product(a: &[F], s: &[F]) -> F {
    let mut sum = F::ZERO;
    for i in 0..a.len() {
        sum = sum.add(a[i].mul(s[i]));
    }
    sum
}

struct Ciphertext {
    a: Vec<F>,
    b: F,
}

fn encrypt(m: F, s: &[F], a: &[F], e: F, delta: F) -> Ciphertext {
    let dot = inner_product(a, s);
    let b = dot.add(m.mul(delta)).add(e);
    Ciphertext {
        a: a.to_vec(),
        b,
    }
}

fn decrypt(ct: &Ciphertext, s: &[F], delta: F) -> F {
    let dot = inner_product(&ct.a, s);
    let phase = ct.b.sub(dot);
    let phase_u64 = phase.to_u64();
    let delta_u64 = delta.to_u64();
    let half_delta = delta_u64 / 2;
    let shifted = if phase_u64 <= HALF_P {
        phase_u64.wrapping_add(half_delta)
    } else {
        let neg_phase = 0xFFFF_FFFF_0000_0001u64 - phase_u64;
        let neg_m = (neg_phase + half_delta) / delta_u64;
        return F::from_u64(DOMAIN - neg_m);
    };
    F::from_u64(shifted / delta_u64)
}

fn ct_add(ct1: &Ciphertext, ct2: &Ciphertext) -> Ciphertext {
    let a: Vec<F> = ct1
        .a
        .iter()
        .zip(ct2.a.iter())
        .map(|(&a1, &a2)| a1.add(a2))
        .collect();
    Ciphertext {
        a,
        b: ct1.b.add(ct2.b),
    }
}

fn ct_scale(ct: &Ciphertext, k: F) -> Ciphertext {
    let a: Vec<F> = ct.a.iter().map(|&ai| ai.mul(k)).collect();
    Ciphertext {
        a,
        b: ct.b.mul(k),
    }
}

fn private_dot(cts: &[Ciphertext], weights: &[F]) -> Ciphertext {
    let n = cts[0].a.len();
    let mut out = Ciphertext {
        a: vec![F::ZERO; n],
        b: F::ZERO,
    };
    for i in 0..cts.len() {
        let scaled = ct_scale(&cts[i], weights[i]);
        out = ct_add(&out, &scaled);
    }
    out
}

fn private_linear(cts: &[Ciphertext], w: &[Vec<F>]) -> Vec<Ciphertext> {
    w.iter()
        .map(|row| private_dot(cts, row))
        .collect()
}

// ===========================================================================
// Phase 2: Dense neural layer with lookup-table activation
// ===========================================================================
// ReLU via precomputed lookup table over [0, DOMAIN).
// This is Reader #1 of the Rosetta Stone.

fn build_relu_lut() -> Vec<F> {
    let half = DOMAIN / 2;
    (0..DOMAIN)
        .map(|i| {
            if i < half {
                F::from_u64(i)
            } else {
                F::ZERO
            }
        })
        .collect()
}

fn lut_read(table: &[F], index: F) -> F {
    // In Triton VM, reading past the table returns 0 (uninit RAM).
    // In Rust reference, we emulate this behavior.
    let idx = index.to_u64() as usize;
    if idx < table.len() {
        table[idx]
    } else {
        F::ZERO
    }
}

fn matvec(mat: &[F], vec: &[F], rows: usize, cols: usize) -> Vec<F> {
    (0..rows)
        .map(|i| {
            let mut sum = F::ZERO;
            for j in 0..cols {
                sum = sum.add(mat[i * cols + j].mul(vec[j]));
            }
            sum
        })
        .collect()
}

fn dense(w: &[F], x: &[F], b: &[F], lut: &[F], rows: usize, cols: usize) -> Vec<F> {
    let mv = matvec(w, x, rows, cols);
    mv.iter()
        .zip(b.iter())
        .map(|(&v, &bi)| lut_read(lut, v.add(bi)))
        .collect()
}

// ===========================================================================
// Phase 3a: LUT sponge hash commitment (Rosetta Stone Reader #2)
// ===========================================================================
// Sponge with S-box reading from the same LUT as ReLU activation.
// State width 8, 14 rounds, circulant MDS.

const LUT_SPONGE_ROUNDS: usize = 14;
const LUT_SPONGE_WIDTH: usize = 8;

fn lut_sponge_sbox_layer(state: &mut [F; LUT_SPONGE_WIDTH], lut: &[F], domain: u64) {
    for i in 0..LUT_SPONGE_WIDTH {
        let reduced = state[i].to_u64() % domain;
        state[i] = lut[reduced as usize];
    }
}

fn lut_sponge_mds(state: &mut [F; LUT_SPONGE_WIDTH]) {
    let sum = state.iter().fold(F::ZERO, |acc, &x| acc.add(x));
    for i in 0..LUT_SPONGE_WIDTH {
        state[i] = state[i].add(sum);
    }
}

fn lut_sponge_add_constants(state: &mut [F; LUT_SPONGE_WIDTH], rc: &[F]) {
    for i in 0..LUT_SPONGE_WIDTH {
        state[i] = state[i].add(rc[i]);
    }
}

fn lut_sponge_permute(state: &mut [F; LUT_SPONGE_WIDTH], lut: &[F], domain: u64, rc: &[F]) {
    for r in 0..LUT_SPONGE_ROUNDS {
        let rc_offset = r * LUT_SPONGE_WIDTH;
        lut_sponge_add_constants(state, &rc[rc_offset..rc_offset + LUT_SPONGE_WIDTH]);
        lut_sponge_sbox_layer(state, lut, domain);
        lut_sponge_mds(state);
    }
}

fn lut_sponge_round_constants() -> Vec<F> {
    (0..LUT_SPONGE_ROUNDS * LUT_SPONGE_WIDTH)
        .map(|i| {
            let v = (i as u64 + 42).wrapping_mul(0x9E3779B97F4A7C15) % 0xFFFF_FFFF_0000_0001;
            F::from_u64(v)
        })
        .collect()
}

fn lut_hash_commit(
    activated: &[F],
    weights_digest: F,
    key_digest: F,
    class: F,
    lut: &[F],
) -> F {
    let output_digest = activated.iter().fold(F::ZERO, |acc, &x| acc.add(x));
    let mut state = [F::ZERO; LUT_SPONGE_WIDTH];
    state[0] = weights_digest;
    state[1] = key_digest;
    state[2] = output_digest;
    state[3] = class;
    state[4] = F::from_u64(4); // domain separation
    let rc = lut_sponge_round_constants();
    lut_sponge_permute(&mut state, lut, DOMAIN, &rc);
    state[0]
}

// ===========================================================================
// Phase 3b: Poseidon2 hash commitment — production binding
// ===========================================================================

fn hash_commit(
    activated: &[F],
    weights_digest: F,
    key_digest: F,
    class: F,
) -> F {
    let output_digest = activated.iter().fold(F::ZERO, |acc, &x| acc.add(x));
    let input = [weights_digest, key_digest, output_digest, class];
    let result = poseidon2::hash_fields_goldilocks(&input);
    result[0]
}

// ===========================================================================
// Phase 4: PBS demo (Rosetta Stone Reader #3)
// ===========================================================================
// Simplified PBS: decrypt + lookup in same table as NN activation.

fn pbs_demo(ct: &Ciphertext, s: &[F], d: F, lut: &[F]) -> F {
    let m = decrypt(ct, s, d);
    let m_u64 = m.to_u64();
    if m_u64 < DOMAIN {
        lut[m_u64 as usize]
    } else {
        F::ZERO
    }
}

// ===========================================================================
// Phase 5: Quantum commitment (2-qubit Bell pair)
// ===========================================================================

fn quantum_commit(class: usize) -> bool {
    let (q00, q01, mut q10, mut q11) = (F::ONE, F::ZERO, F::ZERO, F::ONE);
    if class != 0 {
        q11 = q11.neg();
    }
    std::mem::swap(&mut q10, &mut q11);
    let h00 = q00.add(q10);
    let h01 = q01.add(q11);
    let h10 = q00.sub(q10);
    let h11 = q01.sub(q11);
    let p0 = h00.mul(h00).add(h01.mul(h01));
    let p1 = h10.mul(h10).add(h11.mul(h11));
    let diff = p0.sub(p1);
    let hi = (diff.to_u64() >> 32) as u32;
    hi < 2147483647u32
}

// ===========================================================================
// Full pipeline
// ===========================================================================

fn argmax(v: &[F]) -> usize {
    let mut best = 0;
    let mut best_val = v[0].to_u64();
    for i in 1..v.len() {
        let val = v[i].to_u64();
        if val < HALF_P && (best_val >= HALF_P || val > best_val) {
            best = i;
            best_val = val;
        }
    }
    best
}

/// Trinity pipeline result — all intermediate values for verification.
struct TrinityResult {
    result: Vec<F>,      // decrypted plaintexts
    activated: Vec<F>,   // after dense+ReLU
    class: usize,        // argmax classification
    lut_digest: F,       // LUT sponge hash
    poseidon_digest: F,  // Poseidon2 hash
    pbs_result: F,       // PBS demo output
    quantum: bool,       // quantum commitment
}

fn trinity(
    cts: &[Ciphertext],
    s: &[F],
    priv_w: &[Vec<F>],
    dense_w: &[F],
    dense_b: &[F],
    lut: &[F],
    weights_digest: F,
    key_digest: F,
    d: F,
) -> TrinityResult {
    // Phase 1: Encrypted linear layer
    let ct_out = private_linear(cts, priv_w);
    // Phase 1b: Decrypt
    let result: Vec<F> = ct_out.iter().map(|ct| decrypt(ct, s, d)).collect();
    // Phase 2: Dense neural layer — Reader 1 (lut activation)
    let activated = dense(dense_w, &result, dense_b, lut, NEURONS, NEURONS);
    // Compute class from neural output
    let class = argmax(&activated);
    let class_f = F::from_u64(class as u64);
    // Phase 3a: LUT sponge hash — Reader 2 (S-box from same table)
    let lut_digest = lut_hash_commit(&activated, weights_digest, key_digest, class_f, lut);
    // Phase 3b: Poseidon2 hash — binding commitment
    let poseidon_digest = hash_commit(&activated, weights_digest, key_digest, class_f);
    // Phase 4: PBS demo — Reader 3 (test polynomial from same table)
    let pbs_result = pbs_demo(&ct_out[0], s, d, lut);
    // Phase 5: Quantum commitment
    let quantum = quantum_commit(class);

    TrinityResult {
        result,
        activated,
        class,
        lut_digest,
        poseidon_digest,
        pbs_result,
        quantum,
    }
}

// ===========================================================================
// Divine value computation for bench harness
// ===========================================================================
// The bench harness operates on zero-initialized RAM (except parameters at
// addr 0-28 and the LUT built by Phase 0). Divine values must be consistent
// with this zero data for asserts to pass.
//
// FIFO order matches program execution: Phase 1b divines, then Phase 4 divines.

/// Compute Poseidon2-Goldilocks round constants (same as src/field/poseidon2.rs).
/// t=8, RF=8, RP=22. Full rounds: 8 constants each, partial: 1 constant each.
/// Layout: first_full(32) + partial(22) + last_full(32) = 86 total.
fn poseidon2_round_constants() -> Vec<u64> {
    let width = 8;
    let rounds_f = 8usize;
    let rounds_p = 22usize;
    let total = rounds_f + rounds_p;
    let mut constants = Vec::new();
    for r in 0..total {
        let is_full = r < rounds_f / 2 || r >= rounds_f / 2 + rounds_p;
        if is_full {
            for e in 0..width {
                let tag = format!("Poseidon2-Goldilocks-t8-RF8-RP22-{}-{}", r, e);
                let digest = blake3::hash(tag.as_bytes());
                let bytes: [u8; 8] = digest.as_bytes()[..8].try_into().unwrap();
                constants.push(u64::from_le_bytes(bytes));
            }
        } else {
            let tag = format!("Poseidon2-Goldilocks-t8-RF8-RP22-{}-0", r);
            let digest = blake3::hash(tag.as_bytes());
            let bytes: [u8; 8] = digest.as_bytes()[..8].try_into().unwrap();
            constants.push(u64::from_le_bytes(bytes));
        }
    }
    constants
}

/// Simulate LUT sponge on zero bench data to produce reduce_mod divine pairs.
/// Returns (r, k) pairs in FIFO order: 14 rounds × 8 elements × 2 values = 224.
fn compute_lut_sponge_divine(domain: u64, lut: &[F]) -> Vec<u64> {
    let rc = lut_sponge_round_constants();
    // Initial state for hash4_to_digest(0, 0, 0, 0):
    // input = (weights_digest=0, key_digest=0, output_digest=0, class=0)
    // state = [0, 0, 0, 0, 4, 0, 0, 0] (domain sep = 4)
    let mut state = [F::ZERO; 8];
    state[4] = F::from_u64(4); // domain separation

    let mut divine = Vec::new();

    for r in 0..LUT_SPONGE_ROUNDS {
        // add_constants
        let rc_offset = r * LUT_SPONGE_WIDTH;
        for i in 0..LUT_SPONGE_WIDTH {
            state[i] = state[i].add(rc[rc_offset + i]);
        }
        // sbox_layer: reduce_mod + lut.read for each element
        for i in 0..LUT_SPONGE_WIDTH {
            let x = state[i].to_u64() as u128;
            let d = domain as u128;
            let r_val = (x % d) as u64;
            let k_val = (x / d) as u64;
            divine.push(r_val); // r = x mod D
            divine.push(k_val); // k = x / D
            // Apply S-box: state[i] = lut[r_val]
            state[i] = if (r_val as usize) < lut.len() {
                lut[r_val as usize]
            } else {
                F::ZERO
            };
        }
        // mds
        let sum = state.iter().fold(F::ZERO, |acc, &x| acc.add(x));
        for s in state.iter_mut() {
            *s = s.add(sum);
        }
    }

    divine
}

fn compute_bench_divine(
    lwe_n: usize,
    neurons: usize,
    ring_n: usize,
    domain: u64,
    lut: &[F],
) -> Vec<u64> {
    let mut divine = Vec::new();

    // Phase 1b: decrypt_outputs — one divine per neuron.
    // Zero ciphertexts with zero keys: phase = 0, m = 0 satisfies noise bound.
    for _ in 0..neurons {
        divine.push(0);
    }
    let phase1b_count = divine.len();

    // Phase 3: LUT sponge reduce_mod divine pairs.
    // Runs on zero data (activated = all 0, weights_digest = 0, key_digest = 0, class = 0).
    let sponge_divine = compute_lut_sponge_divine(domain, lut);
    let phase3_count = sponge_divine.len();
    divine.extend(sponge_divine);

    // Phase 4: pbs.bootstrap chain

    // 4a: build_test_poly — divine table indices.
    // Verify: idx * domain - table_idx * ring_n < ring_n
    // Solution: table_idx = floor(idx * domain / ring_n)
    for i in 0..ring_n {
        divine.push((i as u64 * domain) / ring_n as u64);
    }

    // 4b: rotation = io.divine()
    // Zero ciphertext → rotation = 0.
    divine.push(0);

    // 4c: blind_rotate calls monomial_mul twice (a-component, b-component).
    // With rotation k=0: monomial_mul is identity.
    // For each j in 0..ring_n: src = j, sign = 1.
    for _ in 0..2 {
        for j in 0..ring_n {
            divine.push(j as u64); // src
            divine.push(1);        // sign (no negation)
        }
    }

    // 4d: key_switch — (lwe_n + 1) divine values for the switched ciphertext.
    // Zero input → zero output.
    for _ in 0..=lwe_n {
        divine.push(0);
    }

    // 4e: lwe.decrypt inside bootstrap — one divine for plaintext m.
    // Zero ciphertext → m = 0.
    divine.push(0);

    let phase4_count = divine.len() - phase1b_count - phase3_count;
    eprintln!("--- Divine values for bench harness ---");
    eprintln!("  Phase 1b: {} values (decrypt plaintexts)", phase1b_count);
    eprintln!("  Phase 3:  {} values (LUT sponge reduce_mod)", phase3_count);
    eprintln!("  Phase 4:  {} values (PBS chain)", phase4_count);
    eprintln!("  Total:    {} divine values", divine.len());
    let _ = std::io::Write::flush(&mut std::io::stderr());

    divine
}

fn main() {
    let d = delta();
    // ========== DATA SETUP ==========

    // Secret key (small values for demo)
    let s: Vec<F> = (0..LWE_N)
        .map(|i| F::from_u64((i as u64 + 1) % 3))
        .collect();

    // Plaintext messages: [1, 2, ..., INPUT_DIM] mod DOMAIN
    let messages: Vec<F> = (0..INPUT_DIM)
        .map(|i| F::from_u64((i as u64 + 1) % DOMAIN))
        .collect();

    // Encrypt INPUT_DIM values
    let cts: Vec<Ciphertext> = (0..INPUT_DIM)
        .map(|i| {
            let m = messages[i];
            let a: Vec<F> = (0..LWE_N)
                .map(|j| F::from_u64(((i * LWE_N + j) as u64 + 7) % 97))
                .collect();
            let e = F::from_u64(i as u64 + 1); // small noise
            encrypt(m, &s, &a, e, d)
        })
        .collect();

    // Private layer weights (NEURONS x INPUT_DIM)
    let priv_w: Vec<Vec<F>> = (0..NEURONS)
        .map(|i| {
            (0..INPUT_DIM)
                .map(|j| F::from_u64(((i * INPUT_DIM + j) as u64 + 1) % 5))
                .collect()
        })
        .collect();

    // Dense layer (NEURONS x NEURONS = 32x32)
    // Sparse pattern: diagonal + one neighbor keeps sum ≤ 2 × max_input.
    // With DOMAIN=64, inputs are in [0, 63], max output = 2×63+31 = 157 → mod 64 wraps.
    // LUT ReLU accepts all indices in [0, DOMAIN); values wrap naturally in demo.
    let dense_w: Vec<F> = (0..NEURONS * NEURONS)
        .map(|i| {
            let row = i / NEURONS;
            let col = i % NEURONS;
            // Diagonal + one neighbor: keeps sum manageable
            if col == row || col == (row + 1) % NEURONS {
                F::ONE
            } else {
                F::ZERO
            }
        })
        .collect();
    let dense_b: Vec<F> = (0..NEURONS)
        .map(|i| F::from_u64(i as u64))
        .collect();

    // Shared lookup table: ReLU over [0, DOMAIN)
    let lut = build_relu_lut();

    // Precomputed digests
    let weights_hash = poseidon2::hash_fields_goldilocks(&dense_w);
    let weights_digest = weights_hash[0];
    let s_hash = poseidon2::hash_fields_goldilocks(&s);
    let key_digest = s_hash[0];

    // ========== END-TO-END EXECUTION ==========

    let tr = trinity(
        &cts, &s, &priv_w, &dense_w, &dense_b, &lut,
        weights_digest, key_digest, d,
    );

    // ========== VERIFICATION PRINTOUT ==========

    eprintln!("=== TRINITY: Rosetta Stone Unification ===");
    eprintln!("=== One table, four readers, five domains ===");
    eprintln!();
    eprintln!("--- Parameters ---");
    eprintln!("  p (Goldilocks)     = {}", 0xFFFF_FFFF_0000_0001u64);
    eprintln!("  delta (p/DOMAIN)   = {}", d.to_u64());
    eprintln!("  LWE_N              = {}", LWE_N);
    eprintln!("  INPUT_DIM          = {}", INPUT_DIM);
    eprintln!("  NEURONS            = {}", NEURONS);
    eprintln!("  RING_N             = {}", RING_N);
    eprintln!("  DOMAIN             = {}", DOMAIN);
    eprintln!();

    eprintln!("--- Secret key ---");
    eprint!("  s = [");
    for (i, &si) in s.iter().enumerate() {
        if i > 0 { eprint!(", "); }
        eprint!("{}", si.to_u64());
    }
    eprintln!("]");
    eprintln!();

    eprintln!("--- Phase 1: LWE Encryption ---");
    eprint!("  plaintexts = [");
    for (i, &m) in messages.iter().enumerate() {
        if i > 0 { eprint!(", "); }
        eprint!("{}", m.to_u64());
    }
    eprintln!("]");
    eprintln!("  {} ciphertexts, each {} field elements", INPUT_DIM, LWE_N + 1);
    eprintln!();

    eprintln!("--- Phase 1b: Decrypt ---");
    eprint!("  decrypted = [");
    for (i, &r) in tr.result.iter().enumerate() {
        if i > 0 { eprint!(", "); }
        eprint!("{}", r.to_u64());
    }
    eprintln!("]");
    // Verify round-trip: decrypt(encrypt(m)) == m for each original ciphertext
    let roundtrip_ok = messages.iter().zip(cts.iter())
        .all(|(&m, ct)| decrypt(ct, &s, d).to_u64() == m.to_u64());
    eprintln!("  encrypt/decrypt    = {}", if roundtrip_ok { "PASS" } else { "FAIL" });
    // Note: tr.result contains weighted sums from private_linear, not original plaintexts
    eprintln!();

    eprintln!("--- Phase 2: Dense Layer + ReLU (Reader 1: lut.apply) ---");
    eprint!("  activated = [");
    for (i, &a) in tr.activated.iter().enumerate() {
        if i > 0 { eprint!(", "); }
        eprint!("{}", a.to_u64());
    }
    eprintln!("]");
    let output_digest = tr.activated.iter().fold(F::ZERO, |acc, &x| acc.add(x));
    eprintln!("  output_digest      = {}", output_digest.to_u64());
    eprintln!("  class (argmax)     = {}", tr.class);
    eprintln!();

    eprintln!("--- Phase 3a: LUT Sponge Hash (Reader 2: lut.read in S-box) ---");
    eprintln!("  lut_digest         = {}", tr.lut_digest.to_u64());
    eprintln!("  (14 rounds * 8 S-box reads = 112 table reads from shared LUT)");
    eprintln!();

    eprintln!("--- Phase 3b: Poseidon2 Hash (production binding) ---");
    eprintln!("  weights_digest     = {}", weights_digest.to_u64());
    eprintln!("  key_digest         = {}", key_digest.to_u64());
    eprintln!("  poseidon_digest    = {}", tr.poseidon_digest.to_u64());
    eprintln!();

    eprintln!("--- Phase 4: PBS Demo (Reader 3: lut.read in test polynomial) ---");
    // PBS operates on ct_out[0] from Phase 1 (the first encrypted weighted sum)
    eprintln!("  input: ct_out[0]   -> decrypt = {}", tr.result[0].to_u64());
    eprintln!("  pbs_result         = lut[{}] = {}", tr.result[0].to_u64(), tr.pbs_result.to_u64());
    // Verify PBS matches direct lookup on the decrypted value
    let direct_lookup = lut_read(&lut, tr.result[0]);
    eprintln!("  direct lut_read    = {}", direct_lookup.to_u64());
    let pbs_ok = tr.pbs_result.to_u64() == direct_lookup.to_u64();
    eprintln!("  PBS == direct      = {}", if pbs_ok { "PASS" } else { "FAIL" });
    eprintln!();

    eprintln!("--- Phase 5: Quantum Commitment (2-qubit Bell) ---");
    eprintln!("  class              = {}", tr.class);
    eprintln!("  quantum_commit     = {}", tr.quantum);
    eprintln!("  (class=0 -> true, class>0 -> false)");
    eprintln!();

    eprintln!("--- Prover Hints (expected values for assert.eq) ---");
    eprintln!("  expected_class     = {}", tr.class);
    eprintln!("  expected_lut_digest= {}", tr.lut_digest.to_u64());
    eprintln!("  expected_digest    = {}", tr.poseidon_digest.to_u64());
    eprintln!("  pbs_expected_m     = {}", tr.pbs_result.to_u64());
    eprintln!();

    eprintln!("--- Rosetta Stone: One Table, Four Readers ---");
    eprintln!("  Reader 1: lut.apply  in dense_layer         -> ReLU activation");
    eprintln!("  Reader 2: lut.read   in lut_sponge S-box    -> crypto hash");
    eprintln!("  Reader 3: lut.read   in pbs test polynomial -> FHE bootstrap");
    eprintln!("  Reader 4: STARK LogUp                       -> proof auth (upstream)");
    eprintln!("  All readers: same {}-entry table at lut_addr", DOMAIN);
    eprintln!();

    // Final verdict
    let all_ok = roundtrip_ok && pbs_ok;
    eprintln!("=== VERDICT: {} ===", if all_ok { "ALL CHECKS PASS" } else { "FAILURE" });

    // ========== BENCHMARK ==========

    // Warmup
    for _ in 0..100 {
        std::hint::black_box(trinity(
            &cts, &s, &priv_w, &dense_w, &dense_b, &lut,
            weights_digest, key_digest, d,
        ));
    }

    let iters = 10000u128;
    let start = Instant::now();
    for _ in 0..iters {
        std::hint::black_box(trinity(
            std::hint::black_box(&cts),
            std::hint::black_box(&s),
            std::hint::black_box(&priv_w),
            std::hint::black_box(&dense_w),
            std::hint::black_box(&dense_b),
            std::hint::black_box(&lut),
            std::hint::black_box(weights_digest),
            std::hint::black_box(key_digest),
            std::hint::black_box(d),
        ));
    }
    println!("rust_ns: {}", start.elapsed().as_nanos() / iters);

    // ========== BENCH HARNESS DATA ==========
    // Compute round constants and divine values for Probe 2 parameters.
    // These are for the zero-data bench — all ciphertext/weight RAM is uninitialized.
    let bench_lwe_n = LWE_N;
    let bench_neurons = NEURONS;
    let bench_ring_n = RING_N;
    let bench_domain = DOMAIN;
    let bench_lut = build_relu_lut();

    // Poseidon2 round constants (86 values, BLAKE3-derived)
    let p2_rc = poseidon2_round_constants();
    eprintln!();
    eprintln!("--- Poseidon2 round constants ({} values) ---", p2_rc.len());

    // LUT sponge round constants (112 values, deterministic formula)
    let sponge_rc: Vec<u64> = lut_sponge_round_constants()
        .iter()
        .map(|f| f.to_u64())
        .collect();
    eprintln!("--- LUT sponge round constants ({} values) ---", sponge_rc.len());

    // Divine values (FIFO order: Phase 1b → Phase 3 → Phase 4)
    let divine = compute_bench_divine(
        bench_lwe_n, bench_neurons, bench_ring_n, bench_domain, &bench_lut,
    );

    // Emit round constants as additional public inputs
    let rc_strs: Vec<String> = p2_rc.iter().chain(sponge_rc.iter())
        .map(|v| v.to_string())
        .collect();
    eprintln!();
    eprintln!("rc_values: {}", rc_strs.join(", "));

    let divine_strs: Vec<String> = divine.iter().map(|v| v.to_string()).collect();
    eprintln!("divine: {}", divine_strs.join(", "));
}
