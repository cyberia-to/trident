//! Source-level trees, full ART1 identity, and rejection at fixed-layout boundaries.
use nebu::Goldilocks;
use nox::{artifact, sequential, NoTrace, Order, Outcome, Reduction};
use trident::{ir::tree::lower::Noun, CompileOptions, RAW_ARTIFACT_LIMITS as LIMITS};

const ARENA: usize = 1 << 15;
fn a(v: u64) -> Noun {
    Noun::atom(v)
}
fn c(l: Noun, r: Noun) -> Noun {
    Noun::cell(l, r)
}
fn load(ar: &mut Reduction<ARENA>, value: &Noun) -> Order {
    match value {
        Noun::Atom(v) => ar.atom(Goldilocks::new(*v)).unwrap(),
        Noun::Cell(l, r) => {
            let l = load(ar, l);
            let r = load(ar, r);
            ar.pair(l, r).unwrap()
        }
    }
}
fn value(ar: &Reduction<ARENA>, root: Order) -> Noun {
    if let Some(v) = ar.atom_value(root) {
        a(v.as_u64())
    } else {
        c(
            value(ar, ar.head(root).unwrap()),
            value(ar, ar.tail(root).unwrap()),
        )
    }
}
fn execute(source: &str, input: Noun) -> Result<Noun, String> {
    let source = source.to_string();
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            let artifact = trident::compile_raw_artifact(
                &source,
                "noun.tri",
                &CompileOptions::default(),
                LIMITS,
            )
            .map_err(|e| format!("{e:?}"))?;
            execute_bytes(&artifact.bytes, input)
        })
        .unwrap()
        .join()
        .unwrap()
}
fn execute_bytes(bytes: &[u8], input: Noun) -> Result<Noun, String> {
    let mut ar = Reduction::<ARENA>::new();
    let root = artifact::decode(&mut ar, bytes, LIMITS).unwrap();
    assert_eq!(
        ar.atom_value(ar.head(root).unwrap()).unwrap().as_u64(),
        0x41525431
    );
    let mut cursor = ar.tail(root).unwrap();
    for _ in 0..3 {
        assert_eq!(ar.atom_value(ar.head(cursor).unwrap()).unwrap().as_u64(), 0);
        cursor = ar.tail(cursor).unwrap();
    }
    let formula = ar.head(cursor).unwrap();
    assert_eq!(ar.atom_value(ar.tail(cursor).unwrap()).unwrap().as_u64(), 0);
    let input = load(&mut ar, &input);
    let execution = sequential::reduce(
        &mut ar,
        input,
        formula,
        1_000_000,
        sequential::Limits { max_frames: 16384 },
        &mut NoTrace,
    )
    .unwrap();
    match execution.outcome {
        Outcome::Ok(output, _) => Ok(value(&ar, output)),
        other => Err(format!("{other:?}")),
    }
}
fn program(body: &str) -> String {
    format!("program noun_test\nfn main(input: Noun) -> Noun {{ {body} }}")
}
fn errors(source: &str) -> String {
    format!(
        "{:?}",
        trident::check_silent(source, "noun.tri").unwrap_err()
    )
}

#[test]
fn noun_roundtrips_arbitrary_trees_and_ordered_pairs() {
    let tree = c(c(a(17), a(0)), c(a(42), c(a(9), a(8))));
    assert_eq!(execute(&program("input"), tree.clone()).unwrap(), tree);
    assert_eq!(
        execute(
            &program("nox_noun_pair(nox_noun_tail(input), nox_noun_head(input))"),
            c(a(4), c(a(2), a(3)))
        )
        .unwrap(),
        c(c(a(2), a(3)), a(4))
    );
    assert_eq!(
        execute(
            &program("nox_noun_atom(nox_noun_as_field(input) + 1)"),
            a(13)
        )
        .unwrap(),
        a(14)
    );
    assert_eq!(
        execute(&program("nox_noun_atom(18446744069414584320)"), a(0)).unwrap(),
        a(18446744069414584320)
    );
}

#[test]
fn checked_projections_trap_on_wrong_shape() {
    for name in ["head", "tail"] {
        assert!(execute(&program(&format!("nox_noun_{name}(input)")), a(1)).is_err());
    }
    assert!(execute(
        &program("nox_noun_atom(nox_noun_as_field(input))"),
        c(a(1), a(2))
    )
    .is_err());
}

#[test]
fn full_identity_and_equality_preserve_native_digest_layout() {
    let src = program("let d = nox_noun_identity(input)\nlet (x, y, z, w) = d\nnox_noun_pair(nox_noun_pair(nox_noun_atom(x), nox_noun_atom(y)), nox_noun_pair(nox_noun_atom(z), nox_noun_atom(w)))");
    let input = c(a(12), c(a(5), a(9)));
    let expected = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn({
            let input = input.clone();
            move || {
                let mut ar = Reduction::<ARENA>::new();
                let root = load(&mut ar, &input);
                let digest = ar.digest(root).unwrap();
                c(
                    c(a(digest[0].as_u64()), a(digest[1].as_u64())),
                    c(a(digest[2].as_u64()), a(digest[3].as_u64())),
                )
            }
        })
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(execute(&src, input).unwrap(), expected);
    let eq = program("if nox_noun_eq(nox_noun_head(input), nox_noun_tail(input)) { nox_noun_atom(7) } else { nox_noun_atom(9) }");
    assert_eq!(execute(&eq, c(c(a(1), a(2)), c(a(1), a(2)))).unwrap(), a(7));
    assert_eq!(execute(&eq, c(c(a(1), a(2)), c(a(2), a(1)))).unwrap(), a(9));
}

#[test]
fn noun_containing_aggregates_keep_subtrees_through_mutation_and_calls() {
    let src = "program aggregate\nstruct Boxed { tree: Noun, n: Field }\nfn wrap(x: Noun) -> Boxed { Boxed { tree: x, n: 1 } }\nfn main(input: Noun) -> Noun {\nlet mut box = wrap(input)\nlet (left, right) = (nox_noun_head(box.tree), nox_noun_tail(box.tree))\nlet mut items: [Noun; 2] = [left, right]\nitems[0] = nox_noun_pair(items[1], items[0])\nbox.tree = items[0]\nbox.tree\n}";
    assert_eq!(
        execute(src, c(c(a(1), a(2)), a(3))).unwrap(),
        c(a(3), c(a(1), a(2)))
    );
}

#[test]
fn native_layout_is_never_a_field_word_width() {
    use trident::types::{StructTy, Ty};
    let structure = StructTy {
        module: "test".into(),
        name: "S".into(),
        fields: vec![
            ("tree".into(), Ty::Noun, true),
            ("n".into(), Ty::Field, false),
        ],
    };
    for ty in [
        Ty::Noun,
        Ty::Array(Box::new(Ty::Noun), 0),
        Ty::Tuple(vec![Ty::Field, Ty::Noun]),
        Ty::Struct(structure.clone()),
    ] {
        assert_eq!(ty.width(), None);
    }
    assert_eq!(structure.field("n"), Some((Ty::Field, false)));
    assert_eq!(structure.field_offset("n"), None);
    assert_eq!(Ty::Array(Box::new(Ty::Field), 3).width(), Some(3));
}

#[test]
fn implicit_noun_operations_constants_events_and_legacy_io_are_rejected() {
    for body in [
        "input + input",
        "input == input",
        "input < input",
        "input[0]",
        "let x: Field = input\ninput",
        "nox_noun_atom(input)",
    ] {
        assert!(!errors(&program(body)).is_empty(), "{body}");
    }
    for source in [
        "program p\nconst X: Noun = nox_noun_atom(0)\nfn main() {}",
        "program p\nevent E { tree: Noun }\nfn main() {}",
        "program p\npub input: Noun\nfn main() {}",
    ] {
        assert!(errors(source).contains("Noun"), "{source}");
    }
}

#[test]
fn flat_entry_rejects_noun_parameters_and_results_but_allows_internal_trees() {
    let options = CompileOptions::default();
    for source in [
        program("input"),
        "program p\nfn main() -> (Field, Noun) { (1, nox_noun_atom(2)) }".into(),
        "program p\nstruct S { v: Noun }\nfn main() -> S { S { v: nox_noun_atom(0) } }".into(),
    ] {
        let e = trident::compile_with_options(&source, "noun.tri", &options).unwrap_err();
        assert!(format!("{e:?}").contains("raw ART1"));
    }
    assert!(trident::compile_with_options(
        "program p\nfn main() -> Field { nox_noun_as_field(nox_noun_atom(2)) }",
        "noun.tri",
        &options
    )
    .is_ok());
}

#[test]
fn raw_entry_requires_exact_signature_and_excludes_host_services() {
    for source in [
        "program p\nfn main(x: Field) -> Field { x }",
        "program p\nfn main(input: Noun) -> Field { 0 }",
        "program p\nfn main(input: Noun) -> Noun { nox_noun_atom(divine()) }",
        "program p\nfn main(input: Noun) -> Noun { nox_noun_atom(os.state.read(0)) }",
    ] {
        assert!(trident::compile_raw_artifact(
            source,
            "noun.tri",
            &CompileOptions::default(),
            LIMITS
        )
        .is_err());
    }
    // Data containing service tag 16 is ordinary data, not a host call.
    assert_eq!(
        execute(&program("nox_noun_pair(nox_noun_atom(16), input)"), a(0)).unwrap(),
        c(a(16), a(0))
    );
}

#[test]
fn canonical_artifact_identity_and_emission_limits_are_enforced() {
    let source = program("nox_noun_pair(input, input)");
    let compile = |limits| {
        trident::compile_raw_artifact(&source, "noun.tri", &CompileOptions::default(), limits)
    };
    let x = compile(LIMITS).unwrap();
    let y = compile(LIMITS).unwrap();
    assert_eq!(x.bytes, y.bytes);
    assert_eq!(x.particle, y.particle);
    assert_eq!(&x.bytes[8..40], &x.particle);
    for limits in [
        artifact::Limits {
            max_bytes: 44,
            ..LIMITS
        },
        artifact::Limits {
            max_nodes: 1,
            ..LIMITS
        },
        artifact::Limits {
            max_depth: 1,
            ..LIMITS
        },
        artifact::Limits {
            max_nodes: 0,
            ..LIMITS
        },
    ] {
        assert!(compile(limits).is_err());
    }
}

#[test]
fn shared_tir_rejects_native_surface_even_in_deferred_generic_bodies() {
    for source in [
        program("input"),
        "program p\nfn main() { let x = nox_noun_atom(1) }".into(),
        "program p\nfn unused<N>() { let x = nox_noun_atom(1) }\nfn main() {}".into(),
        "program p\nfn unused<N>(x: [Noun; N]) {}\nfn main() {}".into(),
    ] {
        let file = trident::parse_source_silent(&source, "noun.tri").unwrap();
        assert!(
            trident::tir::builder::TIRBuilder::new(trident::target::TerrainConfig::nox())
                .build_file(&file)
                .is_err()
        );
        assert!(trident::build_tir(&source, "noun.tri", &CompileOptions::default()).is_err());
    }
    for source in ["program p\nfn nox_noun_atom(x: Field) -> Field { x }\nfn main() -> Field { nox_noun_atom(1) }", "program p\nfn nox_noun_atom() {}\nfn main() { nox_noun_atom() }", "program p\n#[cfg(future)]\nfn unused(x: Noun) -> Noun { x }\nfn main() {}"] {
        assert!(trident::build_tir(source, "noun.tri", &CompileOptions::default()).is_ok(), "{source}");
    }
}

#[test]
fn foreign_target_rejects_noun_in_unused_generics() {
    let mut options = CompileOptions::default();
    options.target_config = trident::target::TerrainConfig::parse_toml(
        include_str!("fixtures/stack-target.toml"),
        std::path::Path::new("fixture"),
    )
    .unwrap();
    let source = "program p\nfn unused<N>(x: [Noun; N]) {}\nfn main() {}";
    let e = trident::build_tir(source, "noun.tri", &options).unwrap_err();
    assert!(format!("{e:?}").contains("native nox target"));
}

#[test]
fn digest_tuple_assignment_and_direct_limb_access_agree() {
    let src = program("let mut x: Field = 0\nlet mut y: Field = 0\nlet mut z: Field = 0\nlet mut w: Field = 0\n(x, y, z, w) = nox_noun_identity(input)\nlet digest = nox_noun_identity(input)\nassert_eq(x, digest[0])\nassert_eq(y, digest[1])\nassert_eq(z, digest[2])\nassert_eq(w, digest[3])\nnox_noun_atom(nox_noun_identity(input)[2])");
    assert!(execute(&src, c(a(99), a(2))).is_ok());
}

#[test]
fn noun_is_reserved_formatted_and_hashed_as_a_distinct_primitive() {
    let source = program("input");
    let file = trident::parse_source_silent(&source, "noun.tri").unwrap();
    let scalar =
        trident::parse_source_silent(&source.replace("Noun", "Field"), "noun.tri").unwrap();
    assert_ne!(
        trident::hash::hash_file_content(&file),
        trident::hash::hash_file_content(&scalar)
    );
    let formatted = trident::ast::display::format_function(match &file.items[0].node {
        trident::ast::Item::Fn(f) => f,
        _ => unreachable!(),
    });
    assert!(formatted.contains("input: Noun") && formatted.contains("-> Noun"));
    assert!(errors("program p\nstruct Noun { x: Field }\nfn main() {}").contains("expected"));
}

#[test]
fn generic_imports_preserve_noun_structs_and_all_project_tir_paths_reject_them() {
    let directory = tempfile::tempdir().unwrap();
    let entry = directory.path().join("main.tri");
    std::fs::write(directory.path().join("helper.tri"), "module helper\npub struct Boxed { pub tree: Noun }\npub fn first<N>(values: [Noun; N]) -> Boxed { Boxed { tree: values[0] } }").unwrap();
    std::fs::write(&entry, "program p\nuse helper\nfn main(input: Noun) -> Noun { let wrapped = helper.first<2>([input, input])\nwrapped.tree }").unwrap();
    let options = CompileOptions::default();
    let artifact = trident::compile_raw_artifact_project(&entry, &options, LIMITS).unwrap();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            assert_eq!(
                execute_bytes(&artifact.bytes, c(a(5), a(6))).unwrap(),
                c(a(5), a(6))
            );
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(trident::build_tir_modules(&entry, &options).is_err());
    assert!(trident::build_tir_project(&entry, &options).is_err());
}

#[test]
fn noun_pair_evaluates_arguments_once_in_source_order() {
    use std::sync::atomic::{AtomicU64, Ordering};
    struct Inputs(AtomicU64);
    impl nox::LookProvider for Inputs {
        fn look(&self, _: Goldilocks, _: Goldilocks, _: Goldilocks) -> Option<Goldilocks> {
            None
        }
    }
    impl nox::CallProvider<ARENA> for Inputs {
        fn provide(&self, ar: &mut Reduction<ARENA>, _: Goldilocks, _: Order) -> Option<Order> {
            let next = self.0.fetch_add(1, Ordering::SeqCst) + 1;
            ar.atom(Goldilocks::new(next))
        }
    }
    let source = "program p\nfn main() -> Field { let tree = nox_noun_pair(nox_noun_atom(divine()), nox_noun_atom(divine()))\nnox_noun_as_field(nox_noun_head(tree)) * 100 + nox_noun_as_field(nox_noun_tail(tree)) }";
    trident::check_silent(source, "p.tri").unwrap();
    let file = trident::parse_source_silent(source, "p.tri").unwrap();
    let formula = trident::ir::tree::lower::nox::NoxCompiler::new()
        .compile_file(&file)
        .unwrap();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let mut ar = Reduction::<ARENA>::new();
            let formula = load(&mut ar, &formula);
            let zero = load(&mut ar, &a(0));
            let inputs = Inputs(AtomicU64::new(0));
            let result = nox::reduce(&mut ar, zero, formula, 10_000, &inputs, &mut NoTrace);
            let Outcome::Ok(output, _) = result else {
                panic!("{result:?}");
            };
            assert_eq!(ar.atom_value(output).unwrap().as_u64(), 102);
            assert_eq!(inputs.0.load(Ordering::SeqCst), 2);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn noun_cannot_be_consumed_implicitly_as_an_index_or_loop_endpoint() {
    for body in [
        "for i in 0..input bounded 2 {} input",
        "for i in input..2 {} input",
        "let values = [1, 2] let x = values[input] input",
        "let mut values = [1, 2] values[input] = 3 input",
    ] {
        assert!(errors(&program(body)).contains("requires Field or U32"));
    }
}

#[test]
fn field_literals_and_constants_are_canonicalized_before_artifact_emission() {
    for (spelling, expected) in [
        ("18446744069414584321", 0),
        ("18446744073709551615", 4294967294),
    ] {
        assert_eq!(
            execute(&program(&format!("nox_noun_atom({spelling})")), a(0)).unwrap(),
            a(expected)
        );
        let source = format!("program p\nconst MAX: Field = {spelling}\nfn main(input: Noun) -> Noun {{ nox_noun_atom(MAX) }}");
        assert_eq!(execute(&source, a(0)).unwrap(), a(expected));
    }
}
