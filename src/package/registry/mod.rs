// ---
// tags: trident, rust
// crystal-type: source
// crystal-domain: comp
// ---
//! Registry Client — HTTP client for interacting with a Trident registry.
//!
//! Provides a client for publishing and pulling content-addressed definitions
//! to/from a remote registry over HTTP. Wire format is JSON.

mod client;
mod json;
mod store_integration;
mod types;

pub use client::RegistryClient;
pub use store_integration::{publish_codebase, pull_into_codebase};
pub use types::{PublishResult, PublishedDefinition, PullResult, SearchResult};

#[cfg(test)]
mod tests;
