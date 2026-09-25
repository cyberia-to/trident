#[path = "native_control/support.rs"]
mod support;
#[path = "support/mod.rs"]
mod target_support;
use trident::CompileOptions;

fn run(bytes: &[u8]) -> u64 {
    support::run(bytes, 0, 1_000_000, 16384, 196608).unwrap().0
}

#[test]
fn native_constant_aliases_preserve_types_and_runtime_field_reduction() {
    support::worker(|| {
        for (definitions, body, expected) in [
            ("const A:Field=B\nconst B:Field=7", "A", 7),
            ("const A:U32=B\nconst B:U32=7", "as_field(A)", 7),
            (
                "const A:Field=B\nconst B:Field=18446744069414584322",
                "A",
                1,
            ),
            (
                "const N:U32=1\nfn f(a:[Field;N])->Field{a[1]}\nconst N:U32=2",
                "f([3,7])",
                7,
            ),
            (
                "const N:U32=BASE\nconst BASE:U32=2\nfn f<K>(a:[Field;K+N])->Field{a[2]}",
                "f<1>([3,5,7])",
                7,
            ),
            ("const X:Field=1\nconst X:Field=7", "X", 7),
        ] {
            let source=format!("program constants\n{definitions}\nfn main(input:Noun)->Noun{{nox_noun_atom({body})}}");
            let artifact = support::compile(&source);
            assert_eq!(run(&artifact.bytes), expected, "{source}");
        }
        let source="program constants\nconst N:Field=18446744069414584322\nconst SIZE:Field=N\nfn f(a:[Field;SIZE])->Field{a[0]}\nfn main(input:Noun)->Noun{nox_noun_atom(f([7]))}";
        assert!(trident::compile_raw_artifact(
            source,
            "constant.tri",
            &CompileOptions::default(),
            trident::RAW_ARTIFACT_LIMITS
        )
        .is_err());
    });
}

#[test]
fn imported_constant_aliases_keep_owners_and_private_generic_dimensions() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        for (name,source) in [
            ("a/same.tri","module a.same\npub const VALUE:Field=7\npub const SIZE:U32=2"),
            ("helper.tri","module helper\nuse a.same\npub const VALUE:Field=same.VALUE\nconst K:U32=same.SIZE\npub fn f<N>(a:[Field;N+K])->Field{a[2]+VALUE}"),
            ("b/same.tri","module b.same\npub const VALUE:Field=13\nconst SIZE:U32=99"),
        ] {let path=dir.path().join(name); std::fs::create_dir_all(path.parent().unwrap()).unwrap(); std::fs::write(path,source).unwrap();}
        let path = dir.path().join("entry.tri");
        std::fs::write(&path,"program constants\nuse helper\nuse b.same\nconst K:U32=99\nconst V:Field=helper.VALUE\nfn main(input:Noun)->Noun{nox_noun_atom(helper.f<1>([3,5,11])+V+same.VALUE)}").unwrap();
        let artifact = trident::compile_raw_artifact_project(
            &path,
            &CompileOptions::default(),
            trident::RAW_ARTIFACT_LIMITS,
        )
        .unwrap();
        assert_eq!(run(&artifact.bytes), 38);
        std::fs::write(&path,"program constants\nuse helper\nuse b.same\nconst V:Field=helper.VALUE\nfn main()->Field{helper.f<1>([3,5,11])+V+same.VALUE}").unwrap();
        let options = CompileOptions::default()
            .with_package(target_support::triton_package())
            .unwrap();
        assert!(trident::build_tir_modules(&path, &options).is_ok());
    });
}

#[test]
fn final_private_constants_cannot_export_an_earlier_public_type() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("entry.tri");
    std::fs::write(
        dir.path().join("values.tri"),
        "module values\npub const X:U32=7\nconst X:Field=4294967296",
    )
    .unwrap();
    for initializer in ["values.X", "as_field(values.X)"] {
        std::fs::write(
            &path,
            format!("program constants\nuse values\nfn main()->Field{{{initializer}}}"),
        )
        .unwrap();
        assert!(trident::compile_to_bundle(&path, &CompileOptions::default()).is_err());
        let options = CompileOptions::default()
            .with_package(target_support::triton_package())
            .unwrap();
        assert!(trident::build_tir_modules(&path, &options).is_err());
    }
}

#[test]
fn stack_constant_aliases_emit_the_same_ir_as_literals() {
    let options = CompileOptions::default()
        .with_package(target_support::triton_package())
        .unwrap();
    for (ty, value, body) in [
        ("Field", "7", "X"),
        ("U32", "7", "as_field(X)"),
        ("Field", "18446744069414584322", "X"),
    ] {
        let alias = format!(
            "program constants\nconst X:{ty}=Y\nconst Y:{ty}={value}\nfn main()->Field{{{body}}}"
        );
        let literal = alias.replace(&format!("X:{ty}=Y"), &format!("X:{ty}={value}"));
        assert_eq!(
            format!(
                "{:?}",
                trident::build_tir(&alias, "constant.tri", &options).unwrap()
            ),
            format!(
                "{:?}",
                trident::build_tir(&literal, "constant.tri", &options).unwrap()
            )
        );
    }
}
