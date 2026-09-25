use super::{nouns::data, signatures, support};
use data::Noun::Atom;

fn probe(name: &str) -> Vec<u8> {
    trident::compile_native_artifact_project(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("tests/fixtures/{name}.tri")),
        &trident::CompileOptions::default(),
        trident::NativeArtifactProfile::RawNoun,
        trident::NATIVE_ARTIFACT_LIMITS,
    )
    .unwrap()
    .bytes
}

fn seed(source: &str) -> Vec<u8> {
    data::seed(
        &(source.replacen("fn main()->Field", "fn result()->Field", 1)
            + " fn main(input:Noun)->Noun{nox_noun_atom(result())}"),
    )
}

fn agrees(body: &str) -> Vec<u8> {
    let source = format!("program sample {body}");
    let artifact = data::compile(&source);
    assert_eq!(support::run_artifact(&artifact), 7, "{source}");
    assert_eq!(
        data::run(&seed(&source), &Atom(0)).unwrap().bytes,
        Atom(7).encoded(),
        "seed {source}"
    );
    artifact
}

fn rejects(body: &str, code: u32, seed_rejects: bool) {
    let source = format!("program sample {body}");
    match support::compile_only(source.as_bytes(), data::caps()) {
        support::Compilation::Errors(errors) => {
            assert_eq!(errors[0].code, code, "{source}");
            assert!(errors[0].start <= errors[0].end && errors[0].end <= source.len() as u32);
        }
        other => panic!("{source}: {other:?}"),
    }
    if seed_rejects {
        assert!(
            trident::compile(&source, "oracle.tri").is_err(),
            "seed {source}"
        );
    }
}

#[test]
fn native_attributes_preserve_artifacts_and_contracts_remain_lexical_metadata() {
    support::worker(|| {
        let original = agrees("fn main()->Field{7}");
        for prefix in [
            "#[pure]",
            "#[pure] #[pure]",
            "#[requires()]",
            "#[ensures()]",
            "#[requires(false)] #[ensures(false)]",
            "#[requires(missing(x) +)]",
            "#[requires((foo(bar))) ] #[ensures(result == 9)]",
            "#[requires(Field XField module use _ > ^ /% [ { #)]",
            "#[pure] // unicode comment: λ )\n #[requires(// ignored )\n)] pub",
        ] {
            assert_eq!(agrees(&format!("{prefix} fn main()->Field{{7}}")), original);
        }
        assert_eq!(
            agrees("const A:Field=7 #[pure] #[ensures(result==A)] fn main()->Field{A}"),
            original
        );
        for nesting in [63, 64, 65, 96] {
            let body = format!(
                "#[requires({}x{})] fn main()->Field{{7}}",
                "(".repeat(nesting),
                ")".repeat(nesting)
            );
            assert_eq!(agrees(&body), original);
        }
        // Contracts and purity metadata consume no expression-node slot.
        let source = b"program sample #[pure] #[requires(((unknown)))] fn main()->Field{7}";
        let mut caps = data::caps();
        caps[2] = 1;
        caps[3] = 1;
        match support::compile_only(source, caps) {
            support::Compilation::Program { bytes, .. } => assert_eq!(bytes, original),
            other => panic!("{other:?}"),
        }
    });
}

#[test]
fn native_attributes_reject_malformed_delimiters_and_invalid_placement() {
    support::worker(|| {
        for (body, code) in [
            ("#[pure fn main()->Field{7}", 2),
            ("#[requires((x)] fn main()->Field{7}", 2),
            ("#[requires(x))] fn main()->Field{7}", 2),
            ("#[requires(x) fn main()->Field{7}", 2),
            ("#[pure]", 2),
            ("pub #[pure] fn main()->Field{7}", 6),
            ("fn main()->Field{#[pure] 7}", 6),
            ("fn f(#[pure] x:Field)->Field{x} fn main()->Field{7}", 6),
            ("const A:Field=#[pure] 7 fn main()->Field{A}", 6),
            ("#[pure] const A:Field=7 fn main()->Field{A}", 2),
            ("#[requires()] const A:Field=7 fn main()->Field{A}", 2),
            ("#[ensures()] pub struct R{a:Field} fn main()->Field{7}", 2),
            ("#[] fn main()->Field{7}", 2),
            ("#[pure()] fn main()->Field{7}", 6),
            ("#[requires] fn main()->Field{7}", 6),
            ("#[ensure()] fn main()->Field{7}", 6),
            ("#[requiresx()] fn main()->Field{7}", 6),
            ("#[requires(asm)] fn main()->Field{7}", 6),
            ("#[requires(@)] fn main()->Field{7}", 1),
            ("#[requires(18446744073709551616)] fn main()->Field{7}", 1),
            ("#[requires(λ)] fn main()->Field{7}", 1),
        ] {
            rejects(body, code, true);
        }
        for prefix in ["#[cfg(nox)]", "#[test]", "#[intrinsic(assert)]"] {
            rejects(&format!("{prefix} fn main()->Field{{7}}"), 6, false);
        }
    });
}

#[test]
fn native_purity_matches_every_seed_io_name_and_only_the_documented_prefixes() {
    support::worker(|| {
        let probe = probe("native_purity");
        for (name, expected) in [
            ("pub_read", true),
            ("pub_read_x", true),
            ("pub_write", true),
            ("pub_write_shadow", true),
            ("divine", true),
            ("divine_anything", true),
            ("sec_read", true),
            ("sponge_init", true),
            ("sponge_absorb", true),
            ("sponge_squeeze", true),
            ("sponge_absorb_mem", true),
            ("ram_read", true),
            ("ram_write", true),
            ("ram_read_block", true),
            ("ram_write_block", true),
            ("merkle_step", true),
            ("merkle_step_mem", true),
            ("pub_rea", false),
            ("pub_reae", false),
            ("pub_writ", false),
            ("pub_writf", false),
            ("divin", false),
            ("divinf", false),
            ("sec_read_x", false),
            ("ram_read_x", false),
            ("sponge_absorb_me", false),
            ("sponge_absorb_men", false),
            ("sponge_absorb_memx", false),
            ("merkle_step_men", false),
            ("merkle_step_memx", false),
            ("x", false),
        ] {
            let (arena, value) = signatures::run_probe(name, 4096, &probe);
            assert_eq!(
                arena.atom_value(value).unwrap().as_u64(),
                u64::from(expected),
                "{name}"
            );
            let source = format!(
                "program sample fn {name}()->Field{{7}} #[pure] fn main()->Field{{{name}()}}"
            );
            let checked = trident::compile(&source, "oracle.tri");
            assert_eq!(checked.is_err(), expected, "seed {name}");
        }
        let long = "pub_read".to_string() + &"a".repeat(256);
        let (arena, value) = signatures::run_probe(&long, 4096, &probe);
        assert_eq!(arena.atom_value(value).unwrap().as_u64(), 1);
    });
}

#[test]
fn native_purity_belongs_to_each_declaration_and_is_not_transitive() {
    support::worker(|| {
        agrees("#[pure] fn f()->Field{7} fn pub_write_shadow()->Field{7} fn main()->Field{pub_write_shadow()}");
        agrees("fn pub_write_shadow()->Field{7} fn helper()->Field{pub_write_shadow()} #[pure] fn main()->Field{helper()}");
        agrees("fn f()->Field{ram_read()} #[pure] fn f()->Field{7} fn ram_read()->Field{7} fn main()->Field{f()}");
        agrees("#[pure] fn f()->Field{7} fn f()->Field{ram_read()} fn ram_read()->Field{7} fn main()->Field{f()}");
        rejects("#[pure] fn f()->Field{ram_read()} fn f()->Field{7} fn ram_read()->Field{7} fn main()->Field{f()}",5,true);
        rejects("fn f()->Field{7} #[pure] fn f()->Field{ram_read()} fn ram_read()->Field{7} fn main()->Field{f()}",5,true);
        agrees("#[pure] fn main()->Field{let ram_read=7 ram_read}");
        agrees("fn ram_read_x()->Field{7} #[pure] fn main()->Field{ram_read_x()}");
        let funcs = (0..9)
            .map(|i| {
                format!(
                    "{} fn f{i}()->Field{{7}}",
                    if [0, 7, 8].contains(&i) {
                        "#[pure]"
                    } else {
                        ""
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        agrees(&format!("{funcs} fn main()->Field{{f8()}}"));
        // A pure declaration crossing a chunk boundary still owns its check.
        let bad = funcs.replacen("fn f8()->Field{7}", "fn f8()->Field{ram_read()}", 1);
        rejects(
            &format!("{bad} fn ram_read()->Field{{7}} fn main()->Field{{7}}"),
            5,
            true,
        );
    });
}

#[test]
fn native_purity_reaches_all_expression_contexts_and_unselected_bodies() {
    support::worker(|| {
        for body in [
            "ram_read()",
            "let x=ram_read() x",
            "let mut x=0 x=ram_read() x",
            "let(a,b)=(ram_read(),0) a",
            "let mut a=0 let mut b=0 (a,b)=(ram_read(),0) a",
            "let mut r=R{a:0} r.a=ram_read() r.a",
            "let r=R{a:ram_read()} r.a",
            "[7][ram_read()]",
            "if ram_read()==7{7}else{9}",
            "if false{return ram_read()} 7",
            "for i in 0..0{ram_read()} 7",
            "assert_eq(ram_read(),7) 7",
        ] {
            rejects(&format!("struct R{{a:Field}} fn ram_read()->Field{{7}} #[pure] fn main()->Field{{{body}}}"),5,true);
        }
    });
}

#[test]
fn native_purity_metadata_shares_the_exact_declaration_cap() {
    support::worker(|| {
        let source = b"program sample #[pure] fn a()->Field{7} #[pure] fn b()->Field{a()} #[pure] fn main()->Field{b()}";
        let expected = data::compile(std::str::from_utf8(source).unwrap());
        for cap in [2, 3, 4] {
            let mut caps = data::caps();
            caps[3] = cap;
            match support::compile_only(source, caps) {
                support::Compilation::Errors(errors) if cap == 2 => assert_eq!(errors[0].code, 7),
                support::Compilation::Program { bytes, .. } if cap >= 3 => {
                    assert_eq!(bytes, expected)
                }
                other => panic!("cap={cap}: {other:?}"),
            }
        }
    });
}
