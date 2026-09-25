//! Export checked direct TIR to the external warrior execution oracle on request.
#[test]
fn direct_tir_execution_fixtures_keep_checked_bindings() {
    let mut cases: Vec<(String, String, Vec<u64>)> = Vec::new();
    for (name, left, right) in [
        ("lt-inferred-explicit", "first([3])", "first<2>([7,9])"),
        ("lt-explicit-inferred", "first<1>([3])", "first([7,9])"),
        ("lt-inferred", "first([3])", "first([7,9])"),
        ("lt-constant", "first<1>([3])", "first<SIZE>([7,9])"),
        ("lt-parentheses", "(first([3]))", "(first<SIZE>([7,9]))"),
    ] {
        cases.push((name.into(), format!("program calls const SIZE:Field=2 fn first<N>(x:[Field;N])->U32{{as_u32(x[0])}} fn main(){{if {left} < {right} {{pub_write(1)}} else {{pub_write(0)}}}}"), vec![1]));
    }
    for (name, declarations, expected) in [
        ("constructor", "struct Pair{a:Field,b:Field} fn main(){let p=Pair{b:first<2>([7,9]),a:first([3])} pub_write(p.a*100+p.b)}", vec![307]),
        ("nested", "fn main(){pub_write(first([first([3]),9]))}", vec![3]),
        ("pass-through", "fn pass(x:[Field;2])->Field{first(x)} fn main(){pub_write(pass([7,9]))}", vec![7]),
        ("early-return", "fn early(flag:Bool)->Field{if flag{return first([7,9])} first<1>([3])} fn main(){pub_write(early(true)) pub_write(early(false))}", vec![7,3]),
        ("loop-return", "fn early()->Field{for i in 0..2{return first([7,9])} first<1>([3])} fn main(){pub_write(early())}", vec![7]),
    ] {
        cases.push((name.into(), format!("program calls fn first<N>(x:[Field;N])->Field{{x[0]}} {declarations}"), expected));
    }
    for (name, declarations, expected) in [
        ("array-return-binding", "fn id<N>(x:[Field;N])->[Field;N]{x} fn main(){let x=id([7,9]) pub_write(x[1])}", vec![9]),
        ("array-return-direct", "fn id<N>(x:[Field;N])->[Field;N]{x} fn main(){pub_write(id([7,9])[1])}", vec![9]),
        ("nominal-generic-abi", "struct Pair{a:Field,b:Field} fn copy<N>(p:Pair,x:[Field;N])->Pair{p} fn main(){let p=copy(Pair{a:3,b:7},[9,11]) pub_write(p.a*100+p.b)}", vec![307]),
        ("nominal-constant-owner", "const N:Field=2 struct Pair{words:[Field;N]} fn copy<N>(p:Pair,x:[Field;N])->Pair{p} fn main(){let p=copy(Pair{words:[7,9]},[11,13,17]) pub_write(p.words[1])}", vec![9]),
        ("generic-size-value", "fn size<N>(x:[Field;N])->Field{N} fn main(){pub_write(size([7,9]))}", vec![2]),
    ] {
        cases.push((name.into(), format!("program calls {declarations}"), expected));
    }
    let fixtures: Vec<_> = cases.into_iter().map(|(name, source, expected)| {
        let ir = super::exports::direct_ir(&source).unwrap();
        assert!(!format!("{ir:?}").contains("ERROR:"), "{name}");
        serde_json::json!({"case":name, "source":source, "tir":format!("{ir:?}"), "expected":expected})
    }).collect();
    if let Ok(path) = std::env::var("TRIDENT_CALLABLE_TIR_FIXTURES") {
        std::fs::write(path, serde_json::to_vec_pretty(&fixtures).unwrap()).unwrap();
    }
}
