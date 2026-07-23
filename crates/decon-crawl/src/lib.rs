//! Local filesystem and GitHub repository crawling for `decon`.
//!
//! Ports `crawl_local_files` / `crawl_github_files` from the Python
//! reference implementation (`utils/crawl_local_files.py`,
//! `utils/crawl_github_files.py`) using `ignore`/`walkdir` for gitignore-aware
//! local walks and `reqwest` for the GitHub REST API. Implementation lands in
//! Milestone 1 — see `docs/move-to-rust.md` §5.

#![deny(missing_docs)]

/// The version of this crate, as declared in `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }
}
