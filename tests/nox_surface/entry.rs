//! Accepted typed entries must share the public-word ABI with the warrior.
use super::*;

fn execute(source: &str, profile: &str, subject: N) -> Result<u64, String> {
    let options = CompileOptions::for_profile(profile);
    let formula = trident::compile_with_options(source, "entry.tri", &options).unwrap();
    let mut arena = Reduction::<4096>::new();
    let f = load(&mut arena, &parse(&formula));
    let s = load(&mut arena, &subject);
    match reduce(&mut arena, s, f, 100_000, &NullCalls, &mut NoTrace) {
        Outcome::Ok(r, _) => arena
            .atom_value(r)
            .map(|v| v.as_u64())
            .ok_or("non-atom output".into()),
        _ => Err("execution rejected".into()),
    }
}

#[test]
fn exact_arity_and_narrow_types_apply_to_unused_parameters() {
    for profile in ["debug", "release"] {
        let source = "program entry\nfn main(a: Field,b: U32,flag: Bool) -> Field { 11 }";
        for flag in [0, 1] {
            assert_eq!(
                execute(source, profile, subject_noun(&[7, u32::MAX as u64, flag])).unwrap(),
                11
            );
        }
        for input in [
            vec![],
            vec![7],
            vec![7, 19],
            vec![7, 19, 0, 31],
            vec![7, 1u64 << 32, 0],
            vec![7, 19, 2],
        ] {
            assert!(
                execute(source, profile, subject_noun(&input)).is_err(),
                "{input:?}"
            );
        }
        // A non-scalar field or a nonzero list terminator must not survive.
        let malformed = N::Cell(
            Box::new(N::Atom(0)),
            Box::new(N::Cell(
                Box::new(N::Atom(19)),
                Box::new(N::Cell(
                    Box::new(N::Cell(Box::new(N::Atom(7)), Box::new(N::Atom(0)))),
                    Box::new(N::Atom(0)),
                )),
            )),
        );
        assert!(execute(source, profile, malformed).is_err());
        let no_args = "program entry\nfn main() -> Field { 11 }";
        assert_eq!(execute(no_args, profile, subject_noun(&[])).unwrap(), 11);
        assert!(execute(no_args, profile, subject_noun(&[1])).is_err());
        assert!(execute(no_args, profile, N::Atom(1)).is_err());
    }
}

#[test]
fn aggregate_and_digest_input_preserve_every_declared_word() {
    let mut source = String::from(
        "program entry\nstruct Inner { value: U32, flag: Bool }\nstruct Pair { a: Field, inner: Inner }\nfn main(pair: Pair, tuple: (Field,U32), words: [Field;20], digest: Digest) -> Field {\n assert(pair.a == 7)\n assert(as_field(pair.inner.value) == 19)\n assert(pair.inner.flag)\n let (a,b) = tuple\n assert(a == 23)\n assert(as_field(b) == 29)\n",
    );
    for i in 0..20 {
        source.push_str(&format!("assert(words[{i}] == {})\n", 101 + i));
    }
    for (i, value) in [307, 311, 313, 317].iter().enumerate() {
        source.push_str(&format!("assert(digest[{i}] == {value})\n"));
    }
    source.push_str("pair.a*100+as_field(pair.inner.value)\n}");
    let mut input = vec![7, 19, 0, 23, 29];
    input.extend(101..121);
    input.extend([307, 311, 313, 317]);
    for profile in ["debug", "release"] {
        assert_eq!(
            execute(&source, profile, subject_noun(&input)).unwrap(),
            719
        );
        for (index, word) in input.iter().enumerate() {
            let mut changed = input.clone();
            changed[index] = word + 1;
            assert!(
                execute(&source, profile, subject_noun(&changed)).is_err(),
                "leaf {index} was ignored"
            );
        }
        assert!(execute(&source, profile, subject_noun(&input[..input.len() - 1])).is_err());
    }
}

#[test]
fn ordinary_call_abi_and_native_bool_encoding_are_preserved() {
    let source = "program entry\nfn helper(a:Field,b:U32,flag:Bool)->Field { if flag { return a*100+as_field(b) }\n 31 }\nfn main(a:Field,b:U32,flag:Bool)->Field { helper(a,b,flag) }";
    for profile in ["debug", "release"] {
        assert_eq!(
            execute(source, profile, subject_noun(&[7, 19, 0])).unwrap(),
            719
        );
        assert_eq!(
            execute(source, profile, subject_noun(&[7, 19, 1])).unwrap(),
            31
        );
    }
}

#[test]
fn excessive_or_unresolved_entry_layouts_fail_before_execution() {
    for signature in ["a: [Field;59]", "a: [[Field;0];59]", "a: XField"] {
        let source = format!("program entry\nfn main({signature})->Field {{ 11 }}");
        assert!(trident::compile_with_options(&source, "entry.tri", &nox_options()).is_err());
    }
}

#[test]
fn deep_aggregate_reads_and_writes_keep_full_paths_and_siblings() {
    let padding = (0..40)
        .map(|i| format!("e{i}: [Field;0],"))
        .collect::<String>();
    let declarations = format!(
        "program entry\nstruct Inner {{ {padding} value:Field, neighbor:Field }}\nstruct Outer {{ {padding} inner:Inner, neighbor:Field }}\n"
    );
    let source = format!(
        "{declarations}fn main(x:Outer)->Field {{ x.inner.value*100+x.inner.neighbor*10+x.neighbor }}"
    );
    let edited = format!(
        "{declarations}fn main(x:Outer)->Field {{ let mut y=x\n y.inner.value=y.inner.value+2\n y.inner.neighbor=3\n y.inner.value*100+y.inner.neighbor*10+y.neighbor }}"
    );
    for profile in ["debug", "release"] {
        assert_eq!(
            execute(&source, profile, subject_noun(&[7, 5, 1])).unwrap(),
            751
        );
        assert_eq!(
            execute(&edited, profile, subject_noun(&[7, 5, 1])).unwrap(),
            931
        );
    }
}

