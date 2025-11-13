#!/usr/bin/env bash
# lean-test.sh — Build and execute Lean smoke tests (requires group-timeout)
set -euo pipefail

SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
if [[ "${GROUP_TIMEOUT_ACTIVE:-}" != "1" ]]; then
  printf 'error: %s must be run under group-timeout (global timeout). See AGENTS.md → Command Line Quick Reference.\n' "$SCRIPT_NAME" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

bash "$ROOT_DIR/scripts/lean-setup.sh"

cd "$ROOT_DIR/lean"

echo ">>> Lean tests (SympLeanTests)"
lake build SympLeanTests
lake exe SympLeanTests
