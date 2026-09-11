//! Version-matched language libraries and target declarations travel with the compiler.

include!(concat!(env!("OUT_DIR"), "/resources.rs"));

pub(crate) fn get(path: &str) -> Option<&'static str> {
    FILES
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, source)| *source)
}

pub(crate) fn module(name: &str) -> Option<&'static str> {
    get(&format!("lib/{}.tri", name.replace('.', "/")))
}

pub(crate) fn target_constants(target: &crate::target::TerrainConfig) -> String {
    format!("module std.target\npub const DIGEST_WIDTH: Field = {}\npub const XFIELD_WIDTH: Field = {}\npub const HASH_RATE: Field = {}\npub const FIELD_LIMBS: Field = {}\npub const STACK_DEPTH: Field = {}\n",
        target.digest_width, target.xfield_width, target.hash_rate, target.field_limbs, target.stack_depth)
}

pub(crate) fn native_hash(target: &crate::target::TerrainConfig) -> String {
    let params = (0..target.hash_rate).map(|i| format!("x{i}: Field")).collect::<Vec<_>>().join(", ");
    let zeros = std::iter::once("value".to_string()).chain((1..target.hash_rate).map(|_| "0".to_string())).collect::<Vec<_>>().join(", ");
    format!("module vm.crypto.hash\n// Target-native structural hash; algorithm and layout are part of the selected ABI.\n#[intrinsic(hash)]\npub fn native({params}) -> Digest\n#[pure]\npub fn single(value: Field) -> Digest {{ native({zeros}) }}\n")
}

/// Complete target-shaped I/O declarations. Availability is checked on reachable calls.
pub(crate) fn io(target: &crate::target::TerrainConfig) -> String {
    let mut source = "module vm.io.io\n#[intrinsic(divine)]\npub fn divine() -> Field\n".to_string();
    source.push_str("#[intrinsic(pub_read)]\npub fn read() -> Field\n#[intrinsic(pub_write)]\npub fn write(v: Field)\n");
    for n in 2..=target.digest_width {
        let fields = vec!["Field"; n as usize].join(", ");
        let params = (0..n).map(|i| format!("v{i}: Field")).collect::<Vec<_>>().join(", ");
        if n < target.digest_width {
            source.push_str(&format!("#[intrinsic(pub_read{n})]\npub fn read{n}() -> ({fields})\n"));
        }
        source.push_str(&format!("#[intrinsic(pub_write{n})]\npub fn write{n}({params})\n"));
    }
    source.push_str(&format!("#[intrinsic(pub_read{})]\npub fn read_digest() -> Digest\n#[intrinsic(divine{})]\npub fn divine_digest() -> Digest\n", target.digest_width, target.digest_width));
    if target.xfield_width > 0 && target.xfield_width != target.digest_width {
        let n = target.xfield_width;
        source.push_str(&format!("#[intrinsic(divine{n})]\npub fn divine{n}() -> ({})\n", vec!["Field"; n as usize].join(", ")));
    }
    source
}

/// Preserve the language declarations; each function advertises transitive
/// requirements, and reachable calls are checked against the owner's surface.
pub(crate) fn intrinsic_modules(_target: &crate::target::TerrainConfig) -> std::collections::BTreeMap<String, String> {
    FILES.iter().filter(|(path, _)| path.starts_with("lib/vm/") && path.ends_with(".tri"))
        .map(|(path, source)| (path.trim_start_matches("lib/").trim_end_matches(".tri").replace('/', "."), source.to_string())).collect()
}
