// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
use crate::*;

#[test]
fn test_generate_docs_simple() {
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        "program my_app\n\nfn helper(x: Field) -> Field {\n    x + 1\n}\n\nfn main() {\n    let a: Field = pub_read()\n    pub_write(helper(a))\n}\n",
    )
    .unwrap();

    let options = CompileOptions::default();
    let doc = generate_docs(&main_path, &options).expect("doc generation should succeed");

    // Should contain the program name as title
    assert!(
        doc.contains("# my_app"),
        "should have program name as title"
    );
    // Should contain function names
    assert!(
        doc.contains("fn helper("),
        "should document helper function"
    );
    assert!(doc.contains("fn main("), "should document main function");
    // Should contain cost summary section
    assert!(doc.contains("## Cost Summary"), "should have cost summary");
    assert!(doc.contains("Processor"), "should list Processor table");
    assert!(doc.contains("Padded"), "should list Padded height");
}

#[test]
fn test_generate_docs_with_structs() {
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        "program test\n\nstruct AuthData {\n    owner: Digest,\n    nonce: Field,\n}\n\nfn main() {\n    let d: Digest = divine5()\n    let auth: AuthData = AuthData { owner: d, nonce: 42 }\n    pub_write(auth.nonce)\n}\n",
    )
    .unwrap();

    let options = CompileOptions::default();
    let doc = generate_docs(&main_path, &options).expect("doc generation should succeed");

    // Should contain struct section
    assert!(doc.contains("## Structs"), "should have Structs section");
    assert!(
        doc.contains("struct AuthData"),
        "should document AuthData struct"
    );
    // Should contain field table with types and widths
    assert!(
        doc.contains("| owner | Digest | 5 |"),
        "should show owner field with Digest width 5"
    );
    assert!(
        doc.contains("| nonce | Field | 1 |"),
        "should show nonce field with Field width 1"
    );
    assert!(
        doc.contains("Total width: 6 field elements"),
        "should show total width"
    );
}

#[test]
fn test_generate_docs_with_events() {
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        "program test\n\nevent Transfer {\n    from: Field,\n    to: Field,\n    amount: Field,\n}\n\nfn main() {\n    reveal Transfer { from: 1, to: 2, amount: 100 }\n}\n",
    )
    .unwrap();

    let options = CompileOptions::default();
    let doc = generate_docs(&main_path, &options).expect("doc generation should succeed");

    // Should contain events section
    assert!(doc.contains("## Events"), "should have Events section");
    assert!(
        doc.contains("event Transfer"),
        "should document Transfer event"
    );
    // Should list event fields
    assert!(doc.contains("| from | Field |"), "should show from field");
    assert!(doc.contains("| to | Field |"), "should show to field");
    assert!(
        doc.contains("| amount | Field |"),
        "should show amount field"
    );
}

#[test]
fn test_generate_docs_cost_annotations() {
    let dir = tempfile::tempdir().unwrap();
    let main_path = dir.path().join("main.tri");
    std::fs::write(
        &main_path,
        "program test\n\nfn compute(x: Field) -> Field {\n    let d: Digest = hash(x, 0, 0, 0, 0, 0, 0, 0, 0, 0)\n    x\n}\n\nfn main() {\n    let a: Field = pub_read()\n    pub_write(compute(a))\n}\n",
    )
    .unwrap();

    let options = CompileOptions::default();
    let doc = generate_docs(&main_path, &options).expect("doc generation should succeed");

    // Should contain cost annotations on functions
    assert!(
        doc.contains("**Cost:**"),
        "should have cost annotations on functions"
    );
    assert!(doc.contains("cc="), "should show cycle count");
    assert!(doc.contains("hash="), "should show hash cost");
    assert!(doc.contains("u32="), "should show u32 cost");
    assert!(doc.contains("dominant:"), "should show dominant table");
    // The compute function uses split which has u32 cost
    assert!(doc.contains("**Module:** test"), "should show module name");
}

