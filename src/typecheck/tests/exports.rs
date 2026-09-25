use super::{check, check_with_flags};
use crate::types::Ty;

#[test]
fn final_callable_declaration_owns_export_visibility_and_resolved_signature() {
    for first in ["pub fn f()->Field{7}", "fn f()->Field{7}"] {
        let hidden = check(&format!("module values\n{first}\nfn f(x:Bool)->Bool{{x}}")).unwrap();
        assert!(hidden.functions.is_empty());
        let visible = check(&format!(
            "module values\n{first}\npub fn f(x:Bool)->Bool{{x}}"
        ))
        .unwrap();
        assert_eq!(visible.functions.len(), 1);
        assert_eq!(
            visible.functions[0],
            ("f".into(), vec![("x".into(), Ty::Bool)], Ty::Bool)
        );
    }
}

#[test]
fn final_callable_kind_cannot_leave_an_ordinary_or_generic_export_behind() {
    let ordinary = "fn f(x:Field)->Field{x}";
    let generic = "fn f<N>(x:[Field;N])->Field{x[0]}";
    for (first, last, is_generic) in [
        (ordinary, generic, true),
        (generic, ordinary, false),
        (generic, generic, true),
    ] {
        for public in [false, true] {
            let prefix = if public { "pub " } else { "" };
            let exports = check(&format!("module values\npub {first}\n{prefix}{last}")).unwrap();
            assert_eq!(exports.functions.len(), usize::from(public && !is_generic));
            assert_eq!(
                exports.generic_functions.len(),
                usize::from(public && is_generic)
            );
            assert!(exports.direct_intrinsics.is_empty());
        }
    }
}

#[test]
fn intrinsic_exports_follow_final_identity_and_visibility_together() {
    let intrinsic = "#[intrinsic(assert)] pub fn stop(c:Bool)";
    for (last, public) in [
        ("fn stop(c:Bool)->Field{7}", false),
        ("pub fn stop(c:Bool)->Field{7}", true),
    ] {
        let exports = check(&format!("module std.failure\n{intrinsic}\n{last}")).unwrap();
        assert_eq!(exports.functions.len(), usize::from(public));
        assert!(exports.direct_intrinsics.is_empty());
        if public {
            assert_eq!(exports.functions[0].2, Ty::Field);
        }
    }
    let exports = check(&format!(
        "module std.failure\npub fn stop(c:Bool)->Field{{7}}\n{intrinsic}"
    ))
    .unwrap();
    assert_eq!(exports.functions.len(), 1);
    assert_eq!(exports.functions[0].2, Ty::Unit);
    assert_eq!(
        exports.direct_intrinsics.get("stop").map(String::as_str),
        Some("assert")
    );
}

#[test]
fn disabled_final_declarations_preserve_the_last_active_export() {
    let source = "module values\npub fn f()->Field{7}\n#[cfg(release)] fn f()->Bool{true}\n#[cfg(debug)] pub fn f(x:Bool)->Bool{x}";
    let neutral = check_with_flags(source, &[]).unwrap();
    assert_eq!(neutral.functions.len(), 1);
    assert_eq!(neutral.functions[0].2, Ty::Field);
    assert!(check_with_flags(source, &["release"])
        .unwrap()
        .functions
        .is_empty());
    let debug = check_with_flags(source, &["debug"]).unwrap();
    assert_eq!(debug.functions.len(), 1);
    assert_eq!(debug.functions[0].1, vec![("x".into(), Ty::Bool)]);
}

#[test]
fn recursion_uses_the_final_body_and_its_source_span() {
    let exports = check(
        "module std.failure\nfn stop(c:Bool){stop(c)}\n#[intrinsic(assert)] pub fn stop(c:Bool)",
    )
    .unwrap();
    assert_eq!(
        exports.direct_intrinsics.get("stop").map(String::as_str),
        Some("assert")
    );
    assert!(check("module values\npub fn f()->Field{f()}\npub fn f()->Field{7}").is_ok());
    let source = "module values\nfn f()->Field{7}\npub fn f()->Field{f()}";
    let errors = check(source).unwrap_err();
    let error = errors
        .iter()
        .find(|e| e.message.contains("recursive call cycle"))
        .unwrap();
    assert_eq!(
        error.span.start as usize,
        source.find("pub fn f").unwrap() + 7
    );
}

#[test]
fn direct_tir_generic_sites_match_the_final_emitted_bodies() {
    use crate::{ir::tir::builder::TIRBuilder, lexer::Lexer, parser::Parser, tir::TIROp};
    for replaced in [
        "fn old()->Field{id([1])} fn old()->Field{7}",
        "#[test] fn earlier(){let x=id([1])}",
    ] {
        let source = format!("program entry fn id<N>(x:[Field;N])->Field{{x[0]}} {replaced} fn main()->Field{{id([7,9])}}");
        let (tokens, _, _) = Lexer::new(&source, 0).tokenize();
        let file = Parser::new(tokens).parse_file().unwrap();
        let checked = check(&source).unwrap();
        assert_eq!(
            checked.call_resolutions.len(),
            2,
            "all source calls still require validation"
        );
        let resolution = checked
            .call_resolutions
            .iter()
            .find(|((function, _, _), _)| function == "main")
            .unwrap()
            .1;
        assert_eq!(resolution.size_args, vec![2]);
        let expected = resolution.mangled_name();
        let ir = TIRBuilder::new(crate::target::TerrainConfig::triton())
            .with_mono_instances(checked.mono_instances)
            .with_call_resolutions(checked.call_resolutions)
            .build_file(&file)
            .unwrap();
        let mut in_main = false;
        let calls: Vec<_> = ir
            .iter()
            .filter_map(|op| match op {
                TIROp::FnStart(name) => {
                    in_main = name == "main";
                    None
                }
                TIROp::FnEnd => {
                    in_main = false;
                    None
                }
                TIROp::Call(name) if in_main => Some(name.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(calls, vec![expected]);
    }
}

#[test]
fn explicit_generic_calls_do_not_shift_later_inferred_call_resolutions() {
    use crate::{ir::tir::builder::TIRBuilder, lexer::Lexer, parser::Parser, tir::TIROp};
    let source = "program entry fn id<N>(x:[Field;N])->Field{x[0]} fn old()->Field{id([1])} fn old()->Field{7} fn main()->Field{let x=id<1>([1]) id([7,9])}";
    let (tokens, _, _) = Lexer::new(source, 0).tokenize();
    let file = Parser::new(tokens).parse_file().unwrap();
    let checked = check(source).unwrap();
    assert_eq!(
        checked
            .call_resolutions
            .iter()
            .filter(|((f, _, _), _)| f == "main")
            .map(|(_, r)| r.size_args.clone())
            .collect::<Vec<_>>(),
        vec![vec![1], vec![2]]
    );
    let expected: Vec<_> = checked
        .call_resolutions
        .iter()
        .filter(|((f, _, _), _)| f == "main")
        .map(|(_, r)| r.mangled_name())
        .collect();
    let ir = TIRBuilder::new(crate::target::TerrainConfig::triton())
        .with_mono_instances(checked.mono_instances)
        .with_call_resolutions(checked.call_resolutions)
        .build_file(&file)
        .unwrap();
    let mut in_main = false;
    let calls: Vec<_> = ir
        .iter()
        .filter_map(|op| match op {
            TIROp::FnStart(name) => {
                in_main = name == "main";
                None
            }
            TIROp::FnEnd => {
                in_main = false;
                None
            }
            TIROp::Call(name) if in_main => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(calls, expected);
}

pub(super) fn direct_ir(
    source: &str,
) -> Result<Vec<crate::tir::TIROp>, Vec<crate::diagnostic::Diagnostic>> {
    let file = crate::parse_source(source, "calls.tri").unwrap();
    let checked = check(source).unwrap();
    crate::ir::tir::builder::TIRBuilder::new(crate::target::TerrainConfig::triton())
        .with_mono_instances(checked.mono_instances)
        .with_call_resolutions(checked.call_resolutions)
        .build_file(&file)
}

fn direct_calls(ops: &[crate::tir::TIROp], function: &str) -> Vec<String> {
    use crate::tir::TIROp;
    let mut inside = false;
    ops.iter()
        .filter_map(|op| match op {
            TIROp::FnStart(name) => {
                inside = name == function;
                None
            }
            TIROp::FnEnd => {
                inside = false;
                None
            }
            TIROp::Call(name) if inside => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn first_instance(size: u64) -> String {
    crate::typecheck::MonoInstance {
        name: "first".into(),
        size_args: vec![size],
    }
    .mangled_name()
}

#[test]
fn generic_bindings_survive_reordered_comparison_operands_and_constants() {
    for expression in [
        "first([3]) < first<2>([7,9])",
        "first<1>([3]) < first([7,9])",
        "first([3]) < first([7,9])",
        "first<1>([3]) < first<SIZE>([7,9])",
        "(first([3])) < (first<SIZE>([7,9]))",
    ] {
        let source = format!("program calls const SIZE:Field=2 fn first<N>(x:[Field;N])->U32{{as_u32(x[0])}} fn main()->Bool{{{expression}}}");
        let ir = direct_ir(&source).unwrap();
        assert_eq!(
            direct_calls(&ir, "main"),
            vec![first_instance(2), first_instance(1)],
            "{expression}"
        );
    }
}

#[test]
fn generic_bindings_preserve_constructor_nested_call_and_pass_through_sites() {
    for (functions, name, sizes) in [
        ("struct Pair{a:Field,b:Field} fn main()->Field{let p=Pair{b:first<2>([7,9]),a:first([3])} p.a*100+p.b}", "main", vec![1,2]),
        ("fn main()->Field{first([first([3]),9])}", "main", vec![1,2]),
        ("fn pass(x:[Field;2])->Field{first(x)} fn main()->Field{pass([7,9])}", "pass", vec![2]),
    ] {
        let source = format!("program calls fn first<N>(x:[Field;N])->Field{{x[0]}} {functions}");
        let ir = direct_ir(&source).unwrap();
        assert_eq!(direct_calls(&ir, name), sizes.into_iter().map(first_instance).collect::<Vec<_>>());
    }
}

#[test]
fn direct_generic_emission_rejects_missing_or_unemitted_site_bindings() {
    use crate::ir::tir::builder::TIRBuilder;
    let source =
        "program calls fn first<N>(x:[Field;N])->Field{x[0]} fn main()->Field{first<2>([7,9])}";
    let file = crate::parse_source(source, "calls.tri").unwrap();
    let checked = check(source).unwrap();
    for builder in [
        TIRBuilder::new(crate::target::TerrainConfig::triton())
            .with_mono_instances(checked.mono_instances),
        TIRBuilder::new(crate::target::TerrainConfig::triton())
            .with_call_resolutions(checked.call_resolutions),
    ] {
        let errors = builder.build_file(&file).unwrap_err();
        assert!(errors[0].message.contains("checked source-site resolution"));
        assert_eq!(
            errors[0].span.start as usize,
            source.rfind("first<").unwrap()
        );
    }
    for call in ["first(x)", "first<M>(x)"] {
        let source = format!("program calls fn first<N>(x:[Field;N])->Field{{x[0]}} fn outer<M>(x:[Field;M])->Field{{{call}}} fn main()->Field{{outer([7,9])}}");
        let errors = direct_ir(&source).unwrap_err();
        assert!(errors[0].message.contains("checked source-site resolution"));
        let file = crate::parse_source(&source, "calls.tri").unwrap();
        let mut checked = check(&source).unwrap();
        let start = source.find(call).unwrap() as u32;
        let forged = crate::typecheck::MonoInstance {
            name: "first".into(),
            size_args: vec![2],
        };
        checked
            .call_resolutions
            .insert(("outer".into(), start, start + 5), forged.clone());
        checked.mono_instances.push(forged);
        assert!(TIRBuilder::new(crate::target::TerrainConfig::triton())
            .with_mono_instances(checked.mono_instances)
            .with_call_resolutions(checked.call_resolutions)
            .build_file(&file)
            .is_err());
    }
}

#[test]
fn generic_aggregate_returns_keep_the_instantiated_width() {
    let source =
        "program calls fn id<N>(x:[Field;N])->[Field;N]{x} fn main()->Field{let x=id([7,9]) x[1]}";
    let ir = direct_ir(source).unwrap();
    let label = crate::typecheck::MonoInstance {
        name: "id".into(),
        size_args: vec![2],
    }
    .mangled_name();
    assert_eq!(direct_calls(&ir, "main"), vec![label]);
    assert!(!format!("{ir:?}").contains("ERROR:"));
}
