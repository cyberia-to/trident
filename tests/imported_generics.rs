use trident::CompileOptions;
fn load(arena: &mut nox::Reduction<16384>, text: &[u8], offset: &mut usize) -> nox::Order {
    if text[*offset] == b'[' {
        *offset += 1;
        let head = load(arena, text, offset);
        *offset += 1;
        let tail = load(arena, text, offset);
        *offset += 1;
        arena.pair(head, tail).unwrap()
    } else {
        let start = *offset;
        while *offset < text.len() && text[*offset].is_ascii_digit() {
            *offset += 1;
        }
        arena
            .atom(nebu::Goldilocks::new(
                std::str::from_utf8(&text[start..*offset])
                    .unwrap()
                    .parse()
                    .unwrap(),
            ))
            .unwrap()
    }
}
fn execute(assembly: &str, subject: &str) -> u64 {
    let mut arena = nox::Reduction::<16384>::new();
    let formula = load(&mut arena, assembly.as_bytes(), &mut 0);
    let subject = load(&mut arena, subject.as_bytes(), &mut 0);
    match nox::reduce(
        &mut arena,
        subject,
        formula,
        1_000_000,
        &nox::NullCalls,
        &mut nox::NoTrace,
    ) {
        nox::Outcome::Ok(value, _) => arena.atom_value(value).unwrap().as_u64(),
        other => panic!("{other:?}"),
    }
}
#[test]
fn imported_generics_keep_lexical_sizes_and_nested_entry_layout() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("shapes.tri"),"module shapes\nconst N:U32=9\nconst K:U32=2\npub struct Pair {a:Field,b:Field}\nfn helper(x:Field)->Field{x+7}\npub fn first<M>(x:[Field;M])->Field{helper(x[0])}\npub fn outer<N>(x:[Field;N])->Field{first([11,13,17])+first(x)}\npub fn padded<N>(x:[Field;N+K])->Field{x[0]*100+x[2]}\n").unwrap();
    let entry = dir.path().join("entry.tri");
    std::fs::write(&entry,"program entry\nuse shapes\nconst K:U32=17\nfn main(x:[shapes.Pair;2])->Field{shapes.first<3>([101,103,107])+shapes.outer([19,23])+shapes.padded<1>([3,5,7])+x[1].b}").unwrap();
    for profile in ["debug", "release"] {
        let options = CompileOptions::for_profile(profile);
        let bundle = trident::compile_to_bundle(&entry, &options).unwrap();
        assert_eq!(execute(&bundle.assembly, "[13 [11 [7 [5 0]]]]"), 472);
        assert_eq!(
            bundle.assembly,
            trident::compile_to_bundle(&entry, &options)
                .unwrap()
                .assembly
        );
    }
}
#[test]
fn generic_calls_reject_bad_arity_shapes_bodies_and_dimensions() {
    let dir = tempfile::tempdir().unwrap();
    let entry = dir.path().join("entry.tri");
    for (definition, call) in [
        ("pub fn f<N,N>(x:[Field;N])->Field{x[0]}", "f<2,2>([1,2])"),
        ("pub fn f<N>(x:[Field;N])->Field{f<N+1>(x)}", "f<2>([1,2])"),
        ("pub fn f<N>(x:[Field;N])->Field{x[0]}", "f<2,3>([1,2])"),
        ("pub fn f<N>(x:[Field;N])->Field{x[0]}", "f<3>([1,2])"),
        ("pub fn f<N>(x:[Field;N])->Field{x[0]}", "f<0>([])"),
        ("pub fn f<N>(x:[Field;N])->Field{true}", "f<2>([1,2])"),
        (
            "pub fn f<N>(x:[Field;N])->Field{return true}",
            "f<2>([1,2])",
        ),
        ("pub fn f<N>(x:[Field;N])->Field{x[0]}", "f<MISSING>([1,2])"),
        (
            "pub fn f<N>(x:[Field;N])->Field{x[0]}",
            "f<18446744073709551615+1>([1,2])",
        ),
        (
            "pub fn f<N>(x:[Field;N])->Field{let y:Bool=x[0]\n0}",
            "f<2>([1,2])",
        ),
    ] {
        std::fs::write(
            dir.path().join("generic.tri"),
            format!("module generic\n{definition}"),
        )
        .unwrap();
        std::fs::write(
            &entry,
            format!("program entry\nuse generic\nfn main()->Field{{generic.{call}}}"),
        )
        .unwrap();
        assert!(
            trident::compile_to_bundle(&entry, &CompileOptions::default()).is_err(),
            "accepted {definition}: {call}"
        );
    }
}
#[test]
fn ordinary_return_contracts_reject_wrong_types_and_missing_paths() {
    for body in [
        "fn main()->Field{true}",
        "fn main()->U32{4294967296}",
        "fn main(x:Field)->Field{if x==0{return 7}}",
        "fn main()->Field{for i in 0..0{return 7}}",
        "fn main()->Field{if true{return false}\n7}",
        "fn main()->Field{return}",
        "fn main(){return 1}",
        "const flag:Field=0\nfn main(flag:Field)->Field{if flag{return 7}}",
        "const flag:Field=0\nfn main(x:Field)->Field{let flag=x\nif flag{return 7}}",
    ] {
        let source = format!("program entry\n{body}");
        assert!(
            trident::check(&source, "entry.tri").is_err(),
            "accepted {body}"
        );
        assert!(
            trident::compile_with_options(&source, "entry.tri", &CompileOptions::default())
                .is_err()
        );
    }
    for body in [
        "fn main()->Field{for i in 0..1{return 7}}",
        "fn main(x:Bool)->Field{if x{return 7}else{return 9}}",
        "fn main()->Field{if true{return 7}}",
        "fn main(x:Field)->Field{if x==0{7}else{9}}",
        "fn main(x:Field)->Field{if x==0{return 7}else{9}}",
        "fn main(x:Bool)->Field{match x{true=>{7} false=>{9}}}",
        "fn main()->Field{for i in 0..0{return 9}\n7}",
        "fn main(){return}",
    ] {
        assert!(
            trident::check(&format!("program entry\n{body}"), "entry.tri").is_ok(),
            "rejected {body}"
        );
    }
}
#[test]
fn standalone_generics_are_checked_after_concrete_substitution() {
    let source = "program entry\nfn f<N>(x:[Field;N])->Field{true}\nfn main()->Field{f<2>([1,2])}";
    assert!(trident::check(source, "entry.tri").is_err());
    assert!(trident::check_silent(source, "entry.tri").is_err());
    assert!(
        trident::compile_with_options(source, "entry.tri", &CompileOptions::default()).is_err()
    );
    let good = source.replace("->Field{true}", "->Field{x[0]}");
    assert_eq!(
        execute(
            &trident::compile_with_options(&good, "entry.tri", &CompileOptions::default()).unwrap(),
            "0"
        ),
        1
    );
}

#[test]
fn constants_keep_nominal_types_and_check_u32_range() {
    assert!(
        trident::check(
            "program entry\nfn main(x:[Field;N])->Field{x[0]}\nconst N:U32=2",
            "entry.tri"
        )
        .is_ok()
    );

    let good = "program entry\nconst C:U32=4294967295\nfn main()->U32{C}";
    assert!(trident::check(good, "entry.tri").is_ok());
    assert_eq!(
        execute(
            &trident::compile_with_options(good, "entry.tri", &CompileOptions::default()).unwrap(),
            "0"
        ),
        4294967295
    );
    assert!(trident::check(&good.replace("4294967295", "4294967296"), "entry.tri").is_err());
    assert!(trident::check(&good.replace("main()->U32", "main()->Field"), "entry.tri").is_err());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("values.tri"),
        "module values\nconst C:U32=7\npub fn value<N>(x:[Field;N])->U32{C}",
    )
    .unwrap();
    let path = dir.path().join("entry.tri");
    std::fs::write(
        &path,
        "program entry\nuse values\nconst C:Field=99\nfn main()->U32{values.value<2>([1,2])}",
    )
    .unwrap();
    assert_eq!(
        execute(
            &trident::compile_to_bundle(&path, &CompileOptions::default())
                .unwrap()
                .assembly,
            "0"
        ),
        7
    );
}
