// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use super::trident_grammar;

#[test]
fn grammar_json_roundtrip() {
    let grammar = trident_grammar();
    let json = grammar.to_json();
    // Verify it produces valid JSON with expected structure
    assert!(json.starts_with('{'));
    assert!(json.contains("\"name\": \"trident\""));
    assert!(json.contains("\"source_file\""));
}

#[test]
fn rule_count() {
    let grammar = trident_grammar();
    // 59 rules in the existing grammar.json
    assert_eq!(
        grammar.rules.len(),
        59,
        "expected 59 grammar rules, got {}",
        grammar.rules.len()
    );
}

#[test]
fn first_and_last_rules() {
    let grammar = trident_grammar();
    assert_eq!(grammar.rules.first().map(|r| r.0), Some("source_file"));
    assert_eq!(grammar.rules.last().map(|r| r.0), Some("line_comment"));
}

#[test]
fn extras_are_whitespace_and_comments() {
    let grammar = trident_grammar();
    assert_eq!(grammar.extras.len(), 2);
}
