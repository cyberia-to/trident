#[path = "native_control/support.rs"]
mod support;

#[test]
fn raw_typed_assertion_failure_never_returns_a_value() {
    for return_value in ["assert(false)", "return assert(false)"] {
        let source = support::program(return_value);
        support::worker(move || {
            let artifact = support::compile(&source);
            assert!(support::run(&artifact.bytes, 0, 1_000_000, 16384, 196608).is_err());
        });
    }
}

#[test]
fn raw_conditional_joins_keep_only_continuing_values() {
    for body in [
        "if n==0 {assert(false)} else {7}",
        "if n==0 {return assert(false)} else {return 7}",
        "if n==1 {7} else {assert(false)}",
        "if n==0 {if true {assert(false)}} else {7}",
        "if n==0 {if 0 {assert(false)}} else {7}",
        "if n==0 {if 18446744069414584322 {} else {assert(false)}} else {7}",
    ] {
        let source=format!("program halt\nfn choose(n:Field)->Field {{ {body} }}\nfn main(input:Noun)->Noun {{ nox_noun_atom(choose(nox_noun_as_field(input))) }}");
        support::worker(move || {
            let artifact = support::compile(&source);
            assert_eq!(
                support::run(&artifact.bytes, 1, 1_000_000, 16384, 196608)
                    .unwrap()
                    .0,
                7
            );
            assert!(support::run(&artifact.bytes, 0, 1_000_000, 16384, 196608).is_err());
        });
    }
}

#[test]
fn raw_imports_follow_direct_intrinsic_ownership() {
    support::worker(|| {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("entry.tri");
        std::fs::write(
            directory.path().join("assert.tri"),
            "module assert\npub fn is_true(c:Bool)->Field {7}\n",
        )
        .unwrap();
        std::fs::write(&path,"program ordinary\nuse assert\nfn main(input:Noun)->Noun { assert.is_true(false) nox_noun_atom(assert.is_true(false)) }\n").unwrap();
        let artifact = trident::compile_raw_artifact_project(
            &path,
            &trident::CompileOptions::default(),
            trident::RAW_ARTIFACT_LIMITS,
        )
        .unwrap();
        assert_eq!(
            support::run(&artifact.bytes, 0, 1_000_000, 16384, 196608)
                .unwrap()
                .0,
            7
        );
        for call in ["assert.is_true(false)", "vm.core.assert.is_true(false)"] {
            std::fs::write(&path,format!("program intrinsic\nuse vm.core.assert\nfn main(input:Noun)->Noun {{ {call} }}\n")).unwrap();
            let artifact = trident::compile_raw_artifact_project(
                &path,
                &trident::CompileOptions::default(),
                trident::RAW_ARTIFACT_LIMITS,
            )
            .unwrap();
            assert!(support::run(&artifact.bytes, 0, 1_000_000, 16384, 196608).is_err());
        }
    });
}

#[test]
fn raw_callable_aliases_preserve_distinct_owners_and_earlier_scopes() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        let modules = [
            (
                "early.tri",
                "module std.same\npub const FLAG:Field=0\npub fn a()->Field {7}\npub fn stop(c:Bool)->Field {11}",
            ),
            (
                "helper.tri",
                "module helper\nuse early\npub fn value()->Field {same.stop(false)}",
            ),
            (
                "late.tri",
                "module os.same\n#[intrinsic(assert)] pub fn stop(c:Bool)\nconst FLAG:Field=1\npub fn b()->Field {13}",
            ),
        ];
        for (name, source) in modules {
            std::fs::write(dir.path().join(name), source).unwrap();
        }
        let path = dir.path().join("entry.tri");
        std::fs::write(&path,"program aliases\nuse helper\nuse late\nfn main(input:Noun)->Noun {if same.FLAG {nox_noun_atom(same.a()+helper.value()+same.b())} else {assert(false)}}").unwrap();
        let artifact = trident::compile_raw_artifact_project(
            &path,
            &trident::CompileOptions::default(),
            trident::RAW_ARTIFACT_LIMITS,
        )
        .unwrap();
        assert_eq!(
            support::run(&artifact.bytes, 0, 1_000_000, 16384, 196608)
                .unwrap()
                .0,
            31
        );
    });
}

#[test]
fn raw_local_struct_roots_shadow_constant_values_and_types() {
    support::worker(|| {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("constants.tri"),
            "module constants\npub const FLAG:Field=0",
        )
        .unwrap();
        let path = dir.path().join("entry.tri");
        for (field, init, body) in [
            ("Field", "n", "if constants.FLAG {7} else {11}"),
            ("Bool", "n==0", "assert(constants.FLAG) 7"),
        ] {
            let source=format!("program shadow\nuse constants\nstruct Dynamic {{FLAG:{field}}}\nfn choose(n:Field)->Field {{let constants=Dynamic{{FLAG:{init}}} {body}}}\nfn main(input:Noun)->Noun {{nox_noun_atom(choose(nox_noun_as_field(input)))}}");
            std::fs::write(&path, source).unwrap();
            let artifact = trident::compile_raw_artifact_project(
                &path,
                &trident::CompileOptions::default(),
                trident::RAW_ARTIFACT_LIMITS,
            )
            .unwrap();
            assert_eq!(
                support::run(&artifact.bytes, 0, 1_000_000, 16384, 196608)
                    .unwrap()
                    .0,
                7
            );
            let one = support::run(&artifact.bytes, 1, 1_000_000, 16384, 196608);
            if field == "Field" {
                assert_eq!(one.unwrap().0, 11);
            } else {
                assert!(one.is_err());
            }
        }
    });
}
