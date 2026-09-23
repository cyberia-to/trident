//! Versioned, content-identified resources supplied by the owning warrior.
use super::{StateConfig, TerrainConfig, UnionConfig};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCapabilities {
    pub run: bool,
    pub prove: bool,
    pub verify: bool,
    pub deploy: bool,
    pub proof_formats: Vec<String>,
    pub restrictions: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetPackage {
    pub schema_version: u32,
    pub compiler_api: u32,
    pub owner: String,
    pub version: String,
    pub terrain: TerrainConfig,
    pub union: Option<UnionConfig>,
    pub states: Vec<StateConfig>,
    pub modules: BTreeMap<String, String>,
    pub module_hashes: BTreeMap<String, String>,
    pub intrinsics: Vec<String>,
    /// Owner-defined operations beyond the language's intrinsic vocabulary.
    #[serde(default)]
    pub intrinsic_abis: BTreeMap<String, super::IntrinsicAbi>,
    pub instructions: Vec<String>,
    pub runtime: RuntimeCapabilities,
}

impl TargetPackage {
    /// Refuse unsupported runtime operations before dispatching the owner binary.
    pub fn require_command(&self, command: &str) -> Result<(), String> {
        self.validate()?;
        let supported = match command {
            "build" => true,
            "run" | "test" => self.runtime.run,
            "prove" => self.runtime.prove,
            "verify" => self.runtime.verify,
            "deploy" => self.runtime.deploy,
            _ => false,
        };
        if supported {
            Ok(())
        } else {
            Err(format!(
                "{} does not support '{command}' for target '{}'",
                self.owner,
                self.union
                    .as_ref()
                    .map(|u| u.name.as_str())
                    .unwrap_or(&self.terrain.name)
            ))
        }
    }

    pub fn seal(mut self) -> Result<Self, String> {
        self.module_hashes = self
            .modules
            .iter()
            .map(|(name, source)| {
                (
                    name.clone(),
                    crate::hash::ContentHash(crate::hash::content_hash_bytes(source.as_bytes()))
                        .to_hex(),
                )
            })
            .collect();
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.compiler_api != crate::COMPILER_API {
            return Err("unsupported target package schema/compiler API".into());
        }
        if !identifier(&self.owner) || !identifier(&self.terrain.name) || self.version.is_empty() {
            return Err("invalid target package identity".into());
        }
        if self.modules.len() > 1024
            || self.modules.values().map(String::len).sum::<usize>() > 16 * 1024 * 1024
        {
            return Err("target package module size limit exceeded".into());
        }
        if self.terrain.digest_width == 0
            || self.terrain.digest_width > 64
            || self.terrain.hash_rate == 0
            || self.terrain.hash_rate > 256
            || self.terrain.field_limbs == 0
            || self.terrain.field_limbs > 64
            || self.terrain.xfield_width > 64
            || !self.terrain.output_extension.starts_with('.')
            || !identifier(&self.terrain.output_extension[1..])
        {
            return Err("invalid target ABI bounds or artifact extension".into());
        }
        if self.module_hashes.len() != self.modules.len() {
            return Err("module identity set differs from module source set".into());
        }
        for (name, source) in &self.modules {
            if !name.split('.').all(identifier)
                || matches!(name.as_str(), "std.target" | "vm.crypto.hash" | "vm.io.io")
            {
                return Err(format!("invalid or reserved target module {name}"));
            }
            if let Some(rest) = name.strip_prefix("os.") {
                let namespace = rest.split('.').next();
                if namespace != self.union.as_ref().map(|u| u.name.as_str()) {
                    return Err(format!("module {name} requires its selected union"));
                }
            }
            let file = crate::parse_source_silent(source, name)
                .map_err(|_| format!("invalid target module source: {name}"))?;
            if file.kind != crate::ast::FileKind::Module || file.name.node != *name {
                return Err(format!(
                    "target module declaration differs from key: {name}"
                ));
            }
            let hash = crate::hash::ContentHash(crate::hash::content_hash_bytes(source.as_bytes()))
                .to_hex();
            if self.module_hashes.get(name) != Some(&hash) {
                return Err(format!("module content identity mismatch: {name}"));
            }
        }
        if let Some(union) = &self.union {
            if !identifier(&union.name) || union.vm != self.terrain.name {
                return Err("union does not match target terrain".into());
            }
        }
        let mut names = std::collections::BTreeSet::new();
        for state in &self.states {
            if !identifier(&state.name)
                || self.union.as_ref().map(|u| &u.name) != Some(&state.union)
                || !names.insert(&state.name)
            {
                return Err("invalid, duplicate or unrelated target state".into());
            }
        }
        if self.states.iter().filter(|state| state.is_default).count() > 1 {
            return Err("target package has multiple default states".into());
        }
        for values in [&self.intrinsics, &self.instructions] {
            let mut names = std::collections::BTreeSet::new();
            if values.len() > 1024
                || values
                    .iter()
                    .any(|s| s.is_empty() || s.len() > 128 || !names.insert(s))
            {
                return Err("invalid or duplicate target capability names".into());
            }
        }
        if self.intrinsic_abis.len() > 256 {
            return Err("too many target intrinsic ABI declarations".into());
        }
        let language = crate::typecheck::TypeChecker::with_target(self.terrain.clone());
        for (name, abi) in &self.intrinsic_abis {
            if !identifier(name) || name.len() > 128 || !self.intrinsics.contains(name)
                || abi.params.len() > 64 || abi.results.len() > 64
                || abi.params.iter().chain(&abi.results).any(|t| t.width(&self.terrain) == 0)
            {
                return Err(format!("invalid or unavailable target intrinsic ABI '{name}'"));
            }
            if language.has_intrinsic_signature(name) {
                return Err(format!("target intrinsic ABI cannot redefine language intrinsic '{name}'"));
            }
            if self.terrain.name == "nox" {
                return Err("reference nox lowering does not support owner-defined target intrinsics".into());
            }
            let (inputs, outputs) = abi.widths(&self.terrain);
            if inputs > 4096 || outputs > 4096 {
                return Err(format!("target intrinsic ABI '{name}' exceeds the word limit"));
            }
        }
        if self.runtime.prove && (!self.runtime.verify || self.runtime.proof_formats.is_empty()) {
            return Err("proving capability requires verification and proof formats".into());
        }
        Ok(())
    }

    /// Stable identity of the compilation ABI and resources, excluding deployment presets.
    pub fn compilation_hash(&self) -> Result<String, String> {
        self.validate()?;
        let bytes = serde_json::to_vec(&(
            "trident-target-package-v1",
            self.compiler_api,
            &self.owner,
            &self.version,
            &self.terrain,
            &self.union,
            &self.module_hashes,
            &self.intrinsics,
            &self.intrinsic_abis,
        ))
        .map_err(|e| e.to_string())?;
        Ok(crate::hash::ContentHash(crate::hash::content_hash_bytes(&bytes)).to_hex())
    }
}

pub(super) fn identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

impl TerrainConfig {
    /// Implemented intrinsic surface of the reference nox lowering.
    /// Foreign implementations must supply their own explicit package capabilities.
    pub fn supported_intrinsics(&self) -> Vec<String> {
        if self.name == "nox" {
            return [
                "assert",
                "assert_eq",
                "assert_digest",
                "inv",
                "sub",
                "neg",
                "field_add",
                "field_mul",
                "as_field",
                "as_u32",
                "hash",
                "divine",
                "os.state.read",
                "nox_noun_atom",
                "nox_noun_pair",
                "nox_noun_head",
                "nox_noun_tail",
                "nox_noun_as_field",
                "nox_noun_eq",
                "nox_noun_identity",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect();
        }
        Vec::new()
    }

    /// Frozen foreign capability fixture for compiler unit tests only.
    #[cfg(test)]
    pub(crate) fn test_intrinsics() -> Vec<String> {
        include_str!("../../../tests/fixtures/stack-intrinsics.txt")
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

#[cfg(test)]
mod intrinsic_ownership_tests {
    use super::*;

    #[test]
    fn unknown_and_foreign_machines_do_not_inherit_reference_capabilities() {
        assert!(TerrainConfig::triton().supported_intrinsics().is_empty());
        let mut unknown = TerrainConfig::nox();
        unknown.name = "unknown".into();
        assert!(unknown.supported_intrinsics().is_empty());
        assert!(!TerrainConfig::nox().supported_intrinsics().is_empty());
    }
}
