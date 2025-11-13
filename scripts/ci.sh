#!/usr/bin/env bash
# ci.sh — strict CI entrypoint (requires group-timeout)
# Contract
# - Must be invoked under group-timeout (checks GROUP_TIMEOUT_ACTIVE=1).
# - Strict: formatting, lint, and type errors fail the run (no `|| true`).
# - Builds the native extension unconditionally so failures surface early.
# - Optionally runs benches and renders docs tables (controlled via RUN_BENCH_IN_CI=0/1).
# - No internal timeouts; inherits top-level timeout from group-timeout.
set -euo pipefail

SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
if [[ "${GROUP_TIMEOUT_ACTIVE:-}" != "1" ]]; then
  printf 'error: %s must be run under group-timeout (global timeout). See AGENTS.md → Command Line Quick Reference.\n' "$SCRIPT_NAME" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo ">>> Ensure Python env (uv, locked)"
if [[ ! -d ".venv" ]]; then
  uv venv
fi
uv sync --extra dev --locked

echo ">>> Python format check (ruff --check)"
uv run ruff format --check src tests
echo ">>> Python lint (ruff)"
uv run ruff check src tests
echo ">>> Type check (pyright)"
uv run pyright
echo ">>> Python smoke tests"
uv run pytest -q -m "not e2e"

echo ">>> Rust fmt/test/clippy"
bash scripts/rust-fmt.sh
bash scripts/rust-test.sh
bash scripts/rust-clippy.sh

echo ">>> Build native extension (maturin) — fail fast if missing"
uv run maturin develop -m crates/viterbo-py/Cargo.toml

if [[ "${RUN_BENCH_IN_CI:-1}" == "1" ]]; then
  echo ">>> CI running Criterion benches"
  BENCH_RUN_POSTPROCESS=0 bash scripts/rust-bench.sh
  echo ">>> CI rendering bench tables"
  uv run python -m viterbo.bench.stage_docs --config configs/bench/docs_local.json
else
  echo ">>> skipping benches (RUN_BENCH_IN_CI=$RUN_BENCH_IN_CI)"
fi

echo ">>> Suggested E2E (on-demand): uv run pytest -q -m e2e -k atlas"
