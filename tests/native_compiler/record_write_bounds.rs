use super::{codegen, nouns::data, support};

fn wide(bits: usize) -> String {
    let outer = bits.min(64) - 32;
    let fields = (0..31)
        .map(|i| format!("f{i}:Field"))
        .collect::<Vec<_>>()
        .join(",");
    let values = (0..31)
        .map(|i| format!("f{i}:{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let prefix_fields = (0..outer - 1)
        .map(|i| format!("p{i}:Field"))
        .collect::<Vec<_>>()
        .join(",");
    let prefix_values = (0..outer - 1)
        .map(|i| format!("p{i}:{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let inner = format!("Inner{{{values},last:31}}");
    let outer_value = format!("Outer{{{prefix_values},last:{inner}}}");
    let (extra, value, path) = if bits == 65 {
        (
            "struct Wrap{value:Outer}",
            format!("Wrap{{value:{outer_value}}}"),
            "w.value.last.last",
        )
    } else {
        ("", outer_value, "w.last.last")
    };
    format!("program sample struct Inner{{{fields},last:Field}} struct Outer{{{prefix_fields},last:Inner}} {extra} fn main()->Field{{let mut w={value} let old=w {path}=99 {}*100+{path}}}",path.replacen("w.","old.",1))
}

#[test]
fn wide_record_sources_execute_when_the_unchanged_compiler_arena_fits_them() {
    support::worker(|| {
        let mut accepted = Vec::new();
        for bits in [61, 62, 63, 64, 65] {
            let source = wide(bits);
            assert!(source.len() < 4096);
            // Keep every original wide source and the 786432 lifetime ceiling.
            let result = support::try_compile_only_package(
                &[support::module(source.as_bytes())],
                "sample",
                "main",
                support::options(),
                data::caps(),
            );
            match result {
                Ok(support::Compilation::Program {
                    bytes,
                    reductions,
                    nodes,
                    frames,
                }) => {
                    assert_eq!(support::run_artifact(&bytes), 3199, "guest bits={bits}");
                    println!("wide{bits}: reductions={reductions}, nodes={nodes}, frames={frames}");
                    accepted.push(bits);
                }
                Err(error) => {
                    assert!(
                        error.contains("Error(Unavailable)") && error.contains("nodes=786432"),
                        "bits={bits}: {error}"
                    );
                    println!("wide{bits}: {error}");
                }
                Ok(support::Compilation::Errors(errors)) => panic!("bits={bits}: {errors:?}"),
            }
            assert_eq!(support::rust_value(&source), 3199, "seed bits={bits}");
        }
        assert_eq!(accepted, [61, 62, 63, 64]);
    });
}

#[test]
fn static_record_edit_node_and_emitted_depth_allowances_are_exact() {
    support::worker(|| {
        let source =
            "program sample struct S{x:Field} fn main()->Field{let mut s=S{x:7} s.x=9 s.x}";
        let mut previous = None;
        for cap in [5, 6, 7, 8, 9] {
            let mut caps = data::caps();
            caps[2] = 1;
            caps[3] = cap;
            match support::compile_only(source.as_bytes(), caps) {
                support::Compilation::Errors(errors) if cap < 8 => {
                    assert_eq!(errors[0].code, 7, "cap={cap}");
                    if cap == 5 {
                        assert_eq!(
                            &source[errors[0].start as usize..errors[0].end as usize],
                            "9"
                        );
                    }
                }
                support::Compilation::Program { bytes, .. } if cap >= 8 => {
                    assert_eq!(support::run_artifact(&bytes), 9);
                    if let Some(ref previous) = previous {
                        assert_eq!(&bytes, previous);
                    }
                    previous = Some(bytes);
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
        let probe = trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_digest_depth.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap()
        .bytes;
        for source in [source.to_string(),"program sample struct S{x:Field,y:Field} struct Box{x:S,y:S} fn main()->Field{let mut b=Box{x:S{x:1,y:2},y:S{x:3,y:4}} b.y.x=9 b.x.y+b.y.x+b.y.y}".to_string()] {
            let generate=|cap|codegen::generate_with_input::<{1<<20}>(&probe,|arena|{
                let source=support::data::Bytes::from_slice(arena,source.as_bytes(),4096).unwrap().encode(arena).unwrap();
                let cap=support::data::atom(arena,cap).unwrap();
                support::data::pair(arena,cap,source).unwrap()
            });
            let bytes=generate(4096).unwrap();let exact=codegen::artifact_depth(&bytes)+4;
            assert_eq!(generate(exact-1),Err(7),"{source}");
            assert_eq!(generate(exact).unwrap(),bytes);
            assert_eq!(generate(exact+1).unwrap(),bytes);
            assert_eq!(support::run_artifact(&bytes),support::rust_value(&source));
        }
    });
}
