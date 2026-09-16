//! Editor signatures come from the selected target's type checker.
use crate::CompileOptions;

pub fn builtin_signature(
    name: &str,
    options: &CompileOptions,
) -> Option<(Vec<(String, String)>, String)> {
    let checker = options.checker();
    let signature = checker
        .available_functions()
        .find(|(candidate, _)| candidate.as_str() == name)?
        .1;
    Some((
        signature
            .params
            .iter()
            .map(|(name, ty)| (name.clone(), ty.display()))
            .collect(),
        if signature.return_ty == crate::types::Ty::Unit {
            String::new()
        } else {
            signature.return_ty.display()
        },
    ))
}

pub fn builtin_hover(name: &str, options: &CompileOptions) -> Option<String> {
    let (params, ret) = builtin_signature(name, options)?;
    let params = params
        .iter()
        .map(|(name, ty)| format!("{name}: {ty}"))
        .collect::<Vec<_>>()
        .join(", ");
    let ret = if ret.is_empty() {
        ret
    } else {
        format!(" -> {ret}")
    };
    Some(format!(
        "```trident\nfn {name}({params}){ret}\n```\nIntrinsic for `{}`.",
        options.target_config.name
    ))
}

pub fn builtin_completions(options: &CompileOptions) -> Vec<(String, String)> {
    options
        .checker()
        .available_functions()
        .map(|(name, sig)| {
            let params = sig
                .params
                .iter()
                .map(|(name, ty)| format!("{name}: {}", ty.display()))
                .collect::<Vec<_>>()
                .join(", ");
            (
                name.clone(),
                format!("({params}) -> {}", sig.return_ty.display()),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_follow_target_abi() {
        let nox = CompileOptions::default();
        let package = crate::target::TargetPackage {
            intrinsic_abis: Default::default(),
            schema_version: 1,
            compiler_api: crate::COMPILER_API,
            owner: "fixture".into(),
            version: "1".into(),
            terrain: crate::target::TerrainConfig::triton(),
            union: None,
            states: vec![],
            modules: Default::default(),
            module_hashes: Default::default(),
            intrinsics: crate::target::TerrainConfig::test_intrinsics(),
            instructions: vec![],
            runtime: crate::target::RuntimeCapabilities {
                run: false,
                prove: false,
                verify: false,
                deploy: false,
                proof_formats: vec![],
                restrictions: vec![],
            },
        };
        let triton = nox.clone().with_package(package).unwrap();
        assert_eq!(
            builtin_signature("hash", &nox).unwrap().0.len(),
            nox.target_config.hash_rate as usize
        );
        assert_eq!(
            builtin_signature("hash", &triton).unwrap().0.len(),
            triton.target_config.hash_rate as usize
        );
        assert!(builtin_signature("pub_read5", &nox).is_none());
        assert!(builtin_signature("pub_read5", &triton).is_some());
        assert!(!builtin_hover("hash", &nox).unwrap().contains("Tip5"));
        assert!(builtin_hover("missing", &nox).is_none());
    }
}
