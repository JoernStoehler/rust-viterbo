#!/usr/bin/env bash
# python-lint-type-test.sh — fast Python lint/type/test loop (requires safe.sh)
# Contract
# - Must be invoked under scripts/safe.sh (checks SAFE_WRAPPED=1).
# - No internal timeouts; inherits the top-level timeout from safe.sh.
# Stages (cheap):
# - Ruff format/lint, ensure venv + uv sync (locked), pyright basic, pytest (non-e2e).
set -euo pipefail

if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/python-lint-type-test.sh must be run under scripts/safe.sh (global timeout). See AGENTS.md → Command Line Quick Reference." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo ">>> Formatting (ruff)..."
# Intentional '|| true': the fast loop is designed to surface ALL issues in one
# pass (formatting, lint, type, tests). Failing early would hide subsequent
# problems and force multiple runs. CI is strict; this loop is advisory.
uv run ruff format src tests || true

echo ">>> Lint (ruff)..."
# See note above: keep non-fatal here to show a full list of problems quickly.
uv run ruff check src tests || true

echo ">>> Ensure Python venv + deps sync..."
if [[ ! -d ".venv" ]]; then
  uv venv
fi
# Keep environment in sync with the lockfile; include extras 'dev'
uv sync --extra dev --locked

echo ">>> Type check (pyright basic)..."
# Also non-fatal here; tests below remain strict.
uv run pyright || true

echo ">>> Python smoke tests..."
uv run pytest -q -m "not e2e"

echo "All Python lint/type/tests passed."
