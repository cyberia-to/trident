use super::{signatures::run_probe, support};

fn check(source: &str) -> u64 {
    static PROBE: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    let code = PROBE.get_or_init(|| {
        trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_function_check.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes
    });
    let (arena, result) = run_probe(source, 4096, code);
    arena.atom_value(result).unwrap().as_u64()
}

#[test]
fn every_declared_body_checks_against_its_own_signature_with_final_call_bindings() {
    support::worker(|| {
        for source in [
            "program sample fn main()->Field{later(2)} fn later(x:Field)->Field{x+1}",
            "program sample fn f()->Field{7} fn main()->Field{let f=9 f()+f}",
            "program sample fn f(x:Field,x:Bool)->Bool{x} fn main()->Field{if f(1,true){7}else{9}}",
            "program sample fn f()->Field{7} fn f()->Bool{true} fn main()->Field{if f(){7}else{9}}",
            "program sample fn unit(){} fn main()->Field{unit() 7}",
            "program sample fn unit(){return} fn main()->Field{unit() 7}",
            "program sample fn unit(){} fn relay(){return unit()} fn main()->Field{relay() 7}",
            "program sample fn unit(){} fn relay(){unit()} fn main()->Field{relay() 7}",
            "program sample fn unit(){} fn main()->Field{let mut x=unit() x=unit() if x==unit(){7}else{9}}",
        ] { assert_eq!(check(source),0,"{source}"); }
    });
}

#[test]
fn unit_return_scope_and_unused_body_errors_reject_before_code_generation() {
    support::worker(|| {
        for source in [
            "program sample fn bad()->Field{false} fn main()->Field{7}",
            "program sample fn f()->Field{f()} fn f()->Bool{true} fn main()->Field{7}",
            "program sample fn f(x:Field)->Field{x=9 x} fn main()->Field{7}",
            "program sample fn unit(){7} fn main()->Field{7}",
            "program sample fn unit(){return 7} fn main()->Field{7}",
            "program sample fn unit(){} fn main()->Field{unit()}",
            "program sample fn unit(){} fn main()->Field{let mut x=unit() x=7 9}",
            "program sample fn unit(){} fn main()->Field{if unit(){7}else{9}}",
            "program sample fn f(x:Bool)->Field{if x{7}} fn main()->Field{7}",
            "program sample fn f()->Field{return} fn main()->Field{7}",
            "program sample fn f()->Field{missing()} fn main()->Field{7}",
        ] {
            assert_eq!(check(source), 5, "{source}");
        }
    });
}
