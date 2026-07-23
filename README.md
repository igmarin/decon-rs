# decon-rs
Deconstruct massive code monoliths into structured, AI-generated tutorials using high-speed, local-first Rust architecture.

## Code review automation

Every pull request is reviewed automatically by [rs-guard](https://github.com/nebulaideas/rs-guard),
an AI code review CLI, using the decon-rs-specific prompt in
[`.github/review-prompt.md`](.github/review-prompt.md):

- **CI:** [`.github/workflows/rs-guard-review.yml`](.github/workflows/rs-guard-review.yml) posts
  an APPROVE / REQUEST_CHANGES / COMMENT review on every non-draft PR. Requires an `OPENAI_API_KEY`
  repository secret.
- **Local (optional):** run `./scripts/install-hooks.sh` once to install a pre-commit hook that
  reviews staged changes before they're committed (bypass with `git commit --no-verify`). Requires
  `rs-guard` on `PATH` (`cargo install rs-guard`) and `OPENAI_API_KEY` set in your environment or
  in `~/.config/rs-guard/env`.
