//! Native table identity follows defining functions and concrete size arguments.
#[path = "native_control/support.rs"]
mod support;
use support::*;
use trident::{CompileOptions, RAW_ARTIFACT_LIMITS as LIMITS};

#[test]
fn irrelevant_generic_instances_cannot_renumber_reachable_native_code() {
    worker(|| {
        let base = "program p
fn sum<N>(a: [Field; N]) -> Field { let mut total: Field = 0\nfor i in 0..N { total = total + a[i] }\ntotal }
fn main(input: Noun) -> Noun { nox_noun_atom(sum<1>([3]) + sum<2>([4,5])) }";
        let extra = "\nfn aa_unused() -> Field { sum<3>([7,8,9]) }";
        let first = compile(base);
        let second = compile(&format!("{base}{extra}"));
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(
            run(&first.bytes, 0, 1_000_000, 65536, LIMITS.max_nodes)
                .unwrap()
                .0,
            12
        );
    });
}

#[test]
fn imported_generic_calls_keep_defining_constants_and_struct_layouts() {
    worker(|| {
        let dir = tempfile::tempdir().unwrap();
        let entry = dir.path().join("main.tri");
        std::fs::write(dir.path().join("left.tri"),"module left\nconst K: Field = 11\npub struct Result { pub value: Field }\npub fn step<N>(a: [Field; N]) -> Result { let mut total: Field = K\nfor i in 0..N { total = total + a[i] }\nResult { value: total } }").unwrap();
        std::fs::write(dir.path().join("right.tri"),"module right\nconst K: Field = 100\npub struct Result { pub ignored: Field, pub value: Field }\npub fn step<N>(a: [Field; N]) -> Result { let mut total: Field = K\nfor i in 0..N { total = total + a[i] }\nResult { ignored: 999, value: total } }").unwrap();
        std::fs::write(&entry,"program p\nuse left\nuse right\nconst K: Field = 1000\nfn main(input: Noun) -> Noun { let a = left.step<2>([1,2])\nlet b = right.step<1>([3])\nnox_noun_atom(a.value * 1000 + b.value + K) }").unwrap();
        let artifact =
            trident::compile_raw_artifact_project(&entry, &CompileOptions::default(), LIMITS)
                .unwrap();
        assert_eq!(
            run(&artifact.bytes, 0, 1_000_000, 65536, LIMITS.max_nodes)
                .unwrap()
                .0,
            15103
        );
    });
}
