use std::process::Command;
fn audit(source: &str, z3: bool) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.tri");
    std::fs::write(&path, source).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_trident"));
    cmd.args(["audit", "--json"]).arg(&path);
    if z3 {
        cmd.arg("--z3");
    }
    cmd.output().unwrap()
}
#[test]
fn absent_or_unmodeled_obligations_are_inconclusive() {
    for body in [
        "let x = 1",
        "for i in 0..2 { assert(true) }",
        "let x = [1, 2] assert(true)",
    ] {
        let out = audit(&format!("program p\nfn main() {{ {body} }}"), false);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains("\"verdict\":\"unknown\""));
    }
}
#[test]
fn static_scalar_contracts_bind_real_return_and_reject_false_postcondition() {
    for (post, code) in [("result == 7", 0), ("result == 8", 1)] {
        let source = format!("module m\n#[ensures({post})]\npub fn f() -> Field {{ return 7 }}");
        let out = audit(&source, false);
        assert_eq!(
            out.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
#[test]
fn preconditions_guard_obligations_and_functions_have_independent_solver_state() {
    if Command::new("z3").arg("--version").output().is_err() {
        return;
    }
    let source = "module m\n#[requires(x == 3)]\n#[ensures(result == 4)]\npub fn a(x: Field) -> Field { x + 1 }\n#[requires(x == 8)]\n#[ensures(result == 9)]\npub fn b(x: Field) -> Field { x + 1 }";
    let out = audit(source, true);
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let bad = audit(&source.replace("result == 9", "result == 10"), true);
    assert_eq!(bad.status.code(), Some(1));
}
#[cfg(unix)]
#[test]
fn requested_solver_unknown_error_and_sat_cannot_exit_success() {
    use std::os::unix::fs::PermissionsExt;
    for (answer, code, exit) in [("unknown", 2, 0), ("sat", 1, 0), ("unsat", 2, 1)] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("z3");
        std::fs::write(
            &path,
            format!("#!/bin/sh\ncat >/dev/null\nprintf '{answer}\\n'\nexit {exit}\n"),
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let source = dir.path().join("main.tri");
        std::fs::write(&source, "program p\nfn main() { assert(true) }").unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_trident"))
            .args(["audit", "--z3"])
            .arg(source)
            .env("PATH", format!("{}:/usr/bin:/bin", dir.path().display()))
            .output()
            .unwrap();
        assert_eq!(
            out.status.code(),
            Some(code),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn independent_functions_do_not_share_witness_variables() {
    let file = trident::parse_source_silent(
        "module m\nfn a(x: Field) { assert(x == 3) }\nfn b(x: Field) { assert(x == 8) }",
        "m.tri",
    )
    .unwrap();
    let mut combined = trident::sym::ConstraintSystem::new();
    for (name, system) in trident::sym::analyze_all(&file) {
        combined.append_independent(system, &name);
    }
    let script = trident::smt::encode_system(&combined, trident::smt::QueryMode::WitnessExistence);
    if Command::new("z3").arg("--version").output().is_ok() {
        assert_eq!(
            trident::smt::run_z3(&script).unwrap().status,
            trident::smt::SmtStatus::Sat
        );
    }
}

#[test]
fn ssa_versions_cannot_alias_user_identifier_suffixes() {
    use trident::sym::{Constraint, ConstraintSystem, SymValue, SymVar};
    let mut system = ConstraintSystem::new();
    system.variables.insert("x".into(), 1);
    system.variables.insert("x_1".into(), 0);
    system.constraints.push(Constraint::Equal(
        SymValue::Var(SymVar {
            name: "x".into(),
            version: 1,
        }),
        SymValue::Const(3),
    ));
    system.constraints.push(Constraint::Equal(
        SymValue::Var(SymVar {
            name: "x_1".into(),
            version: 0,
        }),
        SymValue::Const(8),
    ));
    let script = trident::smt::encode_system(&system, trident::smt::QueryMode::WitnessExistence);
    if Command::new("z3").arg("--version").output().is_ok() {
        assert_eq!(
            trident::smt::run_z3(&script).unwrap().status,
            trident::smt::SmtStatus::Sat
        );
    }
}

#[test]
fn branch_returns_check_each_postcondition_and_skip_later_effects() {
    let source = "module branches\n#[ensures(result == 7)]\npub fn f(x: Field) -> Field { if x == 0 { return 7 }\n if x == 1 { return 7 }\n 7 }";
    assert!(audit(source, true).status.success());
    assert_eq!(
        audit(
            &source.replace("if x == 1 { return 7 }", "if x == 1 { return 8 }"),
            true
        )
        .status
        .code(),
        Some(1)
    );
    let source = "module skip\n#[requires(x == 0)]\n#[ensures(result == 7)]\npub fn f(x: Field) -> Field { if x == 0 { return 7 }\n assert(x == 1)\n 9 }";
    assert!(audit(source, true).status.success());
    assert!(
        audit(
            "module zero\n#[ensures(result == 0)]\npub fn f()->Field{neg(0)}",
            true
        )
        .status
        .success()
    );
    let canonical = "module canonical\n#[ensures(result == 7)]\npub fn f()->Field{if 18446744069414584321 {7}else{9}}";
    assert!(audit(canonical, true).status.success());
    assert_eq!(
        audit(&canonical.replace("result == 7", "result == 9"), true)
            .status
            .code(),
        Some(1)
    );
    let terminal = "module terminal\n#[ensures(result == 0)]\npub fn f(x: Field)->Field {if x == 0 {7} else {9}}";
    assert_eq!(audit(terminal, true).status.code(), Some(1));
    assert!(
        audit(&terminal.replace("result == 0", "result == result"), true)
            .status
            .success()
    );
    let nested = "module nested\n#[ensures(result == 7)]\npub fn f(a: Bool, b: Bool) -> Field { if a { if b { return 7 }\n return 7 }\n 7 }";
    assert!(audit(nested, true).status.success());
    assert_eq!(
        audit(
            &nested.replace("if b { return 7 }", "if b { return 8 }"),
            true
        )
        .status
        .code(),
        Some(1)
    );
    // Assertions must not become assumptions that erase their own failing path.
    let bad = "module fail\n#[ensures(result == 7)]\npub fn f(x: Field) -> Field { if x == 0 { assert(x == 1)\n return 7 }\n 7 }";
    assert_eq!(audit(bad, true).status.code(), Some(1));
}

#[test]
fn branch_scope_restores_shadowed_names_but_preserves_outer_mutation() {
    let source = "module scopes\n#[ensures(result == 13)]\npub fn f(flag: Bool) -> Field { let mut x: Field = 10\n if flag { x = 11\n let x: Field = 99\n assert(x == 99) } else { x = 11\n let other: Field = 77\n assert(other == 77) }\n let later = x + 2\n later }";
    let out = audit(source, true);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        audit(
            &source.replace("x = 11\n let other", "x = 12\n let other"),
            true
        )
        .status
        .code(),
        Some(1)
    );
}

#[test]
fn unreachable_constant_paths_and_empty_audits_are_not_false_passes() {
    assert!(audit(
        "module m\n#[ensures(result == 7)]\npub fn f() -> Field { if false { assert(false) }\n 7 }",
        true
    )
    .status
    .success());
    assert_eq!(
        audit("module m\npub fn f() { if false { assert(false) } }", true)
            .status
            .code(),
        Some(2)
    );
    let file = trident::parse_source_silent("module m\n#[requires(x == 0)]\n#[ensures(result == 7)]\npub fn f(x: Field) -> Field { if true { return 7 }\n let unused = divine()\n 9 }", "m.tri").unwrap();
    let system = trident::sym::SymExecutor::new().execute_function(&file, "f");
    assert!(system.divine_inputs.is_empty());
    assert!(system.unsupported.is_empty());
}

#[test]
fn field_branch_truth_and_assertion_words_follow_the_selected_target() {
    use trident::{smt, sym, target::TerrainConfig};
    for target in [
        TerrainConfig::nox(),
        TerrainConfig::parse_toml(
            include_str!("fixtures/stack-target.toml"),
            std::path::Path::new("fixture"),
        )
        .unwrap(),
    ] {
        for value in [0, 1, 2] {
            let taken = if target.name == "nox" {
                value == 0
            } else {
                value != 0
            };
            let expected = if taken { 7 } else { 9 };
            for (post, status) in [(expected, smt::SmtStatus::Unsat), (8, smt::SmtStatus::Sat)] {
                let source = format!(
                    "module m\n#[requires(x == {value})]\n#[ensures(result == {post})]\npub fn f(x: Field) -> Field {{ if x {{ return 7 }}\n 9 }}"
                );
                let file = trident::parse_source_silent(&source, "m.tri").unwrap();
                let system = sym::SymExecutor::with_target(&target)
                    .unwrap()
                    .execute_function(&file, "f");
                assert!(system.unsupported.is_empty());
                let query = smt::encode_system(&system, smt::QueryMode::SafetyCheck);
                assert_eq!(
                    smt::run_z3(&query).unwrap().status,
                    status,
                    "{} x={value}",
                    target.name
                );
            }
            let source =
                format!("module m\n#[requires(x == {value})]\npub fn f(x: Field) {{ assert(x) }}");
            let file = trident::parse_source_silent(&source, "m.tri").unwrap();
            let system = sym::SymExecutor::with_target(&target)
                .unwrap()
                .execute_function(&file, "f");
            let valid = value == if target.name == "nox" { 0 } else { 1 };
            assert_eq!(
                smt::run_z3(&smt::encode_system(&system, smt::QueryMode::SafetyCheck))
                    .unwrap()
                    .status,
                if valid {
                    smt::SmtStatus::Unsat
                } else {
                    smt::SmtStatus::Sat
                }
            );
        }
    }
}

#[test]
fn path_budget_and_missing_solver_fail_closed() {
    let params = (0..9)
        .map(|n| format!("b{n}: Bool"))
        .collect::<Vec<_>>()
        .join(",");
    let mut source = format!("module paths\npub fn f({params}) {{\n");
    for n in 0..9 {
        source.push_str(&format!("if b{n} {{ let local: Field = 1 }}\n"));
    }
    source.push_str("assert(true)\n}");
    assert_eq!(audit(&source, true).status.code(), Some(2));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("main.tri");
    std::fs::write(&path, "program absent\nfn main() { assert(true) }").unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_trident"))
        .args(["audit", "--z3"])
        .arg(path)
        .env("PATH", dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn nox_field_and_boolean_branch_models_match_actual_native_execution() {
    use nox::{NoTrace, NullCalls, Outcome, Reduction};
    fn load(arena: &mut Reduction<4096>, text: &[u8], offset: &mut usize) -> nox::Order {
        if text[*offset] == b'[' {
            *offset += 1;
            let head = load(arena, text, offset);
            assert_eq!(text[*offset], b' ');
            *offset += 1;
            let tail = load(arena, text, offset);
            assert_eq!(text[*offset], b']');
            *offset += 1;
            arena.pair(head, tail).unwrap()
        } else {
            let start = *offset;
            while *offset < text.len() && text[*offset].is_ascii_digit() {
                *offset += 1;
            }
            let word = std::str::from_utf8(&text[start..*offset])
                .unwrap()
                .parse::<u64>()
                .unwrap();
            arena.atom(nebu::Goldilocks::new(word)).unwrap()
        }
    }
    for ty in ["Field", "Bool"] {
        for input in 0..=2 {
            if ty == "Bool" && input == 2 {
                continue;
            }
            let source =
                format!("program native\nfn main(x: {ty}) -> Field {{ if x {{ return 7 }}\n 9 }}");
            let assembly = trident::compile_with_options(
                &source,
                "native.tri",
                &trident::CompileOptions::default(),
            )
            .unwrap();
            let mut arena = Reduction::<4096>::new();
            let formula = load(&mut arena, assembly.as_bytes(), &mut 0);
            let subject = load(&mut arena, format!("[{input} 0]").as_bytes(), &mut 0);
            let expected = if input == 0 { 7 } else { 9 };
            match nox::reduce(
                &mut arena,
                subject,
                formula,
                100_000,
                &NullCalls,
                &mut NoTrace,
            ) {
                Outcome::Ok(value, _) => {
                    assert_eq!(arena.atom_value(value).unwrap().as_u64(), expected)
                }
                other => panic!("native execution failed: {other:?}"),
            }
            let precondition = if ty == "Field" {
                format!("x == {input}")
            } else {
                format!("x == {}", input == 0)
            };
            let contracted = source.replace(
                "fn main",
                &format!("#[requires({precondition})]\n#[ensures(result == {expected})]\nfn main"),
            );
            let result = audit(&contracted, true);
            assert!(
                result.status.success(),
                "{contracted}\n{}\n{}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
}

#[test]
fn scalar_helpers_prove_real_bodies_and_preserve_caller_argument_scope() {
    let source = "module helpers
#[ensures(result == x*100+y)]
fn pair(x:Field,y:Field)->Field {x*100+y}
#[ensures(result == x+1)]
fn next(x:Field)->Field {if x==0 {let x:Field=17\n return 1}\n x+1}
#[requires(x == 7)]
#[requires(y == 19)]
#[ensures(result == 1908)]
pub fn main(x:Field,y:Field)->Field {let answer=pair(y,x)\n answer+next(0)}";
    let output = audit(source, true);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let failed = audit(&source.replace("result == 1908", "result == 1909"), true);
    assert_eq!(failed.status.code(), Some(1));
}

#[test]
fn scalar_helper_preconditions_and_ensures_are_obligations_not_summaries() {
    let source = "module helpers
#[requires(x == 0)]
#[ensures(result == 1)]
fn helper(x:Field)->Field {if x==0 {assert(x==0)\n return 1}\n assert(x==3)\n 2}
#[requires(x == 0)]
#[ensures(result == 1)]
pub fn main(x:Field)->Field {helper(x)}";
    let output = audit(source, true);
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let failed_pre = source.replacen(
        "#[requires(x == 0)]\n#[ensures(result == 1)]\npub fn main",
        "#[requires(x == 1)]\n#[ensures(result == 2)]\npub fn main",
        1,
    );
    assert_eq!(audit(&failed_pre, true).status.code(), Some(1));
    let false_ensures = source.replacen("#[ensures(result == 1)]", "#[ensures(result == 99)]", 1);
    assert_eq!(audit(&false_ensures, true).status.code(), Some(1));
}

#[test]
fn recursive_and_effectful_helpers_remain_unknown_in_direct_symbolic_analysis() {
    for source in [
        "module m\n#[ensures(result == 1)] fn helper(x:Field)->Field {helper(x)}\nfn main(){assert(helper(0)==1)}",
        "module m\n#[ensures(result == 1)] fn helper()->Field {let unused:Field=divine()\n1}\nfn main(){assert(helper()==1)}",
    ] {
        let file = trident::parse_source_silent(source, "helper.tri").unwrap();
        let system = trident::verify::sym::SymExecutor::new().execute_function(&file, "main");
        assert!(!system.unsupported.is_empty());
        let cli = audit(source, true);
        if source.contains("helper(x)}") {
            // The public compiler rejects recursion even before symbolic coverage.
            assert_eq!(cli.status.code(), Some(1));
            assert!(String::from_utf8_lossy(&cli.stderr).contains("recursive call cycle"));
        } else {
            assert_eq!(cli.status.code(), Some(2));
            assert!(String::from_utf8_lossy(&cli.stdout).contains("unknown"));
        }
    }
}

#[test]
fn acyclic_scalar_helper_expansion_is_bounded_and_unknown() {
    let mut source = String::from("module bound_test\nfn f0(x:Field)->Field{x+x}\n");
    for depth in 1..25 {
        source.push_str(&format!(
            "fn f{depth}(x:Field)->Field{{f{}(x)+f{}(x)}}\n",
            depth - 1,
            depth - 1
        ));
    }
    source.push_str("fn main(x:Field){assert(f24(x)==x)}");
    let file = trident::parse_source_silent(&source, "bounded.tri").unwrap();
    let system = trident::verify::sym::SymExecutor::new().execute_function(&file, "main");
    assert!(
        system
            .unsupported
            .iter()
            .any(|reason| reason.contains("budget"))
    );
}

#[test]
fn scalar_helper_expression_duplication_exhausts_expansion_budget() {
    let mut source = String::from("module expand_test\nfn helper(x:Field)->Field{let x0=x+x\n");
    for depth in 1..25 {
        source.push_str(&format!("let x{depth}=x{}+x{}\n", depth - 1, depth - 1));
    }
    source.push_str("x24}\nfn main(x:Field){assert(helper(x)==x)}");
    let file = trident::parse_source_silent(&source, "expand.tri").unwrap();
    let system = trident::verify::sym::SymExecutor::new().execute_function(&file, "main");
    assert!(
        system
            .unsupported
            .iter()
            .any(|reason| reason.contains("expansion budget"))
    );
}
