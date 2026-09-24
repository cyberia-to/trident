use super::{nouns::data, support};
use data::{Noun, Noun::Atom};

fn scalar(body: &str, expected: u64) {
    let source = format!("program sample {body}");
    let bytes = data::compile(&source);
    assert_eq!(support::run_artifact(&bytes), expected, "{source}");
    assert_eq!(support::rust_value(&source), expected, "seed {source}");
}

#[test]
fn nominal_source_values_keep_empty_nested_shorthand_and_typed_call_layouts() {
    support::worker(|| {
        for (body, expected) in [
            ("struct Empty{} fn main()->Field{let x=Empty{} if x==(Empty{}){7}else{9}}",7),
            ("struct Pair{x:Field,y:Field} fn main()->Field{let x=7 let p:Pair=Pair{y:9,x,} p.x*10+p.y}",79),
            ("pub struct S{pub x:Field} pub fn id(x:S)->S{x} fn main()->Field{id(S{x:7}).x}",7),
            ("struct S{x:Field} struct Box{inner:S,n:Field} fn main()->Field{Box{n:9,inner:S{x:7}}.inner.x}",7),
            ("struct Unit{} fn main()->Field{let x:Unit=Unit{} if x==(Unit{}){7}else{9}}",7),
            ("fn main()->Field{let x:Later=Later{x:7} x.x} struct Later{x:Field}",7),
            ("struct S{x:Field} fn main()->Field{let mut x=S{x:7} let old=x x=S{x:9} old.x*10+x.x}",79),
            ("struct S{flag:Bool} fn main()->Field{if (S{flag:true}).flag{7}else{9}}",7),
            ("struct S{flag:Bool} fn f(s:S)->Bool{s.flag} fn main()->Field{if f(S{flag:true}){7}else{9}}",7),
            ("struct S{x:Field} fn main()->Field{let(a,b):(S,S)=(S{x:7},S{x:9}) a.x*10+b.x}",79),
            ("struct S{x:(Field,Bool)} fn main()->Field{let s=S{x:(7,true)} let(n,b)=s.x if b{n}else{9}}",7),
            ("struct S{x:Field} fn main()->Field{let mut sum=0 for i in 0..3{let s=S{x:as_field(i)} sum=sum+s.x} sum}",3),
        ] { scalar(body, expected); }
        let input = data::nested();
        let source="program sample struct S{value:Noun,flag:Bool} fn id(x:S)->S{x} fn main(input:Noun)->Noun{let x=id(S{flag:true,value:input}) if x.flag{x.value}else{nox_noun_atom(0)}}";
        data::agrees(source, &input, &input);
        let source="program sample struct S{value:Noun,fields:(Field,Bool),digest:Digest} fn main(input:Noun)->Noun{let s=S{digest:nox_noun_identity(input),fields:(7,true),value:input} let(x,b)=s.fields if b{nox_noun_pair(s.value,nox_noun_pair(nox_noun_atom(x),nox_noun_atom(s.digest[2])))}else{input}}";
        let limb = u64::from_le_bytes(input.encoded()[24..32].try_into().unwrap());
        data::agrees(
            source,
            &input,
            &Noun::pair(input.clone(), Noun::pair(Atom(7), Atom(limb))),
        );
    });
}

#[test]
fn nominal_initializers_execute_once_in_declaration_order_and_reads_evaluate_base_once() {
    support::worker(|| {
        let source="program sample struct Pair{first:Field,second:Field} fn make()->Pair{Pair{second:sub(8,3),first:sub(9,2)}} fn main(input:Noun)->Noun{let p=make() nox_noun_pair(nox_noun_atom(p.first),nox_noun_atom(p.second))}";
        let code = data::compile(source);
        let mut trace = nox::trace::VecTrace::default();
        let run = data::run_traced(&code, &Atom(0), 1_000_000, 65536, 196608, &mut trace).unwrap();
        assert_eq!(run.bytes, Noun::pair(Atom(7), Atom(5)).encoded());
        assert_eq!(trace.0.iter().filter(|r| r.col(0) == 6).count(), 2);
        let source="program sample struct Pair{first:Field,second:Noun} fn main(input:Noun)->Noun{let p=Pair{second:nox_noun_head(nox_noun_atom(1)),first:as_field(as_u32(4294967296))} p.second}";
        for code in [data::compile(source), data::seed(source)] {
            assert_eq!(data::run(&code, &Atom(0)), Err("Error(InvZero)".into()));
        }
        let source="program sample struct S{x:Field} fn make()->S{S{x:sub(9,2)}} fn main(input:Noun)->Noun{nox_noun_atom(make().x)}";
        let code = data::compile(source);
        let mut trace = nox::trace::VecTrace::default();
        assert_eq!(
            data::run_traced(&code, &Atom(0), 1_000_000, 65536, 196608, &mut trace)
                .unwrap()
                .bytes,
            Atom(7).encoded()
        );
        assert_eq!(trace.0.iter().filter(|r| r.col(0) == 6).count(), 1);
        let source="program sample struct S{x:Noun} fn main(input:Noun)->Noun{let unused=S{x:nox_noun_head(nox_noun_atom(1))} input}";
        for code in [data::compile(source), data::seed(source)] {
            assert_eq!(data::run(&code, &Atom(0)), Err("Error(AxisError)".into()));
        }
    });
}

#[test]
fn nominal_32_field_projection_and_complete_long_names_preserve_distinct_identity() {
    support::worker(|| {
        let fields = (0..32)
            .map(|i| format!("f{i}:Field"))
            .collect::<Vec<_>>()
            .join(",");
        let values = (0..32)
            .rev()
            .map(|i| format!("f{i}:{i}"))
            .collect::<Vec<_>>()
            .join(",");
        scalar(&format!("struct Wide{{{fields}}} fn main()->Field{{let w=Wide{{{values}}} w.f31*100+w.f0+w.f17}}"),3117);
        let name = "Snnnnnnnn";
        scalar(&format!("struct {name}{{common_prefix_a:Field,common_prefix_b:Field}} fn main()->Field{{let p={name}{{common_prefix_b:9,common_prefix_a:7}} p.common_prefix_a*10+p.common_prefix_b}}"),79);
        let long_name = format!("S{}", "n".repeat(256));
        scalar(
            &format!("struct {long_name}{{x:Field}} fn main()->Field{{{long_name}{{x:7}}.x}}"),
            7,
        );
        let prefix = "f".repeat(256);
        scalar(
            &format!("struct S{{{prefix}a:Field,{prefix}b:Field}} fn main()->Field{{7}}"),
            7,
        );
    });
}
