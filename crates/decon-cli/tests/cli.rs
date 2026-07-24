//! Smoke tests for the `decon` binary's argument parsing and M1 subcommands.
//!
//! These exercise process-boundary behavior (exit code, `--help`, subcommand
//! contracts). Pipeline logic is unit-tested in library crates.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn decon() -> Command {
    Command::cargo_bin("decon").expect("decon binary should build")
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

#[test]
fn version_flag_prints_version_and_exits_zero() {
    decon()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_lists_subcommands() {
    decon()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("crawl"))
        .stdout(predicate::str::contains("dry-run"))
        .stdout(predicate::str::contains("eval"))
        .stdout(predicate::str::contains("resume"))
        .stdout(predicate::str::contains("init"));
}

#[test]
fn unknown_flag_exits_nonzero() {
    decon().arg("--not-a-real-flag").assert().failure();
}

#[test]
fn crawl_python_lib_text() {
    let dir = fixtures_dir().join("python-lib");
    decon()
        .args(["crawl", "--dir"])
        .arg(&dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("files:"))
        .stdout(predicate::str::contains("README.md"));
}

#[test]
fn crawl_json_has_file_count() {
    let dir = fixtures_dir().join("python-lib");
    decon()
        .args(["crawl", "--dir"])
        .arg(&dir)
        .args(["--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"file_count\""))
        .stdout(predicate::str::contains("\"files\""));
}

#[test]
fn dry_run_json_on_fixture() {
    let dir = fixtures_dir().join("python-lib");
    decon()
        .args(["dry-run", "--dir"])
        .arg(&dir)
        .args(["--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"filter_stats\""))
        .stdout(predicate::str::contains("\"budget\""))
        .stdout(predicate::str::contains("\"setup\""));
}

#[test]
fn dry_run_with_apps_scope() {
    let dir = fixtures_dir().join("umbrella");
    decon()
        .args(["dry-run", "--dir"])
        .arg(&dir)
        .args(["--apps", "apps/alpha", "--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"filtered\":true"));
}

#[test]
fn eval_good_mini_exits_zero() {
    let dir = fixtures_dir().join("tutorials/good-mini");
    decon()
        .args(["eval", "--out"])
        .arg(&dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("passed=true"));
}

#[test]
fn eval_broken_mini_exits_nonzero() {
    let dir = fixtures_dir().join("tutorials/broken-mini");
    decon()
        .args(["eval", "--out"])
        .arg(&dir)
        .assert()
        .failure()
        .stdout(predicate::str::contains("passed=false"));
}

#[test]
fn init_writes_decon_toml() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("decon-cli-init-{n}"));
    std::fs::create_dir_all(&dir).unwrap();
    decon()
        .args(["init", "--dir"])
        .arg(&dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("wrote"));
    assert!(dir.join("decon.toml").is_file());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn resume_missing_checkpoint_exits_config() {
    decon()
        .args(["resume", "--checkpoint", "/no/such/checkpoint-dir-decon"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn resume_valid_checkpoint_json() {
    use decon_core::{CheckpointV1, RunConfig, StageId};
    use decon_pipeline::{CheckpointStore, records_from_files};
    use std::time::{SystemTime, UNIX_EPOCH};

    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("decon-cli-resume-{n}"));
    std::fs::create_dir_all(&dir).unwrap();
    let cfg = RunConfig::default();
    let mut meta = CheckpointV1::new(
        &cfg,
        cfg.redacted_for_checkpoint(),
        ".",
        "2026-07-24T00:00:00Z",
    )
    .unwrap();
    meta.mark_stage_complete(StageId::Fetch, "2026-07-24T00:01:00Z");
    let files = records_from_files(&[("a.txt", b"hi" as &[u8])]);
    CheckpointStore::new(&dir).save(meta, &files).unwrap();

    decon()
        .args(["resume", "--checkpoint"])
        .arg(&dir)
        .args(["--format", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"next_stage\""))
        .stdout(predicate::str::contains("dry_run"));

    let _ = std::fs::remove_dir_all(&dir);
}
