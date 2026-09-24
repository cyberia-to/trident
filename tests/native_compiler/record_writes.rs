use super::{nouns::data, support};
use data::{Noun, Noun::Atom};

fn scalar(body: &str, expected: u64) {
    let source = format!("program sample {body}");
    assert_eq!(
        support::run_artifact(&data::compile(&source)),
        expected,
        "{source}"
    );
    assert_eq!(support::rust_value(&source), expected, "seed {source}");
}

#[test]
fn static_record_writes_preserve_snapshots_siblings_and_lexical_slots() {
    support::worker(|| {
        for (body,expected) in [
            ("struct S{x:Field,y:Field,z:Field} fn main()->Field{let mut s=S{x:1,y:2,z:3} s.x=7 s.y=8 s.z=9 s.x*100+s.y*10+s.z}",789),
            ("struct S{x:Field,y:Field} struct Box{left:S,right:S} fn main()->Field{let mut b=Box{left:S{x:1,y:2},right:S{x:3,y:4}} let old=b b.left.y=7 b.left.x*1000+b.left.y*100+b.right.x*10+old.left.y}",1732),
            ("struct S{x:Field} fn main()->Field{let mut s=S{x:7} if true{let mut s=S{x:3} s.x=9} s.x}",7),
            ("struct S{x:Field} fn main()->Field{let s=S{x:7} let mut s=s s.x=9 s.x}",9),
            ("struct S{x:Field} fn main()->Field{let mut s=S{x:7} for i in 0..3{s.x=s.x+as_field(i)} s.x}",10),
        ] { scalar(body,expected); }
        scalar("struct S{x:Field,y:Field} struct Box{left:S,right:S} fn main()->Field{let mut b=Box{left:S{x:1,y:2},right:S{x:3,y:4}} let old=b b.left=b.right b.left.x=9 old.left.x*1000+b.left.x*100+b.left.y*10+b.right.x}",1943);
        for count in [1, 2, 3, 5] {
            let locals = (0..count)
                .map(|i| format!("let mut s{i}=S{{x:{},y:{}}}", i + 1, i + 11))
                .collect::<Vec<_>>()
                .join(" ");
            let reads = (0..count)
                .map(|i| format!("s{i}.x+s{i}.y"))
                .collect::<Vec<_>>()
                .join("+");
            for selected in 0..count {
                let source=format!("struct S{{x:Field,y:Field}} fn main()->Field{{{locals} s{selected}.x=99 {reads}}}");
                let expected =
                    (0..count).map(|i| i + 1 + i + 11).sum::<u64>() - (selected + 1) + 99;
                scalar(&source, expected);
            }
        }
        scalar("struct S{x:Field,y:Field} fn update(a:S)->S{let mut s=a s.x=sub(s.x,s.y) s} fn main()->Field{let original=S{x:9,y:2} let other=4 let changed=update(original) original.x*100+changed.x*10+changed.y+other}",976);
        scalar("struct S{x:Field,y:Field} fn replace(a:S)->Field{sub(a.x,a.y)} fn main()->Field{let mut s=S{x:9,y:2} let old=s s.x=replace(s) old.x*100+s.x*10+s.y}",972);
        scalar("struct Empty{} struct S{x:Empty,y:Field} fn main()->Field{let mut s=S{x:Empty{},y:7} s.x=Empty{} if s.x==(Empty{}){s.y}else{9}}",7);
        let input = data::nested();
        let source="program sample struct S{x:Noun,y:Noun} fn main(input:Noun)->Noun{let mut s=S{x:input,y:nox_noun_atom(9)} let old=s s.x=nox_noun_pair(s.x,s.y) nox_noun_pair(old.x,s.x)}";
        data::agrees(
            source,
            &input,
            &Noun::pair(input.clone(), Noun::pair(input.clone(), Atom(9))),
        );
        let source="program sample struct S{d:Digest,t:(Field,Noun)} fn main(input:Noun)->Noun{let mut s=S{d:nox_noun_identity(nox_noun_atom(0)),t:(7,nox_noun_atom(0))} s.d=nox_noun_identity(input) s.t=(9,input) let(n,x)=s.t if s.d==nox_noun_identity(input){nox_noun_pair(nox_noun_atom(n),x)}else{nox_noun_atom(0)}}";
        data::agrees(source, &input, &Noun::pair(Atom(9), input.clone()));
    });
}

#[test]
fn static_record_replacement_executes_once_against_old_value_and_keeps_traps() {
    support::worker(|| {
        let source="program sample struct S{x:Field,y:Field} struct Box{a:S,b:Field} fn main(input:Noun)->Noun{let mut b=Box{a:S{x:9,y:2},b:4} b.a.x=sub(b.a.x,b.a.y) nox_noun_atom(b.a.x+b.b)}";
        let bytes = data::compile(source);
        let mut trace = nox::trace::VecTrace::default();
        let run = data::run_traced(&bytes, &Atom(0), 1_000_000, 65536, 196608, &mut trace).unwrap();
        assert_eq!(run.bytes, Atom(11).encoded());
        assert_eq!(trace.0.iter().filter(|r| r.col(0) == 6).count(), 1);
        assert_eq!(
            data::run(&data::seed(source), &Atom(0)).unwrap().bytes,
            run.bytes
        );
        let source="program sample struct S{x:Noun} fn main(input:Noun)->Noun{let mut s=S{x:input} s.x=nox_noun_head(nox_noun_atom(1)) input}";
        for code in [data::compile(source), data::seed(source)] {
            assert_eq!(data::run(&code, &Atom(0)), Err("Error(AxisError)".into()));
        }
    });
}

#[test]
fn static_record_targets_require_a_mutable_local_root_and_exact_field_type() {
    support::worker(|| {
        for body in [
            "struct S{x:Field} fn main()->Field{let s=S{x:7} s.x=9 s.x}",
            "struct S{x:Field} fn f(s:S)->Field{s.x=9 s.x} fn main()->Field{f(S{x:7})}",
            "struct S{x:Field} fn main()->Field{let mut s=S{x:7} if true{let s=s s.x=9} s.x}",
            "struct S{x:Field} fn main()->Field{let mut s=S{x:7} s.x=true s.x}",
            "struct S{x:Field} fn make()->S{S{x:7}} fn main()->Field{make().x=9 7}",
            "struct S{x:Field} fn main()->Field{S{x:7}.x=9 7}",
            "struct S{x:Field} struct T{x:Field} struct Box{x:S} fn main()->Field{let mut b=Box{x:S{x:7}} b.x=T{x:9} 7}",
            "struct S{x:Field} fn main()->Field{let mut s=S{x:7} (s.x,s.x)=(8,9) 7}",
        ] {
            let source=format!("program sample {body}");
            match support::compile_only(source.as_bytes(),data::caps()) {
                support::Compilation::Errors(errors)=>assert_eq!(errors[0].code,5,"{source}"),
                other=>panic!("{source}: {other:?}"),
            }
            assert!(trident::compile_raw_artifact(&source,"oracle.tri",&trident::CompileOptions::default(),trident::NATIVE_ARTIFACT_LIMITS).is_err(),"seed {source}");
        }
    });
}
