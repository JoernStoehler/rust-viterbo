#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo ">>> Formatting (ruff)..."
uvx ruff format src tests || true

echo ">>> Lint (ruff)..."
uvx ruff check src tests || true

echo ">>> Ensure Python venv + editable install..."
if [[ ! -d ".venv" ]]; then
  uv venv
fi
uv pip install -q -e .[dev]

echo ">>> Type check (pyright basic)..."
uvx pyright || true

echo ">>> Python smoke tests..."
uv run pytest -q -m "not e2e"

echo ">>> Cargo check + tests (fast)..."
cargo check -q -p viterbo
cargo test  -q -p viterbo

echo "All quick checks passed."
