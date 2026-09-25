use super::{nouns::data, support};

#[test]
fn field_array_types_raw_indices_and_delimiters_reject_before_emission() {
    support::worker(|| {
        for (body, code) in [
            ("fn main()->Field{let a:[Field;2]=[7] 0}", 5),
            ("fn f(a:[Field;2])->Field{0} fn main()->Field{f([7])}", 5),
            ("fn f()->[Field;2]{[7]} fn main()->Field{0}", 5),
            ("fn main()->Field{[7][1]}", 5),
            ("fn main()->Field{[7][((18446744069414584321))]}", 5),
            ("fn main()->Field{if [0]{7}else{9}}", 5),
            ("fn main()->Field{if [1,2]==(1,2){7}else{9}}", 5),
            (
                "struct Empty{} fn main()->Field{if []==(Empty{}){7}else{9}}",
                5,
            ),
            ("fn main()->Field{[7][18446744069414584321]}", 5),
            ("fn main()->Field{[7][18446744073709551615]}", 5),
            ("fn main()->Field{[][0]}", 5),
            ("fn main()->Field{[7][true]}", 5),
            ("fn main()->Field{let(a,b)=[7,9] a+b}", 5),
            ("fn main()->Field{let a=[7] a=[9] 0}", 5),
            ("fn main()->Field{if [7]==[7,9]{1}else{0}}", 5),
            ("fn main()->Field{let a=[,] 0}", 2),
            ("fn main()->Field{let a=[1,,2] 0}", 2),
            ("fn main()->Field{let a=[1,2) 0}", 2),
            ("fn main()->Field{let a=[(1,2] 0}", 2),
            ("fn main()->Field{let a=[1} 0}", 2),
            ("fn main()->Field{[7][]}", 2),
            ("fn main()->Field{[7][0,1]}", 2),
            ("fn main()->Field{let a=[7;2] 0}", 6),
            ("fn main()->Field{let a:[Field 2]=[1,2] 0}", 2),
            ("fn main()->Field{let a:[Field;2)=[1,2] 0}", 2),
        ] {
            let source = format!("program sample {body}");
            match support::compile_only(source.as_bytes(), data::caps()) {
                support::Compilation::Errors(errors) => {
                    assert_eq!(errors[0].code, code, "{source}")
                }
                other => panic!("{source}: {other:?}"),
            }
            assert!(
                trident::compile(&source, "oracle.tri").is_err(),
                "seed {source}"
            );
        }
        for (body, code) in [
            ("fn main()->Field{let a=[true] 0}", 6),
            ("fn main()->Field{let a=[[7]] 0}", 6),
            ("fn f(a:[Bool;1]){} fn main()->Field{0}", 6),
            ("fn f(a:[Field;N]){} fn main()->Field{0}", 6),
            ("fn f(a:[Field;2+1]){} fn main()->Field{0}", 6),
            (
                "fn f(a:[Field;18446744069414584321]){} fn main()->Field{0}",
                7,
            ),
            ("fn main()->Field{let mut a=[7] a[0]=9 a[0]}", 2),
            ("fn main()->Field{let mut a=[7] a[1]=9 a[0]}", 5),
        ] {
            let source = format!("program sample {body}");
            match support::compile_only(source.as_bytes(), data::caps()) {
                support::Compilation::Errors(errors) => {
                    assert_eq!(errors[0].code, code, "{source}")
                }
                other => panic!("{source}: {other:?}"),
            }
        }
    });
}
