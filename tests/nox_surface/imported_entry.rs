//! Imported array dimensions belong to their defining module.
use super::*;

#[test]
fn imported_entry_sizes_keep_lexical_constants_and_generic_calls() {
    let dir = std::env::temp_dir().join(format!("nox-imported-entry-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("shapes.tri"), "module shapes\nconst N: U32 = 2\npub struct Pair { pub words: [Field; N] }\npub struct Nested { pub pairs: [Pair; N+1] }\npub fn echo(x:Field)->Field {x}\n").unwrap();
    let source = "program entry\nuse shapes\nconst N:U32=17\nfn first<M>(x:[Field;M])->Field {x[0]}\nfn main(x:shapes.Nested)->Field { assert(x.pairs[0].words[0]==7)\n assert(x.pairs[0].words[1]==19)\n assert(x.pairs[1].words[0]==23)\n assert(x.pairs[1].words[1]==29)\n assert(x.pairs[2].words[0]==31)\n assert(x.pairs[2].words[1]==37)\n shapes.echo(first<3>([101,103,107]))+x.pairs[2].words[1] }";
    let path = dir.join("entry.tri");
    std::fs::write(&path, source).unwrap();
    for profile in ["debug", "release"] {
        let bundle =
            trident::compile_to_bundle(&path, &CompileOptions::for_profile(profile)).unwrap();
        let mut arena = Reduction::<8192>::new();
        let f = load(&mut arena, &parse(&bundle.assembly));
        let s = load(&mut arena, &subject_noun(&[7, 19, 23, 29, 31, 37]));
        match reduce(&mut arena, s, f, 100_000, &NullCalls, &mut NoTrace) {
            Outcome::Ok(r, _) => assert_eq!(arena.atom_value(r).unwrap().as_u64(), 138),
            other => panic!("imported entry failed: {other:?}"),
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn legacy_local_struct_root_shadows_module_constant_value_and_type() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("constants.tri"),
        "module constants\npub const FLAG:Field=0",
    )
    .unwrap();
    let path = dir.path().join("entry.tri");
    for (field, init, body) in [
        ("Field", "n", "if constants.FLAG {7} else {11}"),
        ("Bool", "n==0", "assert(constants.FLAG) 7"),
    ] {
        let source=format!("program shadow\nuse constants\nstruct Dynamic {{FLAG:{field}}}\nfn main(n:Field)->Field {{let constants=Dynamic{{FLAG:{init}}} {body}}}");
        std::fs::write(&path, source).unwrap();
        for profile in ["debug", "release"] {
            let bundle =
                trident::compile_to_bundle(&path, &CompileOptions::for_profile(profile)).unwrap();
            for input in [0, 1] {
                let mut arena = Reduction::<8192>::new();
                let f = load(&mut arena, &parse(&bundle.assembly));
                let s = load(&mut arena, &subject_noun(&[input]));
                match reduce(&mut arena, s, f, 100_000, &NullCalls, &mut NoTrace) {
                    Outcome::Ok(r, _) => {
                        assert!(field == "Field" || input == 0);
                        assert_eq!(
                            arena.atom_value(r).unwrap().as_u64(),
                            if input == 0 { 7 } else { 11 }
                        );
                    }
                    other => assert!(field == "Bool" && input == 1, "{other:?}"),
                }
            }
        }
    }
}
