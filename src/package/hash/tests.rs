// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::*;

fn parse_file(source: &str) -> File {
    crate::parse_source_silent(source, "test.tri").unwrap()
}

#[test]
fn test_same_code_same_hash() {
    let f1 =
        parse_file("program test\nfn add(a: Field, b: Field) -> Field { a + b }\nfn main() { }\n");
    let f2 =
        parse_file("program test\nfn add(x: Field, y: Field) -> Field { x + y }\nfn main() { }\n");

    let h1 = hash_file(&f1);
    let h2 = hash_file(&f2);

    // Same computation, different variable names → same hash
    assert_eq!(
        h1["add"], h2["add"],
        "renamed variables should produce same hash"
    );
}

#[test]
fn test_different_code_different_hash() {
    let f1 =
        parse_file("program test\nfn f(a: Field, b: Field) -> Field { a + b }\nfn main() { }\n");
    let f2 =
        parse_file("program test\nfn f(a: Field, b: Field) -> Field { a * b }\nfn main() { }\n");

    let h1 = hash_file(&f1);
    let h2 = hash_file(&f2);

    assert_ne!(
        h1["f"], h2["f"],
        "different operations should produce different hash"
    );
}

#[test]
fn test_hash_display() {
    let hash = ContentHash([0xAB; 32]);
    assert_eq!(hash.to_hex().len(), 64);
    assert_eq!(hash.to_short().len(), 8);
}

#[test]
fn test_hash_deterministic() {
    let f = parse_file(
        "program test\nfn main() {\n    let x: Field = pub_read()\n    pub_write(x)\n}\n",
    );
    let h1 = hash_file(&f);
    let h2 = hash_file(&f);
    assert_eq!(h1["main"], h2["main"]);
}

#[test]
fn test_file_content_hash() {
    let f = parse_file(
        "program test\nfn main() {\n    let x: Field = pub_read()\n    pub_write(x)\n}\n",
    );
    let h1 = hash_file_content(&f);
    let h2 = hash_file_content(&f);
    assert_eq!(h1, h2);
}

#[test]
fn test_hash_with_if() {
    let f = parse_file("program test\nfn main() {\n    let x: Field = pub_read()\n    if x == 0 {\n        pub_write(0)\n    } else {\n        pub_write(1)\n    }\n}\n");
    let h = hash_file(&f);
    assert_ne!(h["main"], ContentHash::zero());
}

#[test]
fn test_hash_with_for() {
    let f = parse_file("program test\nfn main() {\n    let mut s: Field = 0\n    for i in 0..10 {\n        s = s + 1\n    }\n    pub_write(s)\n}\n");
    let h = hash_file(&f);
    assert_ne!(h["main"], ContentHash::zero());
}

#[test]
fn test_spec_does_not_affect_hash() {
    let f1 =
        parse_file("program test\nfn add(a: Field, b: Field) -> Field { a + b }\nfn main() { }\n");
    let f2 = parse_file("program test\n#[requires(a + b < 1000)]\n#[ensures(result == a + b)]\nfn add(a: Field, b: Field) -> Field { a + b }\nfn main() { }\n");

    let h1 = hash_file(&f1);
    let h2 = hash_file(&f2);

    // Spec annotations don't affect computational hash
    assert_eq!(
        h1["add"], h2["add"],
        "spec annotations should not affect hash"
    );
}

#[test]
fn test_empty_fn_hash() {
    let f = parse_file("program test\nfn main() { }\n");
    let h = hash_file(&f);
    assert_ne!(h["main"], ContentHash::zero());
}
