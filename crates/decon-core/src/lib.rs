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

pub mod budget;
pub mod diagrams;
pub mod mermaid;
pub mod module;
pub mod scope;
pub mod setup;

// Re-exports inherit rustdoc from `budget` (no extra crate-root docs required).
pub use budget::{
    BudgetConfig, BudgetEstimate, DEFAULT_BATCH_CHAR_BUDGET, DEFAULT_CHARS_PER_TOKEN,
    DEFAULT_MAX_FILE_CHARS, DEFAULT_MAX_FULL_FILES_PER_MODULE, FileSize, PATH_STUB_PREFIX,
    TRUNCATION_MARKER, TruncateResult, capped_file_chars, estimate_budget, path_stub,
    path_stub_chars, truncate_content,
};
pub use diagrams::{
    DiagramEdge, learning_path_flowchart, module_inventory_flowchart, module_inventory_from_counts,
    system_map_flowchart,
};
pub use mermaid::{
    MAX_LABEL_CHARS, MAX_SEQUENCE_PARTICIPANTS, ValidateResult, participant_line, sanitize_label,
    sanitize_markdown_mermaid_blocks, sanitize_mermaid, sequence_participant_lines, stable_node_id,
    validate_mermaid,
};
pub use module::{ModuleCount, ModuleKey, ROOT_MODULE, discover_modules, module_key};
pub use scope::{
    FilterStats, ScopeFilterResult, filter_files_by_scope, is_shared_module, unscoped_filter_stats,
};
pub use setup::{
    CONFIG_FILE_NAMES, MIN_README_LEN, SHORT_README_PENALTY, SIGNAL_POINTS, SetupAssessment,
    SetupSignals, assess_setup,
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
