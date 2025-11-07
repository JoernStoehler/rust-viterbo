#!/usr/bin/env bash
set -euo pipefail
# reproduce.sh — end-to-end reproduction entrypoint (human-facing)
# Purpose
# - Build code, run tests (incl. E2E), regenerate data artifacts, and build the book for the current commit.
# - Documents sensible per-stage timeouts by wrapping each stage in scripts/safe.sh.
# Policy
# - Can be run directly (preferred for humans). Wrapping the whole script in safe.sh is optional.
# - Stage timeouts (tuned to catch mistakes yet allow expected runs):
#   * uv venv: 300s
#   * uv sync (locked): 300s
#   * checks.sh (format/lint/typecheck/unit tests/cargo): 300s
#   * benches (optional; RUN_RUST_BENCH=1): 600s
#   * pytest -m e2e: 600s
#   * maturin develop: 300s
#   * atlas pipeline: 300s
#   * mdBook build: 600s

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== Reproduce: ensure Python venv ==="
if [[ ! -d ".venv" ]]; then
  bash scripts/safe.sh --timeout 300 -- uv venv
fi

echo "=== Reproduce: sync project deps (uv, locked) ==="
# Keep env exact to the lock, include extras 'dev' like `.[dev]` used to.
bash scripts/safe.sh --timeout 300 -- uv sync --extra dev --locked

echo "=== Reproduce: run code checks and tests (checks.sh) ==="
# Note: checks.sh expects SAFE_WRAPPED when called directly, so we wrap it here.
bash scripts/safe.sh --timeout 300 -- bash scripts/checks.sh

# Optional: run Rust benches (Criterion) if RUN_RUST_BENCH=1 is set.
if [[ "${RUN_RUST_BENCH:-0}" == "1" ]]; then
  echo "=== Reproduce: run Rust benches (Criterion) to data/bench ==="
  CARGO_TARGET_DIR=data/bench bash scripts/safe.sh --timeout 600 -- bash scripts/rust-bench.sh
fi

echo "=== Reproduce: run end-to-end tests (pytest -m e2e) ==="
bash scripts/safe.sh --timeout 600 -- uv run pytest -q -m e2e

echo "=== Reproduce: build native Python extension (maturin) ==="
bash scripts/safe.sh --timeout 300 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml

echo "=== Reproduce: run data pipeline ==="
bash scripts/safe.sh --timeout 300 -- uv run --locked python -m viterbo.atlas.stage_build --config configs/atlas/full.json

echo "=== Reproduce: build thesis book (mdBook) ==="
bash scripts/safe.sh --timeout 600 -- mdbook build docs

echo "=== Reproduce: done. Artifacts under data/atlas/"
