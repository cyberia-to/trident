// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::*;

#[test]
fn test_analyze_costs_valid() {
    let source = "program test\nfn main() {\n    pub_write(pub_read())\n}";
    let result = analyze_costs(source, "test.tri");
    assert!(result.is_ok());
    let cost = result.unwrap();
    assert!(cost.total.get(0) > 0);
    assert!(cost.padded_height.is_power_of_two());
}

#[test]
fn test_analyze_costs_type_error() {
    let source = "program test\nfn main() {\n    let x: U32 = pub_read()\n}";
    assert!(analyze_costs(source, "test.tri").is_err());
}

#[test]
fn test_coin_cost_analysis() {
    let path = std::path::Path::new("os/neptune/standards/coin.tri");
    if !path.exists() {
        return;
    }
    let cost = analyze_costs_project(path, &CompileOptions::default())
        .expect("cost analysis should succeed");

    // Processor table should be nonzero
    assert!(cost.total.get(0) > 0);

    // Token uses hash heavily (leaf hashing, config hashing, auth verification)
    assert!(cost.total.get(1) > 0, "token should have hash table cost");

    // Token uses u32 range checks for balance verification
    assert!(
        cost.total.get(2) > 0,
        "token should have u32 table cost for range checks"
    );

    // Padded height should be reasonable (power of 2)
    assert!(cost.padded_height.is_power_of_two());
    assert!(
        cost.padded_height <= 4096,
        "padded height {} seems too high",
        cost.padded_height
    );

    // Should have functions for all 5 operations
    let fn_names: Vec<&str> = cost.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(fn_names.contains(&"pay"), "missing pay cost");
    assert!(fn_names.contains(&"mint"), "missing mint cost");
    assert!(fn_names.contains(&"burn"), "missing burn cost");
    assert!(fn_names.contains(&"lock"), "missing lock cost");
    assert!(fn_names.contains(&"update"), "missing update cost");

    // Helper functions should appear (hash_config and verify_config
    // moved to plumb.tri, but hash_metadata remains local)
    assert!(
        fn_names.contains(&"hash_metadata"),
        "missing hash_metadata cost"
    );
    assert!(fn_names.contains(&"hash_leaf"), "missing hash_leaf cost");

    eprintln!(
        "Token cost: padded_height={}, cc={}, hash={}, u32={}",
        cost.padded_height,
        cost.total.get(0),
        cost.total.get(1),
        cost.total.get(2)
    );
    eprintln!("{}", cost.format_report());
}

#[test]
fn test_annotate_source_valid() {
    let source =
        "program test\n\nfn main() {\n    let x: Field = pub_read()\n    pub_write(x)\n}\n";
    let result = annotate_source(source, "test.tri");
    assert!(result.is_ok(), "annotate_source should succeed");
    let annotated = result.unwrap();

    // Should contain line numbers
    assert!(annotated.contains("1 |"), "should have line 1");
    assert!(annotated.contains("3 |"), "should have line 3");

    // Should contain cost annotations (brackets with cc= or jump=)
    assert!(
        annotated.contains("["),
        "should contain cost annotation brackets"
    );
    assert!(annotated.contains("cc="), "should contain cc= cost marker");

    // fn main() line should show call overhead (jump stack)
    let line3 = annotated.lines().find(|l| l.contains("fn main()"));
    assert!(line3.is_some(), "should have fn main() line");
    let line3 = line3.unwrap();
    assert!(
        line3.contains("jump="),
        "fn main() should show jump stack cost from call overhead"
    );
}

#[test]
fn test_annotate_source_shows_hash_cost() {
    let source = "program test\n\nfn main() {\n    let d: Digest = divine5()\n    let (d0, d1, d2, d3, d4) = d\n    let h: Digest = hash(d0, d1, d2, d3, d4, 0, 0, 0, 0, 0)\n    pub_write(0)\n}\n";
    let result = annotate_source(source, "test.tri");
    assert!(result.is_ok(), "annotate_source should succeed");
    let annotated = result.unwrap();

    // The hash line should show hash cost
    let hash_line = annotated.lines().find(|l| l.contains("hash("));
    assert!(hash_line.is_some(), "should have hash() line");
    let hash_line = hash_line.unwrap();
    assert!(
        hash_line.contains("hash="),
        "hash() line should show hash cost, got: {}",
        hash_line
    );
}

#[test]
fn test_cost_json_roundtrip_integration() {
    let source = "program test\nfn helper(x: Field) -> Field {\n    x + x\n}\nfn main() {\n    let x: Field = pub_read()\n    pub_write(helper(x))\n}";
    let cost_result = analyze_costs(source, "test.tri").expect("should analyze");
    let json = cost_result.to_json();

    // Verify JSON structure
    assert!(json.contains("\"functions\""), "JSON should have functions");
    assert!(json.contains("\"total\""), "JSON should have total");
    assert!(
        json.contains("\"padded_height\""),
        "JSON should have padded_height"
    );
    assert!(json.contains("\"main\""), "JSON should have main function");
    assert!(
        json.contains("\"helper\""),
        "JSON should have helper function"
    );

    // Round-trip
    let parsed =
        cost::ProgramCost::from_json(&json).expect("should parse JSON back to ProgramCost");
    for i in 0..parsed.total.count as usize {
        assert_eq!(parsed.total.get(i), cost_result.total.get(i));
    }
    assert_eq!(parsed.padded_height, cost_result.padded_height);
}

#[test]
fn test_comparison_formatting_integration() {
    let source_v1 =
        "program test\nfn main() {\n    let x: Field = pub_read()\n    pub_write(x)\n}";
    let source_v2 = "program test\nfn main() {\n    let x: Field = pub_read()\n    let y: Field = pub_read()\n    pub_write(x + y)\n}";

    let cost_v1 = analyze_costs(source_v1, "test.tri").expect("v1 should analyze");
    let cost_v2 = analyze_costs(source_v2, "test.tri").expect("v2 should analyze");

    let comparison = cost_v1.format_comparison(&cost_v2);
    assert!(
        comparison.contains("Cost comparison:"),
        "should have header"
    );
    assert!(comparison.contains("TOTAL"), "should have TOTAL row");
    assert!(
        comparison.contains("Padded height:"),
        "should have padded height row"
    );
    assert!(
        comparison.contains("main"),
        "should show main function in comparison"
    );

    // v2 has more operations, so delta should be positive
    assert!(
        comparison.contains("+"),
        "v2 should have higher cost than v1, showing + delta"
    );
}

