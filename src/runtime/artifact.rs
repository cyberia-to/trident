// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Program bundle: self-contained compilation artifact for warrior consumption.
//!
//! Contains the compiled assembly, metadata, cost analysis, and function
//! signatures. Warriors deserialize this from a JSON file or receive it
//! via the Rust API.

// ─── Data Types ────────────────────────────────────────────────────

/// Self-contained compilation artifact that a warrior needs to execute,
/// prove, or deploy a Trident program.
#[derive(Clone, Debug)]
pub struct ProgramBundle {
    /// Program name (from project or filename).
    pub name: String,
    /// Program version.
    pub version: String,
    /// Target VM name (e.g. "triton", "miden").
    pub target_vm: String,
    /// Target OS name, if any (e.g. "neptune").
    pub target_os: Option<String>,
    /// Compiled assembly text (TASM for Triton, MASM for Miden, etc.).
    pub assembly: String,
    /// Entry point function name.
    pub entry_point: String,
    /// Function signatures with content hashes.
    pub functions: Vec<BundleFunction>,
    /// Cost analysis summary.
    pub cost: BundleCost,
    /// Content hash of the source AST (hex).
    pub source_hash: String,
}

/// Function metadata within a bundle.
#[derive(Clone, Debug)]
pub struct BundleFunction {
    pub name: String,
    pub hash: String,
    pub signature: String,
}

/// Cost analysis summary.
#[derive(Clone, Debug)]
pub struct BundleCost {
    /// Cost values per table.
    pub table_values: Vec<u64>,
    /// Table names (e.g. ["processor", "hash", "u32", ...]).
    pub table_names: Vec<String>,
    /// Padded trace height (next power of two).
    pub padded_height: u64,
    /// Estimated proving time in nanoseconds.
    pub estimated_proving_ns: u64,
}

// ─── JSON Serialization ────────────────────────────────────────────

impl ProgramBundle {
    /// Serialize to JSON (hand-rolled, matching existing patterns).
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        out.push_str(&format!("  \"name\": {},\n", json_string(&self.name)));
        out.push_str(&format!("  \"version\": {},\n", json_string(&self.version)));
        out.push_str(&format!(
            "  \"target_vm\": {},\n",
            json_string(&self.target_vm)
        ));
        match &self.target_os {
            Some(os) => out.push_str(&format!("  \"target_os\": {},\n", json_string(os))),
            None => out.push_str("  \"target_os\": null,\n"),
        }
        out.push_str(&format!(
            "  \"entry_point\": {},\n",
            json_string(&self.entry_point)
        ));
        out.push_str(&format!(
            "  \"source_hash\": {},\n",
            json_string(&self.source_hash)
        ));

        // Cost
        out.push_str("  \"cost\": {\n");
        for (i, name) in self.cost.table_names.iter().enumerate() {
            let val = self.cost.table_values.get(i).copied().unwrap_or(0);
            out.push_str(&format!("    {}: {},\n", json_string(name), val));
        }
        out.push_str(&format!(
            "    \"padded_height\": {},\n",
            self.cost.padded_height
        ));
        out.push_str(&format!(
            "    \"estimated_proving_ns\": {}\n",
            self.cost.estimated_proving_ns
        ));
        out.push_str("  },\n");

        // Functions
        out.push_str("  \"functions\": [\n");
        for (i, func) in self.functions.iter().enumerate() {
            let comma = if i + 1 < self.functions.len() {
                ","
            } else {
                ""
            };
            out.push_str(&format!(
                "    {{ \"name\": {}, \"hash\": {}, \"signature\": {} }}{}\n",
                json_string(&func.name),
                json_string(&func.hash),
                json_string(&func.signature),
                comma,
            ));
        }
        out.push_str("  ],\n");

        // Assembly (last field, no trailing comma)
        out.push_str(&format!(
            "  \"assembly\": {}\n",
            json_string(&self.assembly)
        ));
        out.push_str("}\n");
        out
    }

    /// Deserialize from JSON (minimal parser for the bundle format).
    pub fn from_json(json: &str) -> Result<Self, String> {
        let name = extract_string(json, "name")?;
        let version = extract_string(json, "version")?;
        let target_vm = extract_string(json, "target_vm")?;
        let target_os = extract_string_opt(json, "target_os");
        let entry_point = extract_string(json, "entry_point")?;
        let source_hash = extract_string(json, "source_hash")?;
        let assembly = extract_string(json, "assembly")?;
        let padded_height = extract_u64(json, "padded_height").unwrap_or(0);
        let estimated_proving_ns = extract_u64(json, "estimated_proving_ns").unwrap_or(0);

        Ok(ProgramBundle {
            name,
            version,
            target_vm,
            target_os,
            assembly,
            entry_point,
            functions: Vec::new(), // TODO: parse functions array
            cost: BundleCost {
                table_values: Vec::new(),
                table_names: Vec::new(),
                padded_height,
                estimated_proving_ns,
            },
            source_hash,
        })
    }
}

// ─── JSON Helpers ──────────────────────────────────────────────────

/// JSON-escape a string and wrap in quotes.
fn json_string(s: &str) -> String {
    let mut out = String::from('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Extract a string value for a key from JSON.
fn extract_string(json: &str, key: &str) -> Result<String, String> {
    let pattern = format!("\"{}\"", key);
    let start = json
        .find(&pattern)
        .ok_or_else(|| format!("missing key '{}'", key))?;
    let rest = &json[start + pattern.len()..];
    // Skip `: "`
    let quote_start = rest
        .find('"')
        .ok_or_else(|| format!("missing value for '{}'", key))?;
    let value_start = quote_start + 1;
    let value_rest = &rest[value_start..];
    // Find closing quote (handling escapes)
    let mut end = 0;
    let mut escaped = false;
    let mut value = String::new();
    for ch in value_rest.chars() {
        if escaped {
            match ch {
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                _ => {
                    value.push('\\');
                    value.push(ch);
                }
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            break;
        } else {
            value.push(ch);
        }
        end += ch.len_utf8();
    }
    let _ = end; // suppress unused warning
    Ok(value)
}

/// Extract an optional string value (returns None if key is "null").
fn extract_string_opt(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let start = json.find(&pattern)?;
    let rest = &json[start + pattern.len()..];
    let trimmed = rest.trim_start().trim_start_matches(':').trim_start();
    if trimmed.starts_with("null") {
        return None;
    }
    extract_string(json, key).ok()
}

/// Extract a u64 value for a key from JSON.
fn extract_u64(json: &str, key: &str) -> Result<u64, String> {
    let pattern = format!("\"{}\"", key);
    let start = json
        .find(&pattern)
        .ok_or_else(|| format!("missing key '{}'", key))?;
    let rest = &json[start + pattern.len()..];
    let colon = rest
        .find(':')
        .ok_or_else(|| format!("missing colon for '{}'", key))?;
    let after_colon = rest[colon + 1..].trim_start();
    let num_end = after_colon
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_colon.len());
    after_colon[..num_end]
        .parse()
        .map_err(|e| format!("invalid u64 for '{}': {}", key, e))
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bundle() -> ProgramBundle {
        ProgramBundle {
            name: "test_program".to_string(),
            version: "0.1.0".to_string(),
            target_vm: "triton".to_string(),
            target_os: Some("neptune".to_string()),
            assembly: "    call main\n    halt\nmain:\n    push 42\n    return\n".to_string(),
            entry_point: "main".to_string(),
            functions: vec![BundleFunction {
                name: "main".to_string(),
                hash: "abc123".to_string(),
                signature: "fn main() -> Field".to_string(),
            }],
            cost: BundleCost {
                table_values: vec![100, 50, 10],
                table_names: vec![
                    "processor".to_string(),
                    "hash".to_string(),
                    "u32".to_string(),
                ],
                padded_height: 128,
                estimated_proving_ns: 1_000_000,
            },
            source_hash: "deadbeef".to_string(),
        }
    }

    #[test]
    fn bundle_json_roundtrip() {
        let bundle = sample_bundle();
        let json = bundle.to_json();
        let parsed = ProgramBundle::from_json(&json).expect("parse failed");

        assert_eq!(parsed.name, bundle.name);
        assert_eq!(parsed.version, bundle.version);
        assert_eq!(parsed.target_vm, bundle.target_vm);
        assert_eq!(parsed.target_os, bundle.target_os);
        assert_eq!(parsed.entry_point, bundle.entry_point);
        assert_eq!(parsed.source_hash, bundle.source_hash);
        assert_eq!(parsed.cost.padded_height, bundle.cost.padded_height);
        assert_eq!(
            parsed.cost.estimated_proving_ns,
            bundle.cost.estimated_proving_ns
        );
        // Assembly contains newlines — verify escape roundtrip
        assert_eq!(parsed.assembly, bundle.assembly);
    }

    #[test]
    fn bundle_no_os() {
        let mut bundle = sample_bundle();
        bundle.target_os = None;
        let json = bundle.to_json();
        let parsed = ProgramBundle::from_json(&json).expect("parse failed");
        assert_eq!(parsed.target_os, None);
    }

    #[test]
    fn bundle_json_contains_assembly() {
        let bundle = sample_bundle();
        let json = bundle.to_json();
        assert!(json.contains("\"assembly\""));
        assert!(json.contains("push 42"));
    }
}
