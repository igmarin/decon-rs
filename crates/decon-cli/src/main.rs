//! `decon` — deconstruct a codebase into an AI-generated tutorial.
//!
//! This binary only parses arguments and wires up the pipeline; business
//! logic lives in `decon-core`, `decon-crawl`, `decon-llm`, and
//! `decon-pipeline` so it stays testable without a CLI harness. Subcommands
//! (`dry-run`, `generate`, `resume`, `each-app`, `eval`, `providers`) land in
//! later milestones — see `docs/move-to-rust.md` §4.2.

use clap::Parser;

/// Deconstruct a codebase into an AI-generated tutorial.
#[derive(Parser, Debug)]
#[command(name = "decon", version, about, long_about = None)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
