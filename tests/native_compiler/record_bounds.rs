use super::{codegen, nouns::data, support};

#[test]
fn nominal_registry_allowance_checks_before_the_first_excess_declaration() {
    support::worker(|| {
        let source = "program sample struct A{} struct B{} struct C{} fn main()->Field{7}";
        let mut previous = None;
        for cap in [2, 3, 4] {
            let mut caps = data::caps();
            caps[2] = 1;
            caps[3] = cap;
            match support::compile_only(source.as_bytes(), caps) {
                support::Compilation::Errors(errors) if cap == 2 => {
                    assert_eq!(errors[0].code, 7);
                    assert_eq!(
                        &source[errors[0].start as usize..errors[0].end as usize],
                        "C"
                    );
                }
                support::Compilation::Program { bytes, .. } if cap >= 3 => {
                    assert_eq!(support::run_artifact(&bytes), 7);
                    if let Some(ref previous) = previous {
                        assert_eq!(&bytes, previous);
                    }
                    previous = Some(bytes);
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
    });
}

#[test]
fn nominal_empty_and_32_field_emission_obey_independent_exact_formula_depth() {
    support::worker(|| {
        let probe = trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_digest_depth.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes;
        let fields = (0..32)
            .map(|i| format!("f{i}:Field"))
            .collect::<Vec<_>>()
            .join(",");
        let values = (0..32)
            .map(|i| format!("f{i}:{i}"))
            .collect::<Vec<_>>()
            .join(",");
        for source in [
            "program sample struct Empty{} fn main()->Field{let s=Empty{} 7}".to_string(),
            format!("program sample struct Wide{{{fields}}} fn main()->Field{{Wide{{{values}}}.f31}}"),
            "program sample struct S{x:Field} fn make(x:Field)->S{S{x}} fn main()->Field{make(sub(9,2)).x}".to_string(),
        ] {
            let generate=|cap|codegen::generate_with_input::<{1<<20}>(&probe,|arena|{
                let source=support::data::Bytes::from_slice(arena,source.as_bytes(),4096).unwrap().encode(arena).unwrap();
                let cap=support::data::atom(arena,cap).unwrap();
                support::data::pair(arena,cap,source).unwrap()
            });
            let bytes=generate(4096).unwrap();let exact=codegen::artifact_depth(&bytes)+4;
            assert_eq!(generate(exact-1),Err(7),"{source}");
            assert_eq!(generate(exact).unwrap(),bytes,"{source}");
            assert_eq!(generate(exact+1).unwrap(),bytes,"{source}");
            assert_eq!(support::run_artifact(&bytes),support::rust_value(&source));
        }
    });
}

#[test]
fn nominal_constructor_context_belongs_to_its_nested_digest_index_delimiter() {
    support::worker(|| {
        let source="program sample struct S{x:Field} fn main(input:Noun)->Noun{let d=nox_noun_identity(input) if d[S{x:2}.x]==d[2]{input}else{nox_noun_atom(0)}}";
        let input = data::nested();
        data::agrees(source, &input, &input);
    });
}
