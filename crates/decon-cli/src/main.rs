//! `decon` — deconstruct a codebase into an AI-generated tutorial.
//!
//! This binary only parses arguments and wires up library crates; business
//! logic lives in `decon-core`, `decon-crawl`, and `decon-pipeline`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use decon_core::{DEFAULT_EVAL_PASS_THRESHOLD, ModuleKey, TutorialFile, evaluate_tutorial};
use decon_crawl::crawl_local;
use decon_pipeline::{DryRunError, dry_run};

/// Deconstruct a codebase into an AI-generated tutorial.
#[derive(Parser, Debug)]
#[command(name = "decon", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List relative file inventory under a directory (no LLM).
    Crawl {
        /// Repository root to crawl.
        #[arg(long = "dir", value_name = "PATH")]
        dir: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Build a dry-run plan: crawl + scope + setup + budget (no LLM).
    DryRun {
        /// Repository root.
        #[arg(long = "dir", value_name = "PATH")]
        dir: PathBuf,
        /// Optional app/module scope keys (repeatable), e.g. `apps/alpha`.
        #[arg(long = "apps", value_name = "MODULE")]
        apps: Vec<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Structural eval of a generated tutorial directory (no LLM).
    Eval {
        /// Tutorial output directory (contains index.md and chapters).
        #[arg(long = "out", value_name = "PATH")]
        out: PathBuf,
        /// Pass threshold 0–100 (default 70).
        #[arg(long, default_value_t = DEFAULT_EVAL_PASS_THRESHOLD)]
        threshold: i32,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Crawl { dir, format } => cmd_crawl(&dir, format),
        Commands::DryRun { dir, apps, format } => cmd_dry_run(&dir, &apps, format),
        Commands::Eval {
            out,
            threshold,
            format,
        } => cmd_eval(&out, threshold, format),
    }
}

fn cmd_crawl(dir: &Path, format: OutputFormat) -> ExitCode {
    match crawl_local(dir) {
        Ok(result) => {
            match format {
                OutputFormat::Text => {
                    println!("files: {}", result.files.len());
                    for f in &result.files {
                        println!("{f}");
                    }
                }
                OutputFormat::Json => {
                    let v = serde_json::json!({
                        "file_count": result.files.len(),
                        "files": result.files,
                    });
                    println!("{v}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: crawl failed: {e}");
            ExitCode::from(1)
        }
    }
}

fn cmd_dry_run(dir: &Path, apps: &[String], format: OutputFormat) -> ExitCode {
    let scope: Option<Vec<ModuleKey>> = if apps.is_empty() {
        None
    } else {
        Some(apps.iter().map(ModuleKey::new).collect())
    };
    let scope_ref = scope.as_deref();
    match dry_run(dir, scope_ref) {
        Ok(plan) => {
            match format {
                OutputFormat::Text => {
                    println!("root: {}", plan.root.display());
                    println!("files: {}", plan.files.len());
                    println!("modules: {}", plan.modules.len());
                    println!(
                        "filter: filtered={} before={} after={}",
                        plan.filter_stats.filtered,
                        plan.filter_stats.before,
                        plan.filter_stats.after
                    );
                    println!(
                        "setup: score={} needs_setup_guide={}",
                        plan.setup.score, plan.setup.needs_setup_guide
                    );
                    println!(
                        "budget: files={} raw_chars={} batches={} tokens≈{}",
                        plan.budget.file_count,
                        plan.budget.raw_chars,
                        plan.budget.batch_count,
                        plan.budget.token_estimate
                    );
                }
                OutputFormat::Json => {
                    let modules: serde_json::Map<String, serde_json::Value> = plan
                        .modules
                        .iter()
                        .map(|m| (m.key.as_str().to_owned(), serde_json::json!(m.count)))
                        .collect();
                    let v = serde_json::json!({
                        "root": plan.root.to_string_lossy(),
                        "files": plan.files,
                        "modules": modules,
                        "filter_stats": {
                            "filtered": plan.filter_stats.filtered,
                            "before": plan.filter_stats.before,
                            "after": plan.filter_stats.after,
                            "kept_shared": plan.filter_stats.kept_shared,
                            "module_keys": plan.filter_stats.module_keys.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
                        },
                        "setup": {
                            "needs_setup_guide": plan.setup.needs_setup_guide,
                            "score": plan.setup.score,
                            "gaps": plan.setup.gaps,
                            "config_files": plan.setup.config_files,
                        },
                        "budget": {
                            "file_count": plan.budget.file_count,
                            "module_count": plan.budget.module_count,
                            "raw_chars": plan.budget.raw_chars,
                            "budgeted_chars": plan.budget.budgeted_chars,
                            "token_estimate": plan.budget.token_estimate,
                            "batch_count": plan.budget.batch_count,
                        },
                    });
                    println!("{v}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: dry-run failed: {e}");
            let code = match e {
                DryRunError::Crawl(_) => 1,
                DryRunError::Io { .. } | DryRunError::FileSizeOverflow { .. } => 2,
            };
            ExitCode::from(code)
        }
    }
}

fn cmd_eval(out: &Path, threshold: i32, format: OutputFormat) -> ExitCode {
    let files = match load_tutorial_markdown(out) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: eval failed to load tutorial: {e}");
            return ExitCode::from(1);
        }
    };
    let report = evaluate_tutorial(&files, threshold);
    match format {
        OutputFormat::Text => {
            println!(
                "score={} passed={} threshold={}",
                report.score, report.passed, report.threshold
            );
            for r in &report.reasons {
                println!("- {r}");
            }
        }
        OutputFormat::Json => {
            let v = serde_json::json!({
                "score": report.score,
                "passed": report.passed,
                "threshold": report.threshold,
                "reasons": report.reasons,
                "checks": {
                    "has_index": report.checks.has_index,
                    "index_has_mermaid": report.checks.index_has_mermaid,
                    "has_setup_or_overview": report.checks.has_setup_or_overview,
                    "mermaid_block_count": report.checks.mermaid_block_count,
                    "mermaid_valid_count": report.checks.mermaid_valid_count,
                    "has_path_citations": report.checks.has_path_citations,
                    "has_evidence_footer": report.checks.has_evidence_footer,
                    "links_resolved": report.checks.links_resolved,
                    "links_total": report.checks.links_total,
                },
            });
            println!("{v}");
        }
    }
    if report.passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn load_tutorial_markdown(root: &Path) -> Result<Vec<TutorialFile>, String> {
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }
    let mut out = Vec::new();
    walk_md(root, root, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk_md(dir: &Path, root: &Path, out: &mut Vec<TutorialFile>) -> Result<(), String> {
    let rd = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for ent in rd {
        let ent = ent.map_err(|e| format!("dir entry: {e}"))?;
        let path = ent.path();
        if path.is_dir() {
            walk_md(&path, root, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let content =
                fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
            let rel = path
                .strip_prefix(root)
                .map_err(|_| format!("strip prefix for {}", path.display()))?
                .to_string_lossy()
                .replace('\\', "/");
            out.push(TutorialFile { path: rel, content });
        }
    }
    Ok(())
}
