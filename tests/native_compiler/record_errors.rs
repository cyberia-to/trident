use super::{nouns::data, support};

#[test]
fn nominal_source_types_names_and_initializer_errors_reject_before_emission() {
    support::worker(|| {
        assert!(trident::compile_raw_artifact(
            "program sample struct S{x:Field} fn main(input:Noun)->Noun{nox_noun_atom(S{x:7}.x)}",
            "oracle.tri",
            &trident::CompileOptions::default(),
            trident::NATIVE_ARTIFACT_LIMITS
        )
        .is_ok());
        for (body,code) in [
            ("struct S{x:Field} fn main()->Field{S{x:true}.x}",5),
            ("struct S{x:Field} fn main()->Field{S{}.x}",5),
            ("struct S{x:Field} fn main()->Field{S{x:7,y:9}.x}",5),
            ("struct S{x:Field} fn main()->Field{S{x:7,x:9}.x}",5),
            ("struct S{x:Field} fn main()->Field{let x=7 S{x,x}.x}",5),
            ("struct S{x:Field} fn main()->Field{S{x:}.x}",2),
            ("struct S{x:Field} fn main()->Field{S{x:,}.x}",2),
            ("struct S{x:Field} fn main()->Field{S{x:7).x}",2),
            ("struct S{x:Field} fn main()->Field{S{x:7].x}",2),
            ("struct S{x:Field} fn main()->Field{S{x:(7}.x}",2),
            ("struct S{x:Field} fn main()->Field{S{x:7}.y}",5),
            ("struct S{x:Field} fn main()->Field{let s=S{x:7} s.x=true 7}",5),
            ("struct S{x:Field} struct T{x:Field} fn main()->Field{let t:T=S{x:7} 7}",5),
            ("struct S{x:Field} struct T{x:Field} fn f(s:S)->Field{s.x} fn main()->Field{f(T{x:7})}",5),
            ("struct S{x:Noun} fn main(input:Noun)->Noun{let s=S{x:input} s==s input}",5),
            ("struct S{x:Field} fn main()->Field{let(s)=(S{x:7},7) 7}",5),
            ("struct S{x:Field} fn main()->Field{let(a)=S{x:7} 7}",5),
            ("struct S{x:Field} fn main()->Field{let s=S{x:7} if s{7}else{9}}",5),
            ("fn f(s:Later)->Later{s} struct Later{x:Field} fn main()->Field{7}",5),
            ("struct S{x:S} fn main()->Field{7}",5),
            ("struct S{x:Later} struct Later{x:Field} fn main()->Field{7}",5),
            ("fn main()->Field{let s:Missing=7 7}",5),
            ("struct S{x:external.T} fn main()->Field{7}",6),
        ] {
            let source=format!("program sample {body}");
            match support::compile_only(source.as_bytes(),data::caps()) {
                support::Compilation::Errors(errors)=>assert_eq!(errors[0].code,code,"{source}"),
                other=>panic!("{source}: {other:?}"),
            }
            assert!(trident::compile_raw_artifact(&source,"oracle.tri",&trident::CompileOptions::default(),trident::NATIVE_ARTIFACT_LIMITS).is_err(),"seed {source}");
        }
    });
}

#[test]
fn nominal_declaration_duplicates_and_wide_layouts_report_exact_name_spans() {
    support::worker(|| {
        for (body,span,code) in [
            ("struct S{} struct S{} fn main()->Field{7}".to_string(),"S",5),
            ("struct S{x:Field,x:Field} fn main()->Field{7}".to_string(),"x",5),
            (format!("struct S{{{}}} fn main()->Field{{7}}",(0..33).map(|i|format!("f{i}:Field")).collect::<Vec<_>>().join(",")),"f32",7),
            ("struct A{n:Field} fn id(x:A)->A{let z:Later=x x} struct Later{n:Field} fn main()->Field{7}".to_string(),"x",5),
        ] {
            let source=format!("program sample {body}");
            match support::compile_only(source.as_bytes(),data::caps()) {
                support::Compilation::Errors(errors)=>{
                    assert_eq!(errors[0].code,code,"{source}");
                    assert_eq!(&source[errors[0].start as usize..errors[0].end as usize],span,"{source}");
                    if source.contains("let z:Later=x") {assert_eq!(errors[0].start as usize,source.find("=x").unwrap()+1);}
                    if source.contains("struct S{} struct S{}") {assert_eq!(errors[0].start as usize,source.rfind("S{}").unwrap());}
                    if source.contains("x:Field,x:Field") {assert_eq!(errors[0].start as usize,source.rfind("x:Field").unwrap());}
                }
                other=>panic!("{source}: {other:?}"),
            }
        }
    });
}
