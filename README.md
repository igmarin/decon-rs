# decon-rs
Deconstruct massive code monoliths into structured, AI-generated tutorials using high-speed, local-first Rust architecture.

## Code review automation

Every pull request is reviewed automatically by [rs-guard](https://github.com/nebulaideas/rs-guard),
an AI code review CLI, using the decon-rs-specific prompt in
[`.github/review-prompt.md`](.github/review-prompt.md):

- **CI:** [`.github/workflows/rs-guard-review.yml`](.github/workflows/rs-guard-review.yml) posts
  an APPROVE / REQUEST_CHANGES / COMMENT review on every non-draft PR. Requires a `DEEPSEEK_API_KEY`
  repository secret (provider is selected via `RS_GUARD_PROVIDER=deepseek` — rs-guard's `--provider`
  CLI flag is not wired into its main review pipeline, only into its scaffold subcommands).
- **Local (optional):** run `./scripts/install-hooks.sh` once to install a pre-commit hook that
  reviews staged changes before they're committed (bypass with `git commit --no-verify`). Requires
  `rs-guard` on `PATH` (`cargo install rs-guard`) and `DEEPSEEK_API_KEY` set in your environment or
  in `~/.config/rs-guard/env`.
