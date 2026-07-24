//! Disk response cache for LLM calls (structure only — no network).
//!
//! Keys are stable SHA-256 digests of the canonical JSON object
//! `{ "prompt", "model", "provider", "extras" }`. Values are opaque UTF-8
//! response bodies stored as files under a cache root.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Errors from cache keying or filesystem operations.
#[derive(Debug, Error)]
pub enum CacheError {
    /// Failed to serialize the key material.
    #[error("cache key serialization failed: {0}")]
    Key(String),
    /// Filesystem I/O failure.
    #[error("cache I/O at {path}: {source}")]
    Io {
        /// Path involved.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: io::Error,
    },
}

/// Inputs that uniquely identify a cached LLM response.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CacheKeyInput<'a> {
    /// Full prompt text (or rendered template).
    pub prompt: &'a str,
    /// Model identifier.
    pub model: &'a str,
    /// Provider identifier (e.g. `openai`, `anthropic`).
    pub provider: &'a str,
    /// Optional extra dimensions (temperature, tools hash, …) as a stable string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extras: Option<&'a str>,
}

/// Compute `sha256:<hex>` cache key for the given inputs.
///
/// # Errors
///
/// Returns [`CacheError::Key`] if JSON serialization fails.
pub fn cache_key(input: &CacheKeyInput<'_>) -> Result<String, CacheError> {
    // Stable field order via serde_json map sort
    let value = serde_json::json!({
        "extras": input.extras,
        "model": input.model,
        "prompt": input.prompt,
        "provider": input.provider,
    });
    // Force sorted keys
    let s = serde_json::to_string(&value).map_err(|e| CacheError::Key(e.to_string()))?;
    let digest = Sha256::digest(s.as_bytes());
    Ok(format!("sha256:{}", hex::encode(digest)))
}

/// Filesystem-backed response cache.
#[derive(Clone, Debug)]
pub struct DiskCache {
    /// Root directory for cache entries.
    pub root: PathBuf,
}

impl DiskCache {
    /// Create a cache rooted at `root` (created on first put if missing).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn entry_path(&self, key: &str) -> PathBuf {
        // Avoid path separators from the key; strip prefix and use flat hex name.
        let name = key.strip_prefix("sha256:").unwrap_or(key);
        self.root.join(format!("{name}.json"))
    }

    /// Look up a cached response body by key.
    ///
    /// # Errors
    ///
    /// I/O errors other than not-found. Missing entries return `Ok(None)`.
    pub fn get(&self, key: &str) -> Result<Option<String>, CacheError> {
        let path = self.entry_path(key);
        match fs::read_to_string(&path) {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(CacheError::Io { path, source }),
        }
    }

    /// Store a response body under `key`.
    ///
    /// # Errors
    ///
    /// Filesystem failures creating the root or writing the entry.
    pub fn put(&self, key: &str, body: &str) -> Result<(), CacheError> {
        fs::create_dir_all(&self.root).map_err(|source| CacheError::Io {
            path: self.root.clone(),
            source,
        })?;
        let path = self.entry_path(key);
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, body).map_err(|source| CacheError::Io {
            path: tmp.clone(),
            source,
        })?;
        fs::rename(&tmp, &path).map_err(|source| CacheError::Io {
            path: path.clone(),
            source,
        })?;
        Ok(())
    }

    /// Convenience: key then get.
    ///
    /// # Errors
    ///
    /// Key or I/O errors.
    pub fn get_for(&self, input: &CacheKeyInput<'_>) -> Result<Option<String>, CacheError> {
        let key = cache_key(input)?;
        self.get(&key)
    }

    /// Convenience: key then put.
    ///
    /// # Errors
    ///
    /// Key or I/O errors.
    pub fn put_for(&self, input: &CacheKeyInput<'_>, body: &str) -> Result<(), CacheError> {
        let key = cache_key(input)?;
        self.put(&key, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("decon-llm-cache-{n}"))
    }

    #[test]
    fn keys_stable_and_sensitive() {
        let a = CacheKeyInput {
            prompt: "hello",
            model: "m1",
            provider: "p1",
            extras: None,
        };
        let b = CacheKeyInput {
            prompt: "hello",
            model: "m1",
            provider: "p1",
            extras: None,
        };
        assert_eq!(cache_key(&a).unwrap(), cache_key(&b).unwrap());
        assert!(cache_key(&a).unwrap().starts_with("sha256:"));

        let c = CacheKeyInput {
            prompt: "hello!",
            model: "m1",
            provider: "p1",
            extras: None,
        };
        assert_ne!(cache_key(&a).unwrap(), cache_key(&c).unwrap());

        let d = CacheKeyInput {
            prompt: "hello",
            model: "m2",
            provider: "p1",
            extras: None,
        };
        assert_ne!(cache_key(&a).unwrap(), cache_key(&d).unwrap());
    }

    #[test]
    fn put_get_round_trip() {
        let root = temp_root();
        let cache = DiskCache::new(&root);
        let input = CacheKeyInput {
            prompt: "p",
            model: "m",
            provider: "prov",
            extras: Some("t=0"),
        };
        assert!(cache.get_for(&input).unwrap().is_none());
        cache.put_for(&input, r#"{"ok":true}"#).unwrap();
        assert_eq!(
            cache.get_for(&input).unwrap().as_deref(),
            Some(r#"{"ok":true}"#)
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn extras_change_key() {
        let a = CacheKeyInput {
            prompt: "x",
            model: "m",
            provider: "p",
            extras: None,
        };
        let b = CacheKeyInput {
            prompt: "x",
            model: "m",
            provider: "p",
            extras: Some("tools=v1"),
        };
        assert_ne!(cache_key(&a).unwrap(), cache_key(&b).unwrap());
    }
}
