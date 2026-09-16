use super::*;

// ─── State Configuration ───────────────────────────────────────────

/// A state is a sovereign chain instance within a union (network).
///
/// States share their union's protocol and engine, but have
/// independent ledgers, validators, and economies. Ethereum
/// Mainnet and Optimism are different states in the Ethereum union.
///
/// State config is purely deployment metadata — no impact on
/// compilation. Only relevant for deploy, run, prove, verify.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateConfig {
    /// State name (e.g. "mainnet").
    pub name: String,
    /// Display name (e.g. "Neptune Mainnet").
    pub display_name: String,
    /// Parent union name (e.g. "neptune").
    pub union: String,
    /// Chain identifier (e.g. "1").
    pub chain_id: String,
    /// RPC endpoint URL.
    pub rpc_url: String,
    /// Block explorer URL.
    pub explorer_url: String,
    /// Native currency symbol (e.g. "NEPT").
    pub currency_symbol: String,
    /// Whether this is the default state for its union.
    pub is_default: bool,
}

impl StateConfig {
    /// Try to resolve a state config by union and state name.
    ///
    /// Reads the selected runtime package. Unregistered unions and missing
    /// states return `Ok(None)`; invalid installed packages return an error.
    pub fn resolve(union: &str, state_name: &str) -> Result<Option<Self>, Diagnostic> {
        if !super::package::identifier(union) || !super::package::identifier(state_name) { return Ok(None); }
        if super::owner_for(union).is_some() {
            return Ok(TargetPackage::discover(union)?.states.into_iter().find(|s| s.name == state_name));
        }
        Ok(None)
    }

    /// Find the default state for a union.
    ///
    /// Reads the unique default state from the installed union package.
    /// Returns the first default found, or `Ok(None)` if none.
    pub fn default_for_union(union: &str) -> Result<Option<Self>, Diagnostic> {
        if !super::package::identifier(union) { return Ok(None); }
        if super::owner_for(union).is_none() { return Ok(None); }
        Ok(TargetPackage::discover(union)?.states.into_iter().find(|s| s.is_default))
    }

    pub fn list_states(union: &str) -> Vec<String> {
        match TargetPackage::discover(union) {
            Ok(package) => package.states.into_iter().map(|s| s.name).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Load a state config from a TOML file.
    pub fn load(path: &Path) -> Result<Self, Diagnostic> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Diagnostic::error(
                format!("cannot read state config '{}': {}", path.display(), e),
                Span::dummy(),
            )
        })?;
        Self::parse_toml(&content, path)
    }

    pub fn parse_toml(content: &str, path: &Path) -> Result<Self, Diagnostic> {
        let err =
            |msg: String| Diagnostic::error(format!("{}: {}", path.display(), msg), Span::dummy());

        let mut name = String::new();
        let mut display_name = String::new();
        let mut union = String::new();
        let mut chain_id = String::new();
        let mut is_default = false;
        let mut rpc_url = String::new();
        let mut explorer_url = String::new();
        let mut currency_symbol = String::new();

        let mut section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                section = trimmed[1..trimmed.len() - 1].trim().to_string();
                continue;
            }
            if let Some((key, value)) = trimmed.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                let unquoted = value.trim_matches('"');

                match (section.as_str(), key) {
                    ("state", "name") => name = unquoted.to_string(),
                    ("state", "display_name") => display_name = unquoted.to_string(),
                    ("state", "union") => union = unquoted.to_string(),
                    ("state", "chain_id") => chain_id = unquoted.to_string(),
                    ("state", "is_default") => is_default = value == "true",
                    ("endpoints", "rpc_url") => rpc_url = unquoted.to_string(),
                    ("endpoints", "explorer_url") => explorer_url = unquoted.to_string(),
                    ("currency", "symbol") => currency_symbol = unquoted.to_string(),
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            return Err(err("missing state.name".to_string()));
        }
        if union.is_empty() {
            return Err(err("missing state.union".to_string()));
        }

        Ok(Self {
            name,
            display_name,
            union,
            chain_id,
            rpc_url,
            explorer_url,
            currency_symbol,
            is_default,
        })
    }
}
