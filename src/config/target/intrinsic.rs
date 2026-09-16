//! Typed ABI declarations for opaque operations implemented by a target owner.
use super::TerrainConfig;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IntrinsicType {
    Field,
    Bool,
    U32,
    Digest,
    XField,
}

impl IntrinsicType {
    pub fn width(self, target: &TerrainConfig) -> u32 {
        match self {
            Self::Digest => target.digest_width,
            Self::XField => target.xfield_width,
            _ => 1,
        }
    }
}

/// Multiple results are a language tuple; no results means unit. Parameter
/// types are primitive ABI values, with widths resolved from the selected target.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntrinsicAbi {
    pub params: Vec<IntrinsicType>,
    pub results: Vec<IntrinsicType>,
}

impl IntrinsicAbi {
    pub fn widths(&self, target: &TerrainConfig) -> (u32, u32) {
        (
            self.params.iter().map(|t| t.width(target)).sum(),
            self.results.iter().map(|t| t.width(target)).sum(),
        )
    }
}
