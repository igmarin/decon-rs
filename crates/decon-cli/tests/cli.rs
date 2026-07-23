//! Smoke tests for the `decon` binary's argument parsing.
//!
//! These only exercise process-boundary behavior (exit code, `--version` /
//! `--help` output); pipeline logic itself is tested in `decon-core`,
//! `decon-crawl`, `decon-llm`, and `decon-pipeline` without a process
//! boundary.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_flag_prints_version_and_exits_zero() {
    Command::cargo_bin("decon")
        .expect("decon binary should build")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn help_flag_exits_zero() {
    Command::cargo_bin("decon")
        .expect("decon binary should build")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn unknown_flag_exits_nonzero() {
    Command::cargo_bin("decon")
        .expect("decon binary should build")
        .arg("--not-a-real-flag")
        .assert()
        .failure();
}
