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
    /// Module inventory derived from the **unscoped** crawl (baseline modules map).
    pub modules: Vec<ModuleCount>,
    /// Filter statistics for this run (unscoped or scoped).
    pub filter_stats: FilterStats,
    /// Setup-doc assessment (README + full unscoped file list, matching baseline).
    pub setup: SetupAssessment,
    /// Context budget estimate for the **scoped** working set.
    pub budget: BudgetEstimate,
}

/// Errors while building a dry-run plan.
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
/// Propagates crawl failures and I/O when reading file metadata for budgets.
/// A missing README is treated as empty content (score gaps), not an error.
///
/// # Examples
///
/// ```no_run
/// use decon_pipeline::dry_run::dry_run;
///
/// let plan = dry_run(".", None).expect("dry-run");
/// assert!(plan.files.len() > 0);
/// assert!(plan.budget.batch_count >= 1 || plan.files.is_empty());
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

    let (files, filter_stats) = match scope {
        None => {
            let stats = unscoped_filter_stats(all_files.len(), &modules);
            (all_files.clone(), stats)
        }
        Some(keys) => {
            let filtered = filter_files_by_scope(all_files.iter().map(String::as_str), keys);
            (filtered.files, filtered.stats)
        }
    };

    let readme_path = root.join("README.md");
    let readme = fs::read_to_string(&readme_path).unwrap_or_default();
    let setup = assess_setup(&readme, all_files.iter().map(String::as_str));

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
