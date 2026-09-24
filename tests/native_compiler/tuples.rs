use super::{codegen, nouns::data, support};
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
fn tuples_preserve_nested_whole_values_through_typed_functions_and_destructuring() {
    support::worker(|| {
        let input = data::nested();
        for body in [
            "fn main(input:Noun)->Noun{let t=(input,7) let (n,x)=t nox_noun_pair(n,nox_noun_atom(x))}",
            "fn id(x:(Noun,Field))->(Noun,Field){x} fn main(input:Noun)->Noun{let (n,x)=id((input,7)) nox_noun_pair(n,nox_noun_atom(x))}",
            "fn make(n:Noun)->(Noun,Field){return (n,7)} fn main(input:Noun)->Noun{let (n,x)=make(input) nox_noun_pair(n,nox_noun_atom(x))}",
            "fn main(input:Noun)->Noun{let (pair,u):((Noun,Bool),U32)=((input,true),as_u32(7)) let (n,flag)=pair if flag{nox_noun_pair(n,nox_noun_atom(as_field(u)))}else{input}}",
            "fn main(input:Noun)->Noun{let mut t=(input,7) let old=t t=(nox_noun_atom(0),9) let(n,x)=old nox_noun_pair(n,nox_noun_atom(x))}",
            "fn unit(){} fn main(input:Noun)->Noun{let (u,n,x)=(unit(),input,7) if u==unit(){nox_noun_pair(n,nox_noun_atom(x))}else{input}}",
            "fn main(input:Noun)->Noun{let mut t=(input,7) for i in 0..3{let(n,x)=t t=(nox_noun_head(nox_noun_pair(n,input)),x)} let(n,x)=t nox_noun_pair(n,nox_noun_atom(x))}",
        ] {
            data::agrees(&format!("program sample {body}"),&input,&Noun::pair(input.clone(),Atom(7)));
        }
        let words: Vec<_> = input.encoded()[8..40]
            .chunks_exact(8)
            .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
            .collect();
        let expected = Noun::pair(
            Noun::pair(Atom(words[0]), Atom(words[1])),
            Noun::pair(Atom(words[2]), Atom(words[3])),
        );
        for body in [
            "let(a,b,c,d):Digest=nox_noun_identity(input) nox_noun_pair(nox_noun_pair(nox_noun_atom(a),nox_noun_atom(b)),nox_noun_pair(nox_noun_atom(c),nox_noun_atom(d)))",
            "let mut a=0 let mut b=0 let mut c=0 let mut d=0 (a,b,c,d)=nox_noun_identity(input) nox_noun_pair(nox_noun_pair(nox_noun_atom(a),nox_noun_atom(b)),nox_noun_pair(nox_noun_atom(c),nox_noun_atom(d)))",
        ] { data::agrees(&data::source(body),&input,&expected); }
        data::agrees("program sample fn identity(n:Noun)->Digest{nox_noun_identity(n)} fn main(input:Noun)->Noun{let(a,b,c,d)=identity(input) nox_noun_pair(nox_noun_pair(nox_noun_atom(a),nox_noun_atom(b)),nox_noun_pair(nox_noun_atom(c),nox_noun_atom(d)))}", &input, &expected);
    });
}

#[test]
fn tuple_calls_grouping_and_owned_element_links_keep_source_order_and_types() {
    support::worker(|| {
        for (body,expected) in [
            ("fn id(x:(Field))->(Field){x} fn main()->Field{7}",7),
            ("fn id(x:(Field,Bool))->(Field,Bool){x} fn main()->Field{let (a,b)=id(((7),true,),) if b{a}else{3}}",7),
            ("fn f(x:Field,y:(Field,Bool),z:Field)->Field{let(a,b)=y if b{x*100+a*10+z}else{0}} fn main()->Field{f(1,(2,true),3)}",123),
            ("fn f(x:Field)->Field{x+1} fn main()->Field{let(a,b,c)=(f(1),f(f(2)),f(3)) a*100+b*10+c}",244),
            ("fn main()->Field{let x:(Field,(Bool,U32))=(7,(true,as_u32(8))) if x==(7,(true,as_u32(8))){9}else{3}}",9),
            ("fn main()->Field{let(a,b,c)=(1,((2,3)),4) let(x,y)=b a*1000+x*100+y*10+c}",1234),
            ("fn unit(){} fn main()->Field{let t=(unit(),7) if t==(unit(),8){3}else{9}}",9),
            ("fn main()->Field{let(a,b,)=(7,true,) if b{a}else{3}}",7),
            ("fn main()->Field{let(mutual,_)=(7,true) mutual}",7),
            ("fn main()->Field{let x=2 let y=7 let(x,y)=(y,x) x*10+y}",72),
            ("fn main()->Field{let mut(a,b)=(2,7) ((a,b))=(b,a) a*10+b}",72),
            ("fn main()->Field{let(x,x)=(7,true) if x{9}else{3}}",9),
            ("fn main()->Field{let mut x=1 (x,x)=(7,9) x}",9),
            ("fn main()->Field{let a=7 if true{let(a,b)=(2,3) a+b} a}",7),
        ] { scalar(body,expected); }
        // Full nested Digest indexing inside a tuple and calls inside an index.
        let source=data::source("let d=nox_noun_identity(input) let(a,b)=(d[sub(3,1)],d[(1+2)]) nox_noun_pair(nox_noun_atom(a),nox_noun_atom(b))");
        let input = data::nested();
        let bytes = input.encoded();
        let word = |i: usize| u64::from_le_bytes(bytes[8 + i * 8..16 + i * 8].try_into().unwrap());
        data::agrees(&source, &input, &Noun::pair(Atom(word(2)), Atom(word(3))));
    });
}

#[test]
fn tuple_destructuring_evaluates_rhs_once_before_ordered_writes_or_discards() {
    support::worker(|| {
        let source=data::source("let mut a=1 let mut b=2 (a,b)=(sub(9,a),sub(7,b)) nox_noun_pair(nox_noun_atom(a),nox_noun_atom(b))");
        let bytes = data::compile(&source);
        let mut trace = nox::trace::VecTrace::default();
        let run = data::run_traced(&bytes, &Atom(0), 1_000_000, 65536, 196608, &mut trace).unwrap();
        assert_eq!(run.bytes, Noun::pair(Atom(8), Atom(5)).encoded());
        assert_eq!(trace.0.iter().filter(|r| r.col(0) == 6).count(), 2);
        let exact = data::run(&bytes, &Atom(0)).unwrap();
        assert_eq!(
            data::run_traced(
                &bytes,
                &Atom(0),
                exact.reductions,
                exact.frames,
                exact.nodes,
                &mut nox::NoTrace
            )
            .unwrap(),
            exact
        );
        for (budget, frames, nodes) in [
            (exact.reductions - 1, exact.frames, exact.nodes),
            (exact.reductions, exact.frames - 1, exact.nodes),
            (exact.reductions, exact.frames, exact.nodes - 1),
        ] {
            assert!(
                data::run_traced(&bytes, &Atom(0), budget, frames, nodes, &mut nox::NoTrace)
                    .is_err()
            );
        }

        for body in [
            "let(_,_)=(nox_noun_head(nox_noun_atom(7)),nox_noun_as_field(input)) input",
            "let (x,y)=(nox_noun_head(nox_noun_atom(7)),7) input",
            "let mut a=input let mut b=7 (a,b)=(nox_noun_head(nox_noun_atom(7)),nox_noun_as_field(input)) input",
        ] {
            let source=data::source(body);
            for bytes in [data::compile(&source),data::seed(&source)] {
                assert_eq!(data::run(&bytes,&data::nested()),Err("Error(AxisError)".into()));
            }
        }
        data::agrees(
            &data::source("if false{let(_,_)=(nox_noun_head(nox_noun_atom(7)),7)} input"),
            &data::nested(),
            &data::nested(),
        );
    });
}

#[test]
fn tuple_projection_and_construction_depth_matches_the_independent_formula_dag() {
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
        for source in [
            "program sample fn main()->Field{let (a,b)=(7,9) a+b}",
            "program sample fn main()->Field{let(a,b)=((1,2),sub(sub(sub(sub(sub(sub(sub(sub(45,1),2),3),4),5),6),7),8)) let(c,d)=a c+d+b}",
            "program sample fn main()->Field{let(a,b)=(sub(sub(sub(sub(sub(sub(sub(sub(45,1),2),3),4),5),6),7),8),(1,2)) let(c,d)=b a+c+d}",
            "program sample fn pair(x:Field)->(Field,Bool){(x,true)} fn main()->Field{let(a,b)=pair(7) if b{a}else{3}}",
        ] {
            let generate=|cap|codegen::generate_with_input::<{1<<20}>(&probe,|arena|{
                let source=support::data::Bytes::from_slice(arena,source.as_bytes(),4096).unwrap().encode(arena).unwrap();
                let cap=support::data::atom(arena,cap).unwrap();support::data::pair(arena,cap,source).unwrap()
            });
            let bytes=generate(4096).unwrap();let exact=codegen::artifact_depth(&bytes)+4;
            assert_eq!(generate(exact-1),Err(7));assert_eq!(generate(exact).unwrap(),bytes);assert_eq!(generate(exact+1).unwrap(),bytes);
            assert_eq!(support::run_artifact(&bytes),support::rust_value(source));
        }
    });
}
