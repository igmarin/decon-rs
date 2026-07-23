//! Stage orchestration, checkpoint/resume, and dry-run planning for `decon`.
//!
//! The Python reference implementation's Pocket Flow graph becomes an
//! explicit `Pipeline` state machine here — clearer than a generic node
//! framework for this linear workflow (fetch → identify → relationships →
//! order → chapters → setup → overview → combine). Checkpoint format follows
//! ADR 0001 (content-addressed manifest, not a monolithic JSON blob).
//! Implementation lands across Milestones 1-4 — see `docs/move-to-rust.md`
//! §2.2 and §5.

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
