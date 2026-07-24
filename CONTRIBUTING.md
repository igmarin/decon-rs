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

# Run the CLI
cargo run -p decon-cli -- --help
cargo run -p decon-cli -- crawl --dir tests/fixtures/python-lib
cargo run -p decon-cli -- dry-run --dir tests/fixtures/umbrella --format json
cargo run -p decon-cli -- eval --out tests/fixtures/tutorials/good-mini
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

# Fixture baseline check (verify baseline.json matches tests/fixtures/)
rustc tests/fixtures/regenerate_baseline.rs -o /tmp/regen_baseline && \
  /tmp/regen_baseline tests/fixtures/ --check
```


## Pre-commit review (rs-guard)

Before committing non-trivial changes on a feature branch:

```bash
git add -A   # stage the change set you intend to commit
rs-guard --prompt-file .github/review-prompt.md
```

Address Critical / Security / Important findings (or document why not), then
commit. PRs also receive an automated rs-guard review from GitHub Actions.


## Domain modules (M1)

| Area | Crate / path | Notes |
|------|----------------|-------|
| Module keys / inventory | `decon-core::module` | Pure |
| Scope filter (`--apps`) | `decon-core::scope` | Pure |
| Setup assessment | `decon-core::setup` | Pure |
| Context budget | `decon-core::budget` | Pure |
| Mermaid sanitize | `decon-core::mermaid` | Pure; table-driven tests |
| Index diagrams | `decon-core::diagrams` | Always sanitize/validate |
| Structural eval | `decon-core::eval` | Fixtures under `tests/fixtures/tutorials/` |
| RunConfig | `decon-core::config` | CLI > file > env > defaults |
| Checkpoint types | `decon-core::checkpoint` | ADR 0001 metadata |
| Progress / LLM budget | `decon-core::progress` | Fail-closed max calls |
| Secrets redaction | `decon-core::secrets` | Paths + content heuristics |
| LLM disk cache | `decon-llm::cache` | No live network |
| Checkpoint store | `decon-pipeline::checkpoint_store` | save/load bundle |
| Resume helpers | `decon-pipeline::resume` | stage-skip / invalidate |
| Local crawl | `decon-crawl::local` | FS I/O |
| Dry-run plan | `decon-pipeline::dry_run` | Orchestration |
| CLI | `decon-cli` | Thin wrappers + `assert_cmd` tests |

## Coverage gate

The project targets **≥ 85% line coverage** on `decon-core` + `decon-crawl`
(M1 exit). Workspace-wide hard fail remains an M2 CI gate. We use
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
