use super::{codegen, support};
#[path = "nouns/support.rs"]
pub(super) mod data;
use data::{Noun, Noun::Atom};

#[test]
fn structured_entry_preserves_whole_input_in_singleton_table_and_loop_frames() {
    support::worker(|| {
        let input = data::nested();
        for source in [
            data::source("input"),
            "program sample fn main()->Field{7} fn main(input:Noun)->Noun{input}".into(),
            data::source("let x=input let y=x let z=y let q=z q"),
            data::source("let input=input input"),
            data::source("if true{return input} input"),
            "program sample fn id(x:Noun)->Noun{x} fn main(input:Noun)->Noun{id(input)}".into(),
            "program sample fn id(a:Field,x:Noun,b:Bool)->Noun{if b{return x} nox_noun_atom(a)} fn main(input:Noun)->Noun{id(9,input,true)}".into(),
            data::source("let mut x=input for i in 0..3{x=nox_noun_head(nox_noun_pair(x,nox_noun_atom(as_field(i))))} x"),
            data::source("for i in 0..2{for j in 0..3{return input}} input"),
        ] {
            data::agrees(&source, &input, &input);
            data::agrees(&source, &Atom(934), &Atom(934));
        }
    });
}

#[test]
fn noun_constructors_projections_and_equality_preserve_nested_values() {
    support::worker(|| {
        let left = data::nested();
        let right = Noun::pair(Atom(5), Atom(9));
        let input = Noun::pair(left.clone(), right.clone());
        for (body, expected) in [
            ("nox_noun_atom(18446744069414584320)", Atom(18446744069414584320)),
            ("nox_noun_pair(nox_noun_tail(input),nox_noun_head(input))", Noun::pair(right, left)),
            ("let x=nox_noun_head(input) nox_noun_pair(x,input)", Noun::pair(data::nested(),input.clone())),
            ("if nox_noun_eq(input,input){input}else{nox_noun_atom(9)}",input.clone()),
            ("if nox_noun_eq(nox_noun_atom(7),nox_noun_atom(8)){input}else{nox_noun_atom(19)}",Atom(19)),
            ("if nox_noun_eq(nox_noun_head(nox_noun_head(input)),nox_noun_tail(nox_noun_head(input))){input}else{nox_noun_atom(0)}",input.clone()),
            ("let mut x=input for i in 0..3{x=nox_noun_pair(nox_noun_atom(as_field(i)),x)} x",Noun::pair(Atom(2),Noun::pair(Atom(1),Noun::pair(Atom(0),input.clone())))),
            ("nox_noun_atom(nox_noun_as_field(nox_noun_atom(9))+3)",Atom(12)),
        ] {
            data::agrees(&data::source(body), &input, &expected);
        }
        let scalar = "program sample fn f(x:Noun)->Noun{nox_noun_tail(x)} fn main()->Field{let x=nox_noun_pair(nox_noun_atom(7),nox_noun_atom(9)) nox_noun_as_field(f(x))}";
        assert_eq!(support::run_artifact(&data::compile(scalar)), 9);
        assert_eq!(support::rust_value(scalar), 9);
        let shadowed = "program sample fn main(input:Noun)->Noun{input} fn main()->Field{7}";
        assert_eq!(support::run_artifact(&data::compile(shadowed)), 7);
        assert_eq!(support::rust_value(shadowed), 7);
    });
}

#[test]
fn noun_callable_bindings_and_table_order_match_seed_resolution() {
    support::worker(|| {
        let input = data::nested();
        for body in [
            "fn nox_noun_head(x:Noun)->Noun{x} fn main(input:Noun)->Noun{nox_noun_head(input)}",
            "fn nox_noun_atom(x:Bool)->Field{7} fn nox_noun_atom(x:Noun)->Noun{x} fn main(input:Noun)->Noun{nox_noun_atom(input)}",
            "fn nox_noun_eq(x:Noun,y:Noun)->Noun{y} fn main(input:Noun)->Noun{nox_noun_eq(input,input)}",
            "fn main(input:Noun)->Noun{let nox_noun_pair=input nox_noun_head(nox_noun_pair(nox_noun_pair,input))}",
            "fn f(x:Noun,x:Noun)->Noun{x} fn main(input:Noun)->Noun{f(nox_noun_atom(0),input)}",
        ] {
            data::agrees(&format!("program sample {body}"), &input, &input);
        }
        let f = "fn f(x:Noun)->Noun{x}";
        let g = "fn g(x:Noun)->Noun{nox_noun_head(nox_noun_pair(f(x),nox_noun_atom(9)))}";
        let main = "fn main(input:Noun)->Noun{g(input)}";
        let mut expected = None;
        for body in [
            format!("{f} {g} {main}"),
            format!("{main} {g} {f}"),
            format!("fn unused(x:Noun)->Noun{{nox_noun_tail(x)}} {g} {main} {f}"),
        ] {
            let bytes = data::agrees(&format!("program sample {body}"), &input, &input);
            if let Some(ref expected) = expected {
                assert_eq!(&bytes, expected);
            }
            expected = Some(bytes);
        }
    });
}

#[test]
fn noun_types_and_entry_signatures_reject_before_program_publication() {
    support::worker(|| {
        for (body, code) in [
            ("fn main(input:Noun)->Field{0}", 3),
            ("fn main(input:Field)->Noun{nox_noun_atom(input)}", 3),
            ("fn main()->Noun{nox_noun_atom(0)}", 3),
            ("fn main(a:Noun,b:Noun)->Noun{a}", 3),
            ("fn main(input:Noun)->Noun{7}", 5),
            ("fn main(input:Noun)->Noun{if input{return input} input}", 5),
            ("fn main(input:Noun)->Noun{let x:Field=input input}", 5),
            ("fn main(input:Noun)->Noun{let x:Noun=7 input}", 5),
            ("fn main(input:Noun)->Noun{input=nox_noun_atom(7) input}", 5),
            ("fn main(input:Noun)->Noun{let mut x=input x=7 input}", 5),
            ("fn main(input:Noun)->Noun{input==input input}", 5),
            ("fn main(input:Noun)->Noun{input+input}", 5),
            ("fn main(input:Noun)->Noun{input*input}", 5),
            ("fn main(input:Noun)->Noun{input<input input}", 5),
            ("fn main(input:Noun)->Noun{input&input}", 5),
            ("fn main(input:Noun)->Noun{input==7 input}", 5),
            ("fn main(input:Noun)->Noun{nox_noun_atom(input)}", 5),
            ("fn main(input:Noun)->Noun{nox_noun_head(7)}", 5),
            ("fn main(input:Noun)->Noun{nox_noun_pair(input,7)}", 5),
            ("fn main(input:Noun)->Noun{nox_noun_tail(input,input)}", 5),
            ("fn main(input:Noun)->Noun{nox_noun_eq(input) input}", 5),
            (
                "fn main(input:Noun)->Noun{nox_noun_as_fielx(input) input}",
                5,
            ),
            ("fn main(input:Noun)->Noun{nox_noun_atoms(7)}", 5),
            (
                "fn main(input:Noun)->Noun{let noun=input noun.head(input)}",
                6,
            ),
            ("fn main(input:Noun)->Noun{input[0]}", 6),
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

#[test]
fn invalid_noun_projection_compiles_then_traps_only_when_reached() {
    support::worker(|| {
        for body in [
            "nox_noun_head(nox_noun_atom(7))",
            "nox_noun_tail(nox_noun_atom(7))",
            "nox_noun_atom(nox_noun_as_field(nox_noun_pair(input,input)))",
            "nox_noun_head(nox_noun_atom(7)) input",
            "let x=nox_noun_tail(nox_noun_atom(7)) input",
        ] {
            let source = data::source(body);
            let actual = data::run(&data::compile(&source), &data::nested());
            assert!(actual.is_err(), "{source}");
            assert_eq!(
                actual,
                data::run(&data::seed(&source), &data::nested()),
                "{source}"
            );
        }
        let source="program sample fn f(x:Noun,y:Noun)->Noun{y} fn main(input:Noun)->Noun{f(nox_noun_head(nox_noun_atom(7)),input)}";
        assert!(data::run(&data::compile(source), &data::nested()).is_err());
        data::agrees(
            &data::source("if false{return nox_noun_head(nox_noun_atom(7))} input"),
            &data::nested(),
            &data::nested(),
        );
    });
}

#[test]
fn noun_builtin_arguments_execute_once_in_source_order() {
    support::worker(|| {
        let source=data::source("nox_noun_pair(nox_noun_atom(sub(9,2)),nox_noun_head(nox_noun_pair(nox_noun_atom(sub(8,3)),input)))");
        let mut trace = nox::trace::VecTrace::default();
        let bytes = data::compile(&source);
        let run = data::run_traced(
            &bytes,
            &data::nested(),
            1_000_000,
            65536,
            196608,
            &mut trace,
        )
        .unwrap();
        assert_eq!(run.bytes, Noun::pair(Atom(7), Atom(5)).encoded());
        assert_eq!(trace.0.iter().filter(|row| row.col(0) == 6).count(), 2);
        // Distinct traps establish left-before-right even when neither argument
        // contributes to a successful result.
        for expression in [
            "nox_noun_pair(nox_noun_atom(as_field(as_u32(4294967296))),nox_noun_head(nox_noun_atom(0)))",
            "nox_noun_eq(nox_noun_atom(as_field(as_u32(4294967296))),nox_noun_head(nox_noun_atom(0))) input",
        ] {
            let source=data::source(expression);
            let actual=data::run(&data::compile(&source),&Atom(0));
            assert_eq!(actual,Err("Error(InvZero)".into()));
            assert_eq!(actual,data::run(&data::seed(&source),&Atom(0)));
        }
        let exact = data::run(&bytes, &data::nested()).unwrap();
        assert_eq!(
            data::run_traced(
                &bytes,
                &data::nested(),
                exact.reductions,
                exact.frames,
                exact.nodes,
                &mut nox::NoTrace
            )
            .unwrap(),
            exact
        );
        assert!(data::run_traced(
            &bytes,
            &data::nested(),
            exact.reductions - 1,
            exact.frames,
            exact.nodes,
            &mut nox::NoTrace
        )
        .is_err());
        assert_eq!(
            data::run_traced(
                &bytes,
                &data::nested(),
                exact.reductions,
                exact.frames - 1,
                exact.nodes,
                &mut nox::NoTrace
            ),
            Err("Frames".into())
        );
        assert!(data::run_traced(
            &bytes,
            &data::nested(),
            exact.reductions,
            exact.frames,
            exact.nodes - 1,
            &mut nox::NoTrace
        )
        .is_err());
    });
}

#[test]
fn structured_initializer_depth_counts_input_padding_and_quoted_tables() {
    support::worker(|| {
        let probe = trident::compile_native_artifact_project(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/native_noun_depth.tri"),
            &trident::CompileOptions::default(),
            trident::NativeArtifactProfile::RawNoun,
            trident::NATIVE_ARTIFACT_LIMITS,
        )
        .unwrap();
        for slots in [1, 2, 3, 5] {
            let bindings = (1..slots)
                .map(|i| format!("let x{i}=input "))
                .collect::<String>();
            for table in [false, true] {
                let mut source = data::source(&format!(
                    "{bindings}{}",
                    if table { "f(input)" } else { "input" }
                ));
                if table {
                    source.push_str(" fn f(x:Noun)->Noun{nox_noun_head(nox_noun_pair(x,nox_noun_pair(x,nox_noun_pair(x,x))))}");
                }
                let generate = |cap| {
                    codegen::generate_with_input::<{ 1 << 20 }>(&probe.bytes, |arena| {
                        let source = support::schema::bytes(arena, source.as_bytes()).unwrap();
                        let cap = support::data::atom(arena, cap).unwrap();
                        support::data::pair(arena, source, cap).unwrap()
                    })
                };
                let bytes = generate(4096).unwrap();
                let exact = codegen::artifact_depth(&bytes) + 4;
                assert_eq!(generate(exact - 1), Err(7), "slots={slots}, table={table}");
                assert_eq!(generate(exact).unwrap(), bytes);
                assert_eq!(generate(exact + 1).unwrap(), bytes);
                assert_eq!(
                    data::run(&bytes, &data::nested()).unwrap().bytes,
                    data::nested().encoded()
                );
            }
        }
    });
}

#[test]
fn noun_parameter_and_local_slots_obey_exact_sequence_caps() {
    support::worker(|| {
        for (source,exact) in [
            (data::source("let x=input x"),2),
            ("program sample fn f(x:Noun)->Noun{x} fn g(x:Noun)->Noun{f(x)} fn main(input:Noun)->Noun{g(input)}".into(),3),
        ] {
            let mut expected=None;
            for cap in [exact-1,exact,exact+1] {
                let mut caps=data::caps();caps[2]=1;caps[3]=cap;
                match support::compile_only(source.as_bytes(),caps) {
                    support::Compilation::Errors(errors) if cap<exact=>assert_eq!(errors[0].code,7),
                    support::Compilation::Program{bytes,..} if cap>=exact=>{
                        assert_eq!(data::run(&bytes,&data::nested()).unwrap().bytes,data::nested().encoded());
                        if let Some(ref expected)=expected{assert_eq!(&bytes,expected);}
                        expected=Some(bytes);
                    }
                    other=>panic!("cap={cap}: {source}: {other:?}"),
                }
            }
        }
    });
}
