use std::{collections::BTreeMap, path::Path};

/// A declared foreign ABI fixture; dispatch tests never depend on an installed warrior.
pub fn triton_package() -> trident::target::TargetPackage {
    let terrain = trident::target::TerrainConfig::parse_toml(
        include_str!("../fixtures/stack-target.toml"),
        Path::new("fixture"),
    )
    .unwrap();
    trident::target::TargetPackage {
        schema_version: 1,
        compiler_api: 1,
        owner: "trisha".into(),
        version: "fixture".into(),
        intrinsics: include_str!("../fixtures/stack-intrinsics.txt")
            .lines()
            .map(str::to_owned)
            .collect(),
        terrain,
        union: None,
        states: Vec::new(),
        modules: BTreeMap::new(),
        module_hashes: BTreeMap::new(),
        instructions: Vec::new(),
        runtime: trident::target::RuntimeCapabilities {
            run: true,
            prove: true,
            verify: true,
            deploy: false,
            proof_formats: vec!["test-fixture".into()],
            restrictions: Vec::new(),
        },
    }
    .seal()
    .unwrap()
}

#[allow(dead_code)]
pub fn write_triton_package(dir: &Path) {
    let package = triton_package();
    std::fs::write(
        dir.join("triton.json"),
        serde_json::to_vec(&package).unwrap(),
    )
    .unwrap();
}
