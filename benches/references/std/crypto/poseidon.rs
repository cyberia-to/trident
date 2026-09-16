//! Standard Poseidon2-HL reference benchmark, same protocol as std.crypto.poseidon v2.
use std::time::Instant;
#[path = "../../common/poseidon_standard.rs"]
mod standard;
fn main() {
    for _ in 0..100 {
        std::hint::black_box(standard::hash2(42, 1337));
    }
    let n = 10000;
    let start = Instant::now();
    for _ in 0..n {
        std::hint::black_box(standard::hash2(
            std::hint::black_box(42),
            std::hint::black_box(1337),
        ));
    }
    println!("rust_ns: {}", start.elapsed().as_nanos() / n);
}
