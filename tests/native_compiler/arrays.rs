use super::{nouns::data, support};
use data::Noun::Atom;

fn scalar(body: &str, expected: u64) {
    let source = format!("program sample {body}");
    assert_eq!(
        support::run_artifact(&data::compile(&source)),
        expected,
        "{source}"
    );
    // Use the native seed path; the legacy assembly API lacks dynamic indexing.
    let oracle = source.replacen("fn main()->Field", "fn result()->Field", 1)
        + " fn main(input:Noun)->Noun{nox_noun_atom(result())}";
    assert_eq!(
        data::run(&data::seed(&oracle), &Atom(0)).unwrap().bytes,
        Atom(expected).encoded(),
        "seed {source}"
    );
}

#[test]
fn field_arrays_preserve_whole_values_annotations_and_owned_delimiters() {
    support::worker(|| {
        for (body, expected) in [
            ("fn main()->Field{let a:[Field;0]=[] if a==[]{7}else{9}}",7),
            ("fn main()->Field{let a:[Field;1]=[7,] a[0]}",7),
            ("fn main()->Field{let a:[Field;3]=[2,4,7] a[0]*100+a[as_u32(1)]*10+a[2]}",247),
            ("fn id(a:[Field;2])->[Field;2]{a} fn main()->Field{id([7,9])[1]}",9),
            ("fn main()->Field{let(a,b):([Field;2],Field)=([7,9],1) a[b]}",9),
            ("struct S{a:[Field;2],b:Field} fn main()->Field{let mut s=S{a:[7,9],b:2} let old=s s.a=[3,4] old.a[0]*100+s.a[1]*10+s.b}",742),
            ("fn main()->Field{let mut a=[7,9] let old=a a=[3,4] old[1]*10+a[0]}",93),
            ("fn f(x:Field)->Field{x+1} fn g(a:[Field;2],b:Field)->Field{a[b]} fn main()->Field{g([1,f(6)],([1,0])[0])}",7),
            ("fn main()->Field{let a=[1,2,3] let mut n=0 for i in 0..3{n=n+a[i]} n}",6),
            ("fn main()->Field{if [1][0]{7}else{9}}",9),
            ("struct S{x:Field} fn main()->Field{if [S{x:0}.x][0]{7}else{9}}",7),
            ("fn main()->Field{let(mutual,b)=([7][0],9) mutual+b}",16),
            ("fn main()->Field{let mut a=0 let mut b=0 (a,b)=([7][0],9) a+b}",16),
            ("struct S{x:Field} fn main()->Field{let mut s=S{x:0} s.x=[7][0] s.x}",7),
            ("fn main()->Field{[7][18446744069414584321+0]}",7),
            ("fn main()->Field{[7][(000000000000000000000000000000)]}",7),
        ] { scalar(body,expected); }
        for count in [1, 24, 31, 32, 33, 65] {
            let values = (0..count)
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",");
            scalar(
                &format!(
                    "fn main()->Field{{let a:[Field;{count}]=[{values}] a[as_u32({})]}}",
                    count - 1
                ),
                (count - 1) as u64,
            );
        }
        let plain = data::compile("program sample fn main()->Field{7}");
        for unused in ["fn unused()->Field{[7][0]}", "fn main()->Field{[7][0]}"] {
            assert_eq!(
                plain,
                data::compile(&format!("program sample {unused} fn main()->Field{{7}}"))
            );
        }
    });
}

#[test]
fn array_read_helpers_share_the_final_table_with_functions_and_loops() {
    support::worker(|| {
        for count in [1, 2, 3, 4, 7, 8] {
            let functions = (0..count)
                .map(|i| format!("fn f{i}(a:[Field;2])->Field{{a[0]+a[1]}}"))
                .collect::<Vec<_>>()
                .join(" ");
            let calls = (0..count)
                .map(|i| format!("f{i}([3,4])"))
                .collect::<Vec<_>>()
                .join("+");
            scalar(
                &format!("{functions} fn main()->Field{{{calls}}}"),
                7 * count as u64,
            );
        }
        scalar("fn f(a:[Field;2])->Field{let mut s=0 for i in 0..2{s=s+a[i]} s} fn main()->Field{let mut s=0 for i in 0..2{s=s+f([3,4])} s}",14);
        let source="program sample fn main(input:Noun)->Noun{let a=[7,9] nox_noun_atom(a[nox_noun_as_field(input)])}";
        let code = data::compile(source);
        for (index, expected) in [(0, 7), (1, 9)] {
            data::agrees(source, &Atom(index), &Atom(expected));
        }
        for index in [2, 3, 4294967296, 18446744069414584320] {
            for bytes in [&code, &data::seed(source)] {
                assert_eq!(data::run(bytes, &Atom(index)), Err("Error(InvZero)".into()));
            }
        }
        let source = data::source("let a:[Field;0]=[] nox_noun_atom(a[nox_noun_as_field(input)])");
        for bytes in [data::compile(&source), data::seed(&source)] {
            assert_eq!(data::run(&bytes, &Atom(0)), Err("Error(InvZero)".into()));
        }
    });
}
