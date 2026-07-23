# Contributing to `decon`

Thanks for helping make `decon` a fast, reliable tool for turning code monoliths
into structured tutorials. This guide covers how to build, test, and land
changes in the workspace.

## Quick start

You need a recent Rust toolchain. The workspace declares `rust-version = "1.85"`.

```bash
# Clone and enter the workspace
git clone https://github.com/igmarin/decon-rs.git
cd decon-rs

# Build the whole workspace
cargo build --workspace

# Run the CLI (currently only --help / --version)
cargo run -p decon-cli -- --help
```

## Development workflow

We use a test-driven workflow for every behavior change, bug fix, or new
helper.

1. **Write a failing test** that expresses the contract or reproduces the bug.
2. **Run the test** and confirm it fails for the *right* reason.
3. **Implement the smallest change** that makes the test pass.
4. **Run the test again** and confirm it passes.
5. **Refactor** with the test suite still green.
6. Add edge cases or property tests only after the happy path is locked.

For library code this is non-negotiable. For CLI-only plumbing, still add an
integration test where the contract is observable (argument parsing, exit codes,
JSON dry-run shape, etc.).

## Running checks

The CI pipeline runs the following on every PR. Run them locally before pushing:

```bash
# Formatting
cargo fmt --all -- --check

# Clippy with warnings-as-errors
cargo clippy --workspace --all-targets -- -D warnings

# Tests
cargo test --workspace

# Docs (also enforces missing rustdoc on public items)
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Security audit
cargo audit
```

## Coverage gate

The project targets **≥ 85% line coverage** for the workspace and **≥ 90%**
for `decon-core` once it contains real domain logic. We use
`cargo-llvm-cov`:

```bash
# Install cargo-llvm-cov (once)
cargo install cargo-llvm-cov

# Generate and view a summary
cargo llvm-cov --workspace --lcov --output-path target/lcov.info
cargo llvm-cov report --summary-only
```

Coverage is report-only in Milestone 0. The hard gate becomes active in
Milestone 2, when the core domain layer has enough logic to make the number
meaningful.

## Code conventions

- **Library crates** (`decon-core`, `decon-crawl`, `decon-llm`,
  `decon-pipeline`) perform no CLI or main-binary logic and stay easy to unit
  test.
- **Public APIs must have rustdoc.** Each library crate declares
  `#![deny(missing_docs)]`.
- Prefer typed errors with `thiserror` in libraries; user-facing messages and
  exit codes live in `decon-cli`.
- Use `clap` derive for CLI flags. Keep the binary a thin wrapper around the
  pipeline crates.
- All GitHub Actions we use are pinned by full commit SHA.

## Crate layout

```text
crates/
  decon-core/      pure domain: models, traits, mermaid, budgeting
  decon-crawl/     local + GitHub fetching
  decon-llm/       provider clients, retries, caching
  decon-pipeline/  stage orchestration, checkpoint/resume
  decon-cli/       clap binary and exit codes
```

Add new crates under `crates/` and register them in the root `Cargo.toml`
workspace `members` list.

## Documentation

- User-facing docs live in `README.md`, `docs/best-practices.md`, and
  `docs/move-to-rust.md`.
- Architecture decisions are recorded in `docs/adr/` when they affect the
  checkpoint format, crate boundaries, or provider contract.
- Every public function, type, and module should have a rustdoc comment.

## Pull requests

1. Create a feature branch from `main`: `feature/#-short-name` (e.g.
   `feature/3-contributing-guide`).
2. Make focused, incremental commits.
3. Ensure `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`,
   and `cargo audit` pass locally.
4. Open a PR that references the issue number (e.g. `Closes #3`).
5. Wait for CI and any automated `rs-guard` review.
6. Merge only when CI is green.

## Questions?

Open an issue or discussion on [GitHub](https://github.com/igmarin/decon-rs).
