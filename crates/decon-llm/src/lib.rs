//! LLM provider clients and the `LlmClient` trait for `decon`.
//!
//! Defines a provider-agnostic `LlmClient` trait plus concrete
//! implementations (OpenAI-compatible first, per the project's provider
//! priority), retries/backoff, bounded concurrency, and disk response
//! caching keyed by `hash(prompt) + model + provider`.
//!
//! Milestone 2 delivers [`cache::DiskCache`] (no live network). Milestone 3
//! adds the [`LlmClient`] trait, the [`LlmError`] error enum, and the
//! [`MockClient`] test double. Provider HTTP clients land in a later
//! milestone — see `docs/move-to-rust.md` §4.5.

#![deny(missing_docs)]

pub mod cache;
pub mod client;
pub mod error;
pub mod mock;

pub use cache::{CacheError, CacheKeyInput, DiskCache, cache_key};
pub use client::LlmClient;
pub use error::LlmError;
pub use mock::MockClient;

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
