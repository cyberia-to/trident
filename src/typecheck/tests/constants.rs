use super::{check, check_err, check_with_flags};
use crate::{ir::tir::builder::TIRBuilder, ir::tree::lower::nox::NoxCompiler};
use crate::{typecheck::TypeChecker, types::Ty};
use std::collections::BTreeSet;

#[test]
fn constant_aliases_keep_raw_integers_and_declared_types() {
    let exports = check("module values\npub const NEXT:Field=BASE\nconst BASE:Field=18446744069414584322\npub const SIZE:U32=COUNT\nconst COUNT:U32=2").unwrap();
    assert_eq!(
        exports.constants,
        vec![
            ("NEXT".into(), Ty::Field, 18446744069414584322),
            ("SIZE".into(), Ty::U32, 2)
        ]
    );
    for declaration in [
        "const X:U32=4294967296",
        "const X:U32=18446744069414584322",
        "const X:Field=7\nconst Y:U32=X",
        "const X:U32=7\nconst Y:Field=X",
        "const X:Bool=true",
        "const X:Field=missing",
        "const X:Field=missing()",
        "const X:Field=1+2",
        "const X:Field=X",
        "const X:Field=Y\nconst Y:Field=X",
        "const X:Field=7\nconst X:Field=missing",
        "const X:Field=missing\nconst X:Field=7",
        "const X:U32=4294967296\nconst X:U32=7",
    ] {
        assert!(
            !check_err(&format!("module bad\n{declaration}")).is_empty(),
            "{declaration}"
        );
    }
}

#[test]
fn final_constant_binding_freezes_signature_sizes_and_export_visibility() {
    let exports =
        check("module sizes\npub const N:U32=1\npub fn f(a:[Field;N])->Field{a[1]}\nconst N:U32=2")
            .unwrap();
    assert!(exports.constants.is_empty());
    assert_eq!(
        exports.functions[0].1[0].1,
        Ty::Array(Box::new(Ty::Field), 2)
    );
    let source = "module values\npub const X:U32=7\nconst X:Field=4294967296\npub const Y:Field=X";
    let exports = check(source).unwrap();
    assert_eq!(exports.constants, vec![("Y".into(), Ty::Field, 4294967296)]);
    let mut checker = TypeChecker::new();
    checker.import_module(&exports);
    let file = crate::parse_source(
        "module consumer\nuse values\npub const Z:Field=values.Y",
        "consumer.tri",
    )
    .unwrap();
    let imported = checker.check_file(&file).unwrap();
    assert!(imported.warnings.is_empty(), "{:?}", imported.warnings);
    assert_eq!(
        imported.constants,
        vec![("Z".into(), Ty::Field, 4294967296)]
    );
    let mut checker = TypeChecker::new();
    checker.import_module(&exports);
    let file = crate::parse_source(
        "module consumer\nuse values\npub const Z:U32=values.X",
        "consumer.tri",
    )
    .unwrap();
    assert!(checker.check_file(&file).is_err());
}

#[test]
fn inactive_constant_initializers_do_not_affect_the_final_environment() {
    let source="module values\n#[cfg(debug)] pub const X:U32=7\n#[cfg(release)] pub const X:Field=9\n#[cfg(unused)] const X:Noun=missing()";
    assert_eq!(
        check_with_flags(source, &["debug"]).unwrap().constants,
        vec![("X".into(), Ty::U32, 7)]
    );
    assert_eq!(
        check_with_flags(source, &["release"]).unwrap().constants,
        vec![("X".into(), Ty::Field, 9)]
    );
}

#[test]
fn long_constant_alias_chains_resolve_without_host_recursion() {
    let mut source = String::from("module chain\npub const START:Field=C0\n");
    for i in 0..4096 {
        source.push_str(&format!("const C{i}:Field=C{}\n", i + 1));
    }
    source.push_str("const C4096:Field=7\n");
    assert_eq!(
        check(&source).unwrap().constants,
        vec![("START".into(), Ty::Field, 7)]
    );
}

#[test]
fn direct_lowerers_reject_invalid_constants_including_unused_declarations() {
    for definitions in [
        "const X:Field=missing",
        "const X:Field=1+2",
        "const X:Field=X",
        "const X:Field=7\nconst X:Field=missing",
        "const X:Field=missing\nconst X:Field=7",
    ] {
        let source = format!("program direct\n{definitions}\nfn main()->Field{{7}}");
        let file = crate::parse_source(&source, "direct.tri").unwrap();
        assert!(
            NoxCompiler::new()
                .compile_modules(&[&file], &file, &BTreeSet::new())
                .is_err(),
            "{source}"
        );
        assert!(
            TIRBuilder::new(crate::target::TerrainConfig::triton())
                .build_file(&file)
                .is_err(),
            "{source}"
        );
    }
    let file=crate::parse_source("program direct\n#[cfg(debug)] const X:Field=missing\n#[cfg(release)] const X:Field=7\nfn main()->Field{X}","direct.tri").unwrap();
    assert!(TIRBuilder::new(crate::target::TerrainConfig::triton())
        .with_module_types(&[&file])
        .with_cfg_flags(BTreeSet::from(["release".into()]))
        .build_file(&file)
        .is_ok());
    let owner = crate::parse_source("module values\nconst PRIVATE:Field=7", "values.tri").unwrap();
    let file = crate::parse_source(
        "program direct\nconst X:Field=values.PRIVATE\nfn main()->Field{7}",
        "direct.tri",
    )
    .unwrap();
    assert!(TIRBuilder::new(crate::target::TerrainConfig::triton())
        .with_module_types(&[&owner])
        .build_file(&file)
        .is_err());
}

#[test]
fn explicit_builder_constants_work_with_optional_module_layouts() {
    let file = crate::parse_source(
        "program direct\nconst X:Field=outside.BASE\nfn main()->Field{X}",
        "direct.tri",
    )
    .unwrap();
    let builder = || {
        TIRBuilder::new(crate::target::TerrainConfig::triton()).with_constants(
            std::collections::BTreeMap::from([("outside.BASE".into(), 7)]),
        )
    };
    assert_eq!(
        format!("{:?}", builder().build_file(&file).unwrap()),
        format!(
            "{:?}",
            builder()
                .with_module_types(&[&file])
                .build_file(&file)
                .unwrap()
        )
    );
}
