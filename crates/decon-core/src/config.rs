//! Run configuration and layered loading.
//!
//! Precedence (highest wins): **CLI overrides** → **project file**
//! (`decon.toml` / `.decon.yaml`) → **environment** (`DECON_*`) → **defaults**.
//!
//! Blank environment values are ignored so exporting empty vars never
//! accidentally clears defaults (move-to-rust config rules).
//!
//! File and env loaders are pure with respect to the process: callers pass
//! strings / key-value maps. Filesystem discovery stays in CLI/pipeline.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;

/// Errors while parsing or serializing configuration layers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    /// TOML body is invalid.
    #[error("invalid decon.toml: {0}")]
    Toml(String),
    /// YAML body is invalid.
    #[error("invalid .decon.yaml: {0}")]
    Yaml(String),
    /// Canonical JSON serialization failed.
    #[error("config JSON serialization failed: {0}")]
    Json(String),
    /// Environment variable value is present but not parseable.
    #[error("invalid value for {key}: {value:?}")]
    InvalidEnvValue {
        /// Environment variable name.
        key: String,
        /// Raw value that failed to parse.
        value: String,
    },
}

/// Default max LLM calls before the budget tracker fails closed.
pub const DEFAULT_MAX_LLM_CALLS: u32 = 200;

/// Default rough chars-per-token heuristic (matches budget defaults).
pub const DEFAULT_CONFIG_CHARS_PER_TOKEN: usize = 4;

/// Full run configuration used by dry-run, generate, and checkpoint hashing.
///
/// Optional fields use `None` to mean “unset at this layer” during merge;
/// after [`resolve_config`] / [`RunConfig::default`] every operational field
/// is populated.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RunConfig {
    /// Repository root or source path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<PathBuf>,
    /// Output directory for generated tutorials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<PathBuf>,
    /// Optional monorepo app/module scope keys (`apps/alpha`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apps: Option<Vec<String>>,
    /// Tutorial language / chrome locale (e.g. `en`, `es`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Hard ceiling on LLM calls for a run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_llm_calls: Option<u32>,
    /// LLM provider id (structure only in M2; no live clients).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Model id for the provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Disk cache directory for LLM responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,
    /// Checkpoint directory (when set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_dir: Option<PathBuf>,
    /// Soft per-batch character budget override for dry-run packing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_char_budget: Option<usize>,
    /// Chars-per-token heuristic for token estimates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chars_per_token: Option<usize>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            root: Some(PathBuf::from(".")),
            output: Some(PathBuf::from("output")),
            apps: Some(Vec::new()),
            language: Some("en".into()),
            max_llm_calls: Some(DEFAULT_MAX_LLM_CALLS),
            provider: None,
            model: None,
            cache_dir: None,
            checkpoint_dir: None,
            batch_char_budget: None,
            chars_per_token: Some(DEFAULT_CONFIG_CHARS_PER_TOKEN),
        }
    }
}

impl RunConfig {
    /// Empty layer (all fields unset) for building overlays.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            root: None,
            output: None,
            apps: None,
            language: None,
            max_llm_calls: None,
            provider: None,
            model: None,
            cache_dir: None,
            checkpoint_dir: None,
            batch_char_budget: None,
            chars_per_token: None,
        }
    }

    /// Merge `overlay` on top of `self`: only `Some` fields in `overlay` win.
    #[must_use]
    pub fn merge_layer(&self, overlay: &Self) -> Self {
        Self {
            root: overlay.root.clone().or_else(|| self.root.clone()),
            output: overlay.output.clone().or_else(|| self.output.clone()),
            apps: overlay.apps.clone().or_else(|| self.apps.clone()),
            language: overlay.language.clone().or_else(|| self.language.clone()),
            max_llm_calls: overlay.max_llm_calls.or(self.max_llm_calls),
            provider: overlay.provider.clone().or_else(|| self.provider.clone()),
            model: overlay.model.clone().or_else(|| self.model.clone()),
            cache_dir: overlay.cache_dir.clone().or_else(|| self.cache_dir.clone()),
            checkpoint_dir: overlay
                .checkpoint_dir
                .clone()
                .or_else(|| self.checkpoint_dir.clone()),
            batch_char_budget: overlay.batch_char_budget.or(self.batch_char_budget),
            chars_per_token: overlay.chars_per_token.or(self.chars_per_token),
        }
    }

    /// Copy suitable for checkpoint display: drop secret-bearing fields if any
    /// are added later. Today this is a clone with provider/model kept (not secrets).
    #[must_use]
    pub fn redacted_for_checkpoint(&self) -> Self {
        self.clone()
    }
}

/// Resolve full config by merging layers in order:
/// defaults, then `env_layer`, then `file_layer`, then `cli_layer`.
///
/// **CLI** overrides file; **file** overrides env; **env** overrides defaults.
#[must_use]
pub fn resolve_config(
    env_layer: &RunConfig,
    file_layer: &RunConfig,
    cli_layer: &RunConfig,
) -> RunConfig {
    RunConfig::default()
        .merge_layer(env_layer)
        .merge_layer(file_layer)
        .merge_layer(cli_layer)
}

/// Parse a TOML document into a config layer (`decon.toml` body).
///
/// # Errors
///
/// Returns a message when TOML is invalid or types do not match.
pub fn parse_toml_config(text: &str) -> Result<RunConfig, ConfigError> {
    toml::from_str(text).map_err(|e| ConfigError::Toml(e.to_string()))
}

/// Parse a YAML document into a config layer (`.decon.yaml` body).
///
/// # Errors
///
/// Returns a message when YAML is invalid or types do not match.
pub fn parse_yaml_config(text: &str) -> Result<RunConfig, ConfigError> {
    serde_yml::from_str(text).map_err(|e| ConfigError::Yaml(e.to_string()))
}

/// Load a config layer from environment-style key/value pairs.
///
/// Recognized keys (case-sensitive):
/// - `DECON_ROOT`, `DECON_OUTPUT`, `DECON_APPS` (comma-separated),
/// - `DECON_LANGUAGE`, `DECON_MAX_LLM_CALLS`, `DECON_PROVIDER`, `DECON_MODEL`,
/// - `DECON_CACHE_DIR`, `DECON_CHECKPOINT_DIR`,
/// - `DECON_BATCH_CHAR_BUDGET`, `DECON_CHARS_PER_TOKEN`
///
/// **Blank values are ignored** (treated as unset). Non-blank values that fail
/// numeric parse return [`ConfigError::InvalidEnvValue`].
///
/// # Errors
///
/// Returns [`ConfigError::InvalidEnvValue`] when a numeric env var is set but
/// not a valid integer.
pub fn config_from_env_map(vars: &BTreeMap<String, String>) -> Result<RunConfig, ConfigError> {
    let mut cfg = RunConfig::empty();
    if let Some(v) = nonblank(vars.get("DECON_ROOT")) {
        cfg.root = Some(PathBuf::from(v));
    }
    if let Some(v) = nonblank(vars.get("DECON_OUTPUT")) {
        cfg.output = Some(PathBuf::from(v));
    }
    if let Some(v) = nonblank(vars.get("DECON_APPS")) {
        let apps: Vec<String> = v
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        cfg.apps = Some(apps);
    }
    if let Some(v) = nonblank(vars.get("DECON_LANGUAGE")) {
        cfg.language = Some(v.to_owned());
    }
    if let Some(v) = nonblank(vars.get("DECON_MAX_LLM_CALLS")) {
        cfg.max_llm_calls = Some(parse_env_u32("DECON_MAX_LLM_CALLS", v)?);
    }
    if let Some(v) = nonblank(vars.get("DECON_PROVIDER")) {
        cfg.provider = Some(v.to_owned());
    }
    if let Some(v) = nonblank(vars.get("DECON_MODEL")) {
        cfg.model = Some(v.to_owned());
    }
    if let Some(v) = nonblank(vars.get("DECON_CACHE_DIR")) {
        cfg.cache_dir = Some(PathBuf::from(v));
    }
    if let Some(v) = nonblank(vars.get("DECON_CHECKPOINT_DIR")) {
        cfg.checkpoint_dir = Some(PathBuf::from(v));
    }
    if let Some(v) = nonblank(vars.get("DECON_BATCH_CHAR_BUDGET")) {
        cfg.batch_char_budget = Some(parse_env_usize("DECON_BATCH_CHAR_BUDGET", v)?);
    }
    if let Some(v) = nonblank(vars.get("DECON_CHARS_PER_TOKEN")) {
        cfg.chars_per_token = Some(parse_env_usize("DECON_CHARS_PER_TOKEN", v)?);
    }
    Ok(cfg)
}

fn parse_env_u32(key: &str, value: &str) -> Result<u32, ConfigError> {
    value
        .parse::<u32>()
        .map_err(|_| ConfigError::InvalidEnvValue {
            key: key.to_owned(),
            value: value.to_owned(),
        })
}

fn parse_env_usize(key: &str, value: &str) -> Result<usize, ConfigError> {
    value
        .parse::<usize>()
        .map_err(|_| ConfigError::InvalidEnvValue {
            key: key.to_owned(),
            value: value.to_owned(),
        })
}

/// Canonical JSON for hashing (sorted keys, no insignificant whitespace).
///
/// Used by checkpoint `config_hash` (ADR 0001). Serializes the full config
/// including optional fields that are set.
///
/// # Errors
///
/// Returns an error if serialization fails (should not happen for `RunConfig`).
pub fn canonical_config_json(config: &RunConfig) -> Result<String, ConfigError> {
    let value = serde_json::to_value(config).map_err(|e| ConfigError::Json(e.to_string()))?;
    let normalized = sort_json_value(value);
    serde_json::to_string(&normalized).map_err(|e| ConfigError::Json(e.to_string()))
}

fn sort_json_value(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().cloned().collect();
            keys.sort();
            let mut out = serde_json::Map::new();
            for k in keys {
                if let Some(val) = map.get(&k) {
                    out.insert(k, sort_json_value(val.clone()));
                }
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(sort_json_value).collect())
        }
        other => other,
    }
}

fn nonblank(v: Option<&String>) -> Option<&str> {
    v.map(String::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_populated() {
        let d = RunConfig::default();
        assert_eq!(d.root.as_deref(), Some(std::path::Path::new(".")));
        assert_eq!(d.max_llm_calls, Some(DEFAULT_MAX_LLM_CALLS));
        assert_eq!(d.language.as_deref(), Some("en"));
    }

    #[test]
    fn precedence_cli_beats_file_beats_env_beats_defaults() {
        let env = RunConfig {
            language: Some("fr".into()),
            max_llm_calls: Some(10),
            ..RunConfig::empty()
        };

        let file = RunConfig {
            language: Some("es".into()),
            provider: Some("openai".into()),
            ..RunConfig::empty()
        };

        let cli = RunConfig {
            language: Some("de".into()),
            ..RunConfig::empty()
        };

        let resolved = resolve_config(&env, &file, &cli);
        assert_eq!(resolved.language.as_deref(), Some("de")); // CLI
        assert_eq!(resolved.provider.as_deref(), Some("openai")); // file
        assert_eq!(resolved.max_llm_calls, Some(10)); // env
        assert_eq!(resolved.root.as_deref(), Some(std::path::Path::new("."))); // default
    }

    #[test]
    fn blank_env_does_not_override_defaults() {
        let mut vars = BTreeMap::new();
        vars.insert("DECON_LANGUAGE".into(), "   ".into());
        vars.insert("DECON_MAX_LLM_CALLS".into(), "".into());
        vars.insert("DECON_ROOT".into(), "/tmp/repo".into());
        let env = config_from_env_map(&vars).expect("env map");
        let resolved = resolve_config(&env, &RunConfig::empty(), &RunConfig::empty());
        assert_eq!(resolved.language.as_deref(), Some("en")); // default, blank ignored
        assert_eq!(resolved.max_llm_calls, Some(DEFAULT_MAX_LLM_CALLS));
        assert_eq!(
            resolved.root.as_deref(),
            Some(std::path::Path::new("/tmp/repo"))
        );
    }

    #[test]
    fn parse_toml_and_yaml_layers() {
        let toml_cfg = parse_toml_config(
            r#"
language = "es"
max_llm_calls = 42
apps = ["apps/alpha", "apps/beta"]
"#,
        )
        .expect("toml");
        assert_eq!(toml_cfg.language.as_deref(), Some("es"));
        assert_eq!(toml_cfg.max_llm_calls, Some(42));
        assert_eq!(
            toml_cfg.apps.as_deref(),
            Some(["apps/alpha".to_owned(), "apps/beta".to_owned()].as_slice())
        );

        let yaml_cfg = parse_yaml_config(
            r#"
language: fr
provider: anthropic
"#,
        )
        .expect("yaml");
        assert_eq!(yaml_cfg.language.as_deref(), Some("fr"));
        assert_eq!(yaml_cfg.provider.as_deref(), Some("anthropic"));
    }

    #[test]
    fn env_apps_comma_separated() {
        let mut vars = BTreeMap::new();
        vars.insert("DECON_APPS".into(), "apps/a, apps/b ,".into());
        let env = config_from_env_map(&vars).expect("env map");
        assert_eq!(
            env.apps.as_deref(),
            Some(["apps/a".to_owned(), "apps/b".to_owned()].as_slice())
        );
    }

    #[test]
    fn canonical_json_is_stable_and_key_ordered() {
        let a = RunConfig {
            provider: Some("x".into()),
            ..RunConfig::default()
        };
        let b = RunConfig {
            provider: Some("x".into()),
            ..RunConfig::default()
        };
        let ja = canonical_config_json(&a).unwrap();
        let jb = canonical_config_json(&b).unwrap();
        assert_eq!(ja, jb);
        // keys sorted: apps before language, etc.
        assert!(ja.find("apps").unwrap() < ja.find("language").unwrap());

        let b2 = RunConfig {
            provider: Some("y".into()),
            ..RunConfig::default()
        };
        assert_ne!(
            canonical_config_json(&a).unwrap(),
            canonical_config_json(&b2).unwrap()
        );
    }

    #[test]
    fn invalid_toml_errors() {
        let err = parse_toml_config("language = [").unwrap_err();
        assert!(
            matches!(err, ConfigError::Toml(_)),
            "expected Toml error, got {err}"
        );
    }

    #[test]
    fn invalid_env_numeric_errors() {
        let mut vars = BTreeMap::new();
        vars.insert("DECON_MAX_LLM_CALLS".into(), "abc".into());
        let err = config_from_env_map(&vars).unwrap_err();
        assert!(
            matches!(
                err,
                ConfigError::InvalidEnvValue { ref key, .. } if key == "DECON_MAX_LLM_CALLS"
            ),
            "got {err}"
        );
    }

    #[test]
    fn invalid_yaml_errors() {
        let err = parse_yaml_config("language: [").unwrap_err();
        assert!(
            matches!(err, ConfigError::Yaml(_)),
            "expected Yaml error, got {err}"
        );
    }
}
