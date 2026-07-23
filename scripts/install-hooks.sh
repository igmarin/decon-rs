#!/usr/bin/env bash
# Install git pre-commit hook for rs-guard code review on decon-rs.
# Usage: ./scripts/install-hooks.sh
#
# rs-guard must be installed separately (cargo install rs-guard, or download a
# release binary from https://github.com/nebulaideas/rs-guard/releases and put
# it on PATH). This script only wires up the git hook.

set -euo pipefail

HOOK_DIR="$(git rev-parse --git-dir)/hooks"
HOOK_FILE="${HOOK_DIR}/pre-commit"

mkdir -p "${HOOK_DIR}"

cat > "${HOOK_FILE}" << 'EOF'
#!/usr/bin/env bash
# rs-guard pre-commit hook for decon-rs.
# Runs rs-guard on staged changes before committing.
# Exit 2 (REQUEST_CHANGES) blocks the commit.
# Exit 0 (APPROVE/COMMENT) allows the commit.
# Bypass: git commit --no-verify

set -euo pipefail

# Load API key from config or environment
if [ -z "${OPENAI_API_KEY:-}" ]; then
  CONFIG_FILE="${HOME}/.config/rs-guard/env"
  if [ -f "${CONFIG_FILE}" ]; then
    # shellcheck source=/dev/null
    source "${CONFIG_FILE}"
  fi
fi

# Check if rs-guard is available
if ! command -v rs-guard &>/dev/null && [ ! -f ./rs-guard ]; then
  echo "[rs-guard] rs-guard not found. Skipping review."
  echo "[rs-guard] Install with: cargo install rs-guard"
  echo "[rs-guard] or download a release from https://github.com/nebulaideas/rs-guard/releases"
  exit 0
fi

RS_GUARD="${RS_GUARD:-$(command -v rs-guard 2>/dev/null || echo ./rs-guard)}"

# Check if API key is available
if [ -z "${OPENAI_API_KEY:-}" ]; then
  echo "[rs-guard] OPENAI_API_KEY not set. Skipping review."
  echo "[rs-guard] Set it in ~/.config/rs-guard/env or export OPENAI_API_KEY=..."
  exit 0
fi

# Skip if nothing is staged
DIFF=$(git diff --cached --diff-filter=ACMR)
if [ -z "${DIFF}" ]; then
  echo "[rs-guard] No staged changes to review."
  exit 0
fi

echo "[rs-guard] Running code review on staged changes..."

# Run review (capture exit code)
EXIT_CODE=0
"${RS_GUARD}" --prompt-file .github/review-prompt.md --provider openai || EXIT_CODE=$?

case ${EXIT_CODE} in
  0)
    echo "[rs-guard] Review: APPROVE/COMMENT. Proceeding with commit."
    exit 0
    ;;
  2)
    echo "[rs-guard] Review: REQUEST_CHANGES. Commit blocked."
    echo "[rs-guard] Fix the issues above or bypass with: git commit --no-verify"
    exit 1
    ;;
  *)
    echo "[rs-guard] Review failed with unexpected exit code ${EXIT_CODE}. Allowing commit."
    exit 0
    ;;
esac
EOF

chmod +x "${HOOK_FILE}"
echo "Pre-commit hook installed at ${HOOK_FILE}"
