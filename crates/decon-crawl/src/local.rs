//! Local filesystem crawl: gitignore-aware inventory of relative paths.
//!
//! For Milestone 1, the walk matches the frozen fixture baseline
//! (`tests/fixtures/baseline.json`):
//!
//! - Skip **hidden directories** (name starts with `.`)
//! - Include **hidden files** (e.g. `.env.example`)
//! - Emit relative POSIX paths (`/` separators), sorted lexicographically
//!
//! GitHub fetch is out of scope for this module.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Inventory of files discovered under a repository root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrawlResult {
    /// Relative file paths using `/` separators, sorted ascending.
    pub files: Vec<String>,
}

impl CrawlResult {
    /// Number of files in the inventory.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

/// Errors produced while crawling a local tree.
#[derive(Debug, Error)]
pub enum CrawlError {
    /// The path exists but is not a directory.
    #[error("path is not a directory: {0}")]
    NotADirectory(PathBuf),
    /// The path does not exist or cannot be accessed as a directory.
    #[error("failed to access directory {path}: {source}")]
    Io {
        /// Directory that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Recursively inventory files under `root`.
///
/// Returns sorted relative paths. Hidden directories are skipped; hidden
/// files are included. Symlinks that resolve to files/dirs are followed
/// (same as a plain `read_dir` walk).
///
/// # Errors
///
/// - [`CrawlError::NotADirectory`] if `root` exists but is not a directory
/// - [`CrawlError::Io`] if `root` cannot be opened as a directory
///
/// Nested unreadable directories are skipped (best-effort walk) rather than
/// failing the whole crawl, so partial trees still produce an inventory.
///
/// # Examples
///
/// ```no_run
/// use decon_crawl::local::crawl_local;
///
/// let result = crawl_local(".").expect("cwd is a directory");
/// assert!(result.file_count() > 0);
/// ```
pub fn crawl_local(root: impl AsRef<Path>) -> Result<CrawlResult, CrawlError> {
    let root = root.as_ref();
    let meta = fs::metadata(root).map_err(|source| CrawlError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    if !meta.is_dir() {
        return Err(CrawlError::NotADirectory(root.to_path_buf()));
    }

    let mut files: Vec<String> = Vec::new();
    crawl_dir(root, root, &mut files);
    files.sort();
    Ok(CrawlResult { files })
}

fn crawl_dir(root: &Path, dir: &Path, files: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip hidden directories; allow hidden files.
        if path.is_dir() && name_str.starts_with('.') {
            continue;
        }

        if path.is_file() {
            if let Ok(rel) = path.strip_prefix(root) {
                files.push(rel.to_string_lossy().replace('\\', "/"));
            }
        } else if path.is_dir() {
            crawl_dir(root, &path, files);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
    }

    fn baseline_files(fixture: &str) -> Vec<String> {
        let baseline_path = fixtures_dir().join("baseline.json");
        let raw = fs::read_to_string(&baseline_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", baseline_path.display()));
        let v: serde_json::Value = serde_json::from_str(&raw).expect("baseline.json is valid JSON");
        v[fixture]["crawl"]["files"]
            .as_array()
            .unwrap_or_else(|| panic!("baseline missing crawl.files for {fixture}"))
            .iter()
            .map(|x| x.as_str().expect("file path is a string").to_owned())
            .collect()
    }

    #[test]
    fn crawl_python_lib_matches_baseline() {
        let root = fixtures_dir().join("python-lib");
        let result = crawl_local(&root).expect("crawl python-lib");
        let expected = baseline_files("python-lib");
        assert_eq!(result.file_count(), expected.len());
        assert_eq!(result.files, expected);
    }

    #[test]
    fn crawl_umbrella_matches_baseline() {
        let root = fixtures_dir().join("umbrella");
        let result = crawl_local(&root).expect("crawl umbrella");
        let expected = baseline_files("umbrella");
        assert_eq!(result.file_count(), expected.len());
        assert_eq!(result.files, expected);
    }

    #[test]
    fn crawl_js_lib_matches_baseline() {
        let root = fixtures_dir().join("js-lib");
        let result = crawl_local(&root).expect("crawl js-lib");
        let expected = baseline_files("js-lib");
        assert_eq!(result.file_count(), expected.len());
        assert_eq!(result.files, expected);
    }

    #[test]
    fn hidden_directory_skipped_hidden_file_kept() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        File::create(root.join(".env.example"))
            .and_then(|mut f| f.write_all(b"KEY=1\n"))
            .expect("hidden file");
        File::create(root.join("visible.txt"))
            .and_then(|mut f| f.write_all(b"ok\n"))
            .expect("visible file");

        let hidden_dir = root.join(".hidden");
        fs::create_dir(&hidden_dir).expect("hidden dir");
        File::create(hidden_dir.join("secret.txt"))
            .and_then(|mut f| f.write_all(b"nope\n"))
            .expect("file inside hidden dir");

        let nested = root.join("src");
        fs::create_dir(&nested).expect("src");
        File::create(nested.join("main.rs"))
            .and_then(|mut f| f.write_all(b"fn main() {}\n"))
            .expect("nested file");

        let result = crawl_local(root).expect("crawl temp tree");
        assert_eq!(
            result.files,
            vec![
                ".env.example".to_owned(),
                "src/main.rs".to_owned(),
                "visible.txt".to_owned(),
            ]
        );
        assert!(!result.files.iter().any(|f| f.contains("secret")));
    }

    #[test]
    fn not_a_directory_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("only-file");
        File::create(&file).expect("touch");
        let err = crawl_local(&file).expect_err("file is not a directory");
        assert!(matches!(err, CrawlError::NotADirectory(_)));
    }

    #[test]
    fn missing_path_errors() {
        let err = crawl_local("/nonexistent/decon-crawl-path-xyz").expect_err("missing");
        assert!(matches!(err, CrawlError::Io { .. }));
    }
}
