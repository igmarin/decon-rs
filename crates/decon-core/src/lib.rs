//! Pure domain models and pipeline traits for `decon`.
//!
//! This crate intentionally performs no network or filesystem I/O so it stays
//! trivially unit-testable. Crawling lives in `decon-crawl`, provider clients
//! in `decon-llm`, and stage orchestration in `decon-pipeline`.
//!
//! Domain types (`FileBlob`, `ModuleKey`, `Abstraction`, `Relationship`,
//! `Chapter`, `RunConfig`, `Checkpoint`, ...) and pure helpers (context
//! budgeting, monorepo scope, mermaid sanitize/validate, setup-doc scoring)
//! land here incrementally, starting with Milestone 1 — see
//! `docs/move-to-rust.md` §2.3 and §4.1 for the full domain model.

#![deny(missing_docs)]

pub mod module;
pub mod scope;

pub use module::{ModuleCount, ModuleKey, ROOT_MODULE, discover_modules, module_key};
pub use scope::{
    FilterStats, ScopeFilterResult, filter_files_by_scope, is_shared_module, unscoped_filter_stats,
};

/// The version of this crate, as declared in `Cargo.toml`.
///
/// Exposed for diagnostics (e.g. `decon --version` reporting per-crate
/// versions) without callers needing to parse `Cargo.toml` themselves.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }
}
