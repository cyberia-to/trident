mod support;
use std::fs;
use trident::tir::TIROp;
use trident::{
    target::{IntrinsicAbi, IntrinsicType, TargetPackage},
    CompileOptions,
};

fn package() -> TargetPackage {
    let mut package = support::triton_package();
    package.terrain.digest_width = 7;
    package.terrain.xfield_width = 4;
    package.intrinsics.push("owner_mix".into());
    package.intrinsic_abis.insert(
        "owner_mix".into(),
        IntrinsicAbi {
            params: vec![IntrinsicType::Digest, IntrinsicType::XField],
            results: vec![IntrinsicType::Field, IntrinsicType::Digest],
        },
    );
    package.modules.insert("vm.triton.custom".into(),
        "module vm.triton.custom\n#[intrinsic(owner_mix)]\npub fn mix(d: Digest, x: XField) -> (Field, Digest)\n".into());
    package.seal().unwrap()
}
fn collect(ops: &[TIROp], calls: &mut Vec<(String, u32, u32)>) {
    for op in ops {
        match op {
            TIROp::TargetCall {
                name,
                inputs,
                outputs,
            } => calls.push((name.clone(), *inputs, *outputs)),
            TIROp::IfElse {
                then_body,
                else_body,
            } => {
                collect(then_body, calls);
                collect(else_body, calls);
            }
            TIROp::IfOnly { then_body } => collect(then_body, calls),
            TIROp::Loop { body, .. } | TIROp::ProofBlock { body, .. } => collect(body, calls),
            _ => {}
        }
    }
}
#[test]
fn opaque_owner_calls_keep_selected_widths_in_regular_and_passthrough_paths() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("main.tri");
    fs::write(&file,"program custom_calls\nuse vm.triton.custom\nfn direct(d: Digest, x: XField) -> (Field, Digest) { let result = custom.mix(d, x)\n result }\nfn wrapper(d: Digest, x: XField) -> (Field, Digest) { custom.mix(d, x) }\nfn main(d: Digest, x: XField) -> (Field, Digest) { let ignored = direct(d, x)\n wrapper(d, x) }\n").unwrap();
    let options = CompileOptions::default().with_package(package()).unwrap();
    let modules = trident::build_tir_modules(&file, &options).unwrap();
    let mut calls = Vec::new();
    for module in &modules {
        collect(&module.ops, &mut calls);
    }
    assert_eq!(
        calls,
        vec![("owner_mix".into(), 11, 8), ("owner_mix".into(), 11, 8)]
    );
    // The nox emitter cannot accidentally manufacture target-owned assembly.
    assert!(trident::compile_to_bundle(&file, &options).is_err());
}

#[test]
fn declared_owner_signature_and_availability_are_enforced_before_tir() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("main.tri");
    fs::write(&file,"program caller\nuse vm.triton.custom\nfn main(d: Digest, x: XField) -> (Field, Digest) { custom.mix(d, x) }\n").unwrap();
    for declaration in [
        "pub fn mix(d: Field, x: XField) -> (Field, Digest)",
        "pub fn mix(d: Digest, x: XField) -> Digest",
        "pub fn mix(d: Digest, x: XField) -> (Field, Field)",
    ] {
        let mut bad = package();
        bad.modules.insert(
            "vm.triton.custom".into(),
            format!("module vm.triton.custom\n#[intrinsic(owner_mix)]\n{declaration}\n"),
        );
        let options = CompileOptions::default()
            .with_package(bad.seal().unwrap())
            .unwrap();
        let errors = trident::build_tir_modules(&file, &options).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("does not match the selected target ABI")),
            "{errors:?}"
        );
    }
    let mut absent = package();
    absent.intrinsic_abis.clear();
    let options = CompileOptions::default()
        .with_package(absent.seal().unwrap())
        .unwrap();
    assert!(trident::build_tir_modules(&file, &options)
        .unwrap_err()
        .iter()
        .any(|e| e.message.contains("unknown intrinsic")));
}

#[test]
fn owner_abi_changes_identity_but_cannot_override_language_or_reference_backend() {
    let package = package();
    let hash = package.compilation_hash().unwrap();
    let mut changed = package.clone();
    changed
        .intrinsic_abis
        .get_mut("owner_mix")
        .unwrap()
        .results
        .clear();
    assert_ne!(changed.compilation_hash().unwrap(), hash);
    let mut unavailable = package.clone();
    unavailable.intrinsics.retain(|n| n != "owner_mix");
    assert!(unavailable.seal().is_err());
    let mut language = package.clone();
    language.intrinsic_abis.insert(
        "hash".into(),
        IntrinsicAbi {
            params: vec![],
            results: vec![],
        },
    );
    assert!(language.seal().is_err());
    let mut too_wide = package.clone();
    too_wide.intrinsic_abis.get_mut("owner_mix").unwrap().params = vec![IntrinsicType::Field; 65];
    assert!(too_wide.seal().is_err());
    let mut nox = package;
    nox.terrain = trident::target::TerrainConfig::nox();
    nox.modules.clear();
    assert!(nox.seal().is_err());
}
