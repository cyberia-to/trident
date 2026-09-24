//! A named constructor must provide each declared field exactly once.
#[path = "native_control/support.rs"]
mod support;
use trident::{CompileOptions, NATIVE_ARTIFACT_LIMITS};

fn source(init: &str) -> String {
    format!("program sample struct Point{{x:Field,y:Field}} fn main(input:Noun)->Noun{{let x=7 let p={init} input}}")
}

#[test]
fn duplicate_initializers_reject_even_when_the_first_value_is_well_typed() {
    let valid = source("Point{x:7,y:9}");
    assert!(trident::compile_raw_artifact(
        &valid,
        "sample.tri",
        &CompileOptions::default(),
        NATIVE_ARTIFACT_LIMITS
    )
    .is_ok());
    for init in [
        "Point{x:7,y:9,x:11}",
        "Point{x:7,x:true,y:9}",
        "Point{x,y:9,x}",
    ] {
        let text = source(init);
        let errors = trident::compile_raw_artifact(
            &text,
            "sample.tri",
            &CompileOptions::default(),
            NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap_err();
        let duplicates: Vec<_> = errors
            .iter()
            .filter(|e| e.message == "duplicate field 'x' in struct init")
            .collect();
        assert_eq!(duplicates.len(), 1, "{init}: {errors:?}");
        let span = duplicates[0].span;
        assert_eq!(&text[span.start as usize..span.end as usize], "x");
        assert_eq!(span.start as usize, text.rfind('x').unwrap());
    }
}

#[test]
fn nested_constructors_apply_the_same_field_uniqueness_rule() {
    let text="program sample struct Point{x:Field,y:Field} struct Outer{item:Point} fn main(input:Noun)->Noun{let p=Outer{item:Point{x:7,y:9,x:11}} input}";
    let errors = trident::compile_raw_artifact(
        text,
        "sample.tri",
        &CompileOptions::default(),
        NATIVE_ARTIFACT_LIMITS,
    )
    .unwrap_err();
    assert!(errors
        .iter()
        .any(|e| e.message == "duplicate field 'x' in struct init"));
}

#[test]
fn unique_reordered_and_shorthand_initializers_preserve_native_artifacts() {
    support::worker(|| {
        let mut canonical = None;
        for init in ["Point{x,y:9}", "Point{y:9,x}", "Point{x:x,y:9}"] {
            let text=format!("program sample struct Point{{x:Field,y:Field}} fn main(input:Noun)->Noun{{let x=7 let p={init} nox_noun_atom(p.x*10+p.y)}}");
            let bytes = trident::compile_raw_artifact(
                &text,
                "sample.tri",
                &CompileOptions::default(),
                NATIVE_ARTIFACT_LIMITS,
            )
            .unwrap()
            .bytes;
            assert_eq!(
                support::run(&bytes, 0, 1_000_000, 65536, 196608).unwrap().0,
                79
            );
            if let Some(previous) = canonical {
                assert_eq!(bytes, previous);
            }
            canonical = Some(bytes);
        }
    });
}
