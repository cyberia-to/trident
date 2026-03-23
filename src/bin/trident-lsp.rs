// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
fn main() {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(trident::lsp::run_server());
}
