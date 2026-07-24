//! Dry-run plan: crawl + scope + setup assessment + budget (no LLM).
//!
//! Assembles the Milestone 1 plan used by `decon dry-run`. Parity with
//! `tests/fixtures/baseline.json` is enforced by integration tests.

use std::fs;
use std::path::{Path, PathBuf};

use decon_core::{
    BudgetConfig, BudgetEstimate, FileSize, FilterStats, ModuleCount, ModuleKey, SetupAssessment,
    assess_setup, discover_modules, estimate_budget, filter_files_by_scope, module_key,
    unscoped_filter_stats,
};
use decon_crawl::{CrawlError, crawl_local};
use thiserror::Error;

/// Full dry-run plan for a repository root (zero LLM calls).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DryRunPlan {
    /// Absolute root that was crawled.
    pub root: PathBuf,
    /// Relative file inventory after optional scope filter (POSIX `/`).
    pub files: Vec<String>,
    /// Module inventory from the **unscoped** crawl (baseline `modules` map).
    ///
    /// Intentionally not re-scoped: baseline and setup assessment always see
    /// the full-repo module layout; only [`Self::files`] / [`Self::budget`]
    /// reflect `--apps` filtering.
    pub modules: Vec<ModuleCount>,
    /// Filter statistics for this run (unscoped or scoped).
    pub filter_stats: FilterStats,
    /// Setup-doc assessment (README + full unscoped file list, matching baseline).
    pub setup: SetupAssessment,
    /// Context budget estimate for the **scoped** working set.
    pub budget: BudgetEstimate,
}

/// Errors while building a dry-run plan (crawl failures or file I/O).
///
/// The CLI maps these to non-zero exit codes; library callers should treat
/// them as terminal for the plan assembly step.
#[derive(Debug, Error)]
pub enum DryRunError {
    /// Local crawl failed.
    #[error(transparent)]
    Crawl(#[from] CrawlError),
    /// Failed to read a file under the root (e.g. README for setup scoring).
    #[error("failed to read {path}: {source}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Build a dry-run plan for `root`, optionally scoping to `apps` / modules.
///
/// Steps:
/// 1. [`crawl_local`] — sorted relative inventory  
/// 2. [`discover_modules`] on the full inventory  
/// 3. Optional [`filter_files_by_scope`] (or unscoped stats)  
/// 4. [`assess_setup`] from `README.md` + **full** inventory (parity with baseline)  
/// 5. [`estimate_budget`] on the working (scoped) file set with on-disk sizes  
///
/// # Errors
///
/// Propagates crawl failures and I/O when reading file metadata for budgets
/// or when README.md exists but cannot be read (e.g. permission denied).
/// A **missing** README is treated as empty content (score gaps), not an error.
///
/// # Examples
///
/// ```no_run
/// use decon_pipeline::dry_run::dry_run;
///
/// let plan = dry_run(".", None).expect("dry-run");
/// // Empty repos are valid: zero files and zero batches.
/// assert_eq!(plan.budget.file_count, plan.files.len());
/// if plan.files.is_empty() {
///     assert_eq!(plan.budget.batch_count, 0);
/// } else {
///     assert!(plan.budget.batch_count >= 1);
/// }
/// ```
pub fn dry_run(
    root: impl AsRef<Path>,
    scope: Option<&[ModuleKey]>,
) -> Result<DryRunPlan, DryRunError> {
    dry_run_with_budget(root, scope, &BudgetConfig::default())
}

/// Same as [`dry_run`] with an explicit budget configuration.
///
/// # Errors
///
/// Same as [`dry_run`].
pub fn dry_run_with_budget(
    root: impl AsRef<Path>,
    scope: Option<&[ModuleKey]>,
    budget_config: &BudgetConfig,
) -> Result<DryRunPlan, DryRunError> {
    let root = root.as_ref();
    let crawl = crawl_local(root)?;
    let all_files = crawl.files;
    let modules = discover_modules(all_files.iter().map(String::as_str));

    // Setup always uses the full inventory (baseline parity); evaluate before
    // we possibly move `all_files` into the unscoped working set.
    let readme_path = root.join("README.md");
    // Tolerate a missing README only; other I/O errors (permissions, EISDIR, …)
    // must surface as DryRunError::Io so setup is not silently wrong.
    let readme = match fs::read_to_string(&readme_path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(source) => {
            return Err(DryRunError::Io {
                path: readme_path,
                source,
            });
        }
    };
    let setup = assess_setup(&readme, all_files.iter().map(String::as_str));

    let (files, filter_stats) = match scope {
        None => {
            // Common path: move inventory — no clone.
            let stats = unscoped_filter_stats(all_files.len(), &modules);
            (all_files, stats)
        }
        Some(keys) => {
            let filtered = filter_files_by_scope(all_files.iter().map(String::as_str), keys);
            (filtered.files, filtered.stats)
        }
    };

    let budget = estimate_budget_for_files(root, &files, budget_config)?;

    Ok(DryRunPlan {
        root: root.to_path_buf(),
        files,
        modules,
        filter_stats,
        setup,
        budget,
    })
}

fn estimate_budget_for_files(
    root: &Path,
    files: &[String],
    config: &BudgetConfig,
) -> Result<BudgetEstimate, DryRunError> {
    // TODO(perf): sizes are loaded with one `metadata` syscall per file.
    // For large monorepos, fold size collection into `crawl_local` (or a
    // bulk walk) so dry-run does not re-stat every path.
    let mut sizes: Vec<FileSize> = Vec::with_capacity(files.len());
    for rel in files {
        let full = root.join(rel);
        let chars = match fs::metadata(&full) {
            Ok(m) => usize::try_from(m.len()).unwrap_or(usize::MAX),
            Err(source) => {
                return Err(DryRunError::Io { path: full, source });
            }
        };
        sizes.push(FileSize {
            path: rel.clone(),
            chars,
            module: module_key(rel),
        });
    }
    Ok(estimate_budget(&sizes, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("decon-pipeline-dry-run-{nanos}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn missing_readme_is_tolerated_as_empty() {
        let dir = unique_temp_dir();
        // Empty tree: crawl succeeds; no README.md → empty string, not an error.
        let plan = dry_run(&dir, None).expect("missing README must not fail dry-run");
        assert!(!plan.setup.signals.has_readme);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_readme_returns_io_error() {
        let dir = unique_temp_dir();
        // A directory named README.md makes read_to_string fail with a non-NotFound
        // error (IsADirectory / InvalidInput depending on OS) — not silently empty.
        fs::create_dir(dir.join("README.md")).expect("create README.md as directory");
        let err = dry_run(&dir, None).expect_err("unreadable README must be DryRunError::Io");
        assert!(
            matches!(err, DryRunError::Io { .. }),
            "expected Io error, got: {err}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
