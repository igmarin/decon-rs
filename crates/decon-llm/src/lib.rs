//! LLM provider clients and the `LlmClient` trait for `decon`.
//!
//! Defines a provider-agnostic `LlmClient` trait plus concrete
//! implementations (OpenAI-compatible first, per the project's provider
//! priority), retries/backoff, bounded concurrency, and disk response
//! caching keyed by `hash(prompt) + model + provider`. Implementation lands
//! in Milestone 3 — see `docs/move-to-rust.md` §4.5.

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
