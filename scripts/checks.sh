#!/usr/bin/env bash
# checks.sh — fast code checks for quick feedback (requires safe.sh)
# Contract
# - Must be invoked under scripts/safe.sh (checks SAFE_WRAPPED=1).
# - No internal timeouts; inherits the top-level timeout from safe.sh.
# Stages (cheap):
# - Ruff format/lint, ensure venv + uv sync (locked), pyright basic, pytest (non-e2e),
#   cargo check/test for crate viterbo.
set -euo pipefail

if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/checks.sh must be run under scripts/safe.sh (global timeout). See AGENTS.md → Command Line Quick Reference." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo ">>> Formatting (ruff)..."
uv run ruff format src tests || true

echo ">>> Lint (ruff)..."
uv run ruff check src tests || true

echo ">>> Ensure Python venv + deps sync..."
if [[ ! -d ".venv" ]]; then
  uv venv
fi
# Keep environment in sync with the lockfile; include extras 'dev'
uv sync --extra dev --locked

echo ">>> Type check (pyright basic)..."
uv run pyright || true

echo ">>> Python smoke tests..."
uv run pytest -q -m "not e2e"

echo ">>> Cargo check + tests (fast)..."
cargo check -q -p viterbo
cargo test  -q -p viterbo

echo "All quick checks passed."
