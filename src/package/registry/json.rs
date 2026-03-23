// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::types::*;

/// Maximum length for a single JSON string value (1 MB).
const MAX_STRING_LEN: usize = 1_000_000;

/// Maximum number of items in a JSON array.
const MAX_ARRAY_ITEMS: usize = 10_000;

pub(super) fn json_escape(s: &str) -> String {
    let mut out = String::from("\"");
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

/// Find a top-level JSON key (depth 1) and return its byte offset.
/// Skips keys nested inside arrays or sub-objects by tracking
/// brace/bracket nesting while being aware of JSON strings.
pub(super) fn find_toplevel_key(json: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{}\":", key);
    let bytes = json.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            // Skip over the entire JSON string (key or value).
            // Record the start position — we may need to match here.
            let start = i;
            i += 1; // skip opening quote
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i += 2; // skip escaped char
                } else if bytes[i] == b'"' {
                    i += 1; // skip closing quote
                    break;
                } else {
                    i += 1;
                }
            }
            // At depth 1, check if this position is our needle.
            if depth == 1 && json[start..].starts_with(&needle) {
                return Some(start);
            }
            continue;
        }
        match b {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    None
}

pub(super) fn extract_json_string(json: &str, key: &str) -> String {
    let needle = format!("\"{}\":", key);
    if let Some(pos) = find_toplevel_key(json, key) {
        let after = &json[pos + needle.len()..];
        let after = after.trim_start();
        if after.starts_with('"') {
            let inner = &after[1..];
            let mut result = String::new();
            let mut chars = inner.chars();
            while let Some(ch) = chars.next() {
                if ch == '"' {
                    break;
                }
                if ch == '\\' {
                    match chars.next() {
                        Some('n') => result.push('\n'),
                        Some('r') => result.push('\r'),
                        Some('t') => result.push('\t'),
                        Some('"') => result.push('"'),
                        Some('\\') => result.push('\\'),
                        Some(c) => {
                            result.push('\\');
                            result.push(c);
                        }
                        None => break,
                    }
                } else {
                    result.push(ch);
                }
                if result.len() > MAX_STRING_LEN {
                    return String::new();
                }
            }
            return result;
        }
    }
    String::new()
}

pub(super) fn extract_json_bool(json: &str, key: &str) -> bool {
    let needle = format!("\"{}\":", key);
    if let Some(pos) = find_toplevel_key(json, key) {
        let after = &json[pos + needle.len()..];
        let after = after.trim_start();
        return after.starts_with("true");
    }
    false
}

pub(super) fn extract_json_array_strings(json: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{}\":", key);
    let mut results = Vec::new();
    if let Some(pos) = find_toplevel_key(json, key) {
        let after = &json[pos + needle.len()..];
        let after = after.trim_start();
        if after.starts_with('[') {
            let bracket_end = find_matching_bracket(after);
            let inner = &after[1..bracket_end];
            for item in inner.split(',') {
                if results.len() >= MAX_ARRAY_ITEMS {
                    break;
                }
                let item = item.trim();
                if item.starts_with('"') && item.ends_with('"') {
                    let value = &item[1..item.len() - 1];
                    if value.len() > MAX_STRING_LEN {
                        continue;
                    }
                    results.push(value.to_string());
                }
            }
        }
    }
    results
}

pub(super) fn find_matching_bracket(s: &str) -> usize {
    let mut depth = 0;
    for (i, ch) in s.chars().enumerate() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    s.len()
}

pub(super) fn format_publish_json(def: &PublishedDefinition) -> String {
    let deps: Vec<String> = def
        .dependencies
        .iter()
        .map(|h| format!("\"{}\"", h))
        .collect();
    let params: Vec<String> = def
        .params
        .iter()
        .map(|(n, t)| {
            format!(
                "{{\"name\":{},\"type\":{}}}",
                json_escape(n),
                json_escape(t)
            )
        })
        .collect();
    let requires: Vec<String> = def.requires.iter().map(|r| json_escape(r)).collect();
    let ensures: Vec<String> = def.ensures.iter().map(|e| json_escape(e)).collect();
    let tags: Vec<String> = def.tags.iter().map(|t| json_escape(t)).collect();

    format!(
        "{{\"hash\":\"{}\",\"source\":{},\"module\":{},\"is_pub\":{},\"params\":[{}],\"return_ty\":{},\"dependencies\":[{}],\"requires\":[{}],\"ensures\":[{}],\"name\":{},\"tags\":[{}],\"verified\":{},\"verification_cert\":{}}}",
        def.hash,
        json_escape(&def.source),
        json_escape(&def.module),
        def.is_pub,
        params.join(","),
        def.return_ty.as_ref().map(|t| json_escape(t)).unwrap_or_else(|| "null".to_string()),
        deps.join(","),
        requires.join(","),
        ensures.join(","),
        def.name.as_ref().map(|n| json_escape(n)).unwrap_or_else(|| "null".to_string()),
        tags.join(","),
        def.verified,
        def.verification_cert.as_ref().map(|c| json_escape(c)).unwrap_or_else(|| "null".to_string()),
    )
}

#[cfg(test)]
pub(super) fn parse_publish_body(body: &str) -> Result<PublishedDefinition, String> {
    let hash = extract_json_string(body, "hash");
    if hash.is_empty() {
        return Err("missing 'hash' field".to_string());
    }
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid hash format (expected 64 hex chars)".to_string());
    }

    let source = extract_json_string(body, "source");
    if source.is_empty() {
        return Err("missing 'source' field".to_string());
    }

    let module = extract_json_string(body, "module");
    let is_pub = extract_json_bool(body, "is_pub");
    let return_ty = {
        let rt = extract_json_string(body, "return_ty");
        if rt.is_empty() {
            None
        } else {
            Some(rt)
        }
    };

    let params = extract_params_array(body);
    let dependencies = extract_json_array_strings(body, "dependencies");
    let requires = extract_json_array_strings(body, "requires");
    let ensures = extract_json_array_strings(body, "ensures");
    let tags = extract_json_array_strings(body, "tags");
    let name = {
        let n = extract_json_string(body, "name");
        if n.is_empty() {
            None
        } else {
            Some(n)
        }
    };
    let verified = extract_json_bool(body, "verified");
    let verification_cert = {
        let vc = extract_json_string(body, "verification_cert");
        if vc.is_empty() {
            None
        } else {
            Some(vc)
        }
    };

    Ok(PublishedDefinition {
        hash,
        source,
        module,
        is_pub,
        params,
        return_ty,
        dependencies,
        requires,
        ensures,
        name,
        tags,
        verified,
        verification_cert,
    })
}

pub(super) fn extract_params_array(json: &str) -> Vec<(String, String)> {
    let needle = "\"params\":";
    let mut results = Vec::new();
    if let Some(pos) = find_toplevel_key(json, "params") {
        let after = &json[pos + needle.len()..];
        let after = after.trim_start();
        if after.starts_with('[') {
            let bracket_end = find_matching_bracket(after);
            let inner = &after[1..bracket_end];
            for obj in inner.split("},") {
                if results.len() >= MAX_ARRAY_ITEMS {
                    break;
                }
                let name = extract_json_string(obj, "name");
                let ty = extract_json_string(obj, "type");
                if !name.is_empty() {
                    results.push((name, ty));
                }
            }
        }
    }
    results
}

pub(super) fn parse_pull_response(body: &str) -> PullResult {
    PullResult {
        hash: extract_json_string(body, "hash"),
        source: extract_json_string(body, "source"),
        module: extract_json_string(body, "module"),
        params: extract_params_array(body),
        return_ty: {
            let rt = extract_json_string(body, "return_ty");
            if rt.is_empty() {
                None
            } else {
                Some(rt)
            }
        },
        dependencies: extract_json_array_strings(body, "dependencies"),
        requires: extract_json_array_strings(body, "requires"),
        ensures: extract_json_array_strings(body, "ensures"),
    }
}

pub(super) fn parse_search_response(body: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let needle = "\"results\":[";
    if let Some(pos) = body.find(needle) {
        let after = &body[pos + needle.len() - 1..]; // include the [
        let bracket_end = find_matching_bracket(after);
        let inner = &after[1..bracket_end];

        for obj in inner.split("},{") {
            if results.len() >= MAX_ARRAY_ITEMS {
                break;
            }
            let name = extract_json_string(obj, "name");
            let hash = extract_json_string(obj, "hash");
            let module = extract_json_string(obj, "module");
            let signature = extract_json_string(obj, "signature");
            let verified = extract_json_bool(obj, "verified");
            let tags = extract_json_array_strings(obj, "tags");

            if !hash.is_empty() {
                results.push(SearchResult {
                    name,
                    hash,
                    module,
                    signature,
                    verified,
                    tags,
                });
            }
        }
    }
    results
}
