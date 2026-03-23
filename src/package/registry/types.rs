// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
// ─── Published Definition (wire format) ───────────────────────────

/// A definition as published to the registry (JSON wire format).
#[derive(Clone, Debug)]
pub struct PublishedDefinition {
    /// Content hash (hex).
    pub hash: String,
    /// Function source code.
    pub source: String,
    /// Module name.
    pub module: String,
    /// Is it public?
    pub is_pub: bool,
    /// Parameters: [(name, type)].
    pub params: Vec<(String, String)>,
    /// Return type (if any).
    pub return_ty: Option<String>,
    /// Dependencies: hex hashes of called functions.
    pub dependencies: Vec<String>,
    /// Preconditions.
    pub requires: Vec<String>,
    /// Postconditions.
    pub ensures: Vec<String>,
    /// Name binding (if any).
    pub name: Option<String>,
    /// Tags for search.
    pub tags: Vec<String>,
    /// Verification status.
    pub verified: bool,
    /// Verification certificate (opaque string, if available).
    pub verification_cert: Option<String>,
}

/// Search result entry.
#[derive(Clone, Debug)]
pub struct SearchResult {
    pub name: String,
    pub hash: String,
    pub module: String,
    pub signature: String,
    pub verified: bool,
    pub tags: Vec<String>,
}

/// Result of a publish operation.
#[derive(Clone, Debug)]
pub struct PublishResult {
    pub hash: String,
    pub created: bool,
    pub name_bound: bool,
}

/// Result of a pull operation.
#[derive(Clone, Debug)]
pub struct PullResult {
    pub hash: String,
    pub source: String,
    pub module: String,
    pub params: Vec<(String, String)>,
    pub return_ty: Option<String>,
    pub dependencies: Vec<String>,
    pub requires: Vec<String>,
    pub ensures: Vec<String>,
}

// ─── Registry Client ──────────────────────────────────────────────
