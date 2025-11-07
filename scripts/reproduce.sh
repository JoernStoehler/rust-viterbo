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
#   * python-lint-type-test.sh: 300s
#   * rust-lint-test.sh: 300s
#   * pytest -m e2e: 600s
#   * Criterion benches: 600s
#   * bench docs stage: 120s
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

echo "=== Reproduce: Python lint/type/tests ==="
bash scripts/safe.sh --timeout 300 -- bash scripts/python-lint-type-test.sh

echo "=== Reproduce: Rust lint/tests ==="
bash scripts/safe.sh --timeout 300 -- bash scripts/rust-lint-test.sh

echo "=== Reproduce: Criterion benchmarks (raw data/bench/criterion) ==="
bash scripts/safe.sh --timeout 600 -- BENCH_RUN_POSTPROCESS=0 bash scripts/rust-bench.sh

echo "=== Reproduce: Criterion → docs assets stage ==="
bash scripts/safe.sh --timeout 180 -- uv run python -m viterbo.bench.stage_docs --config configs/bench/docs_local.json

echo "=== Reproduce: run end-to-end tests (pytest -m e2e) ==="
bash scripts/safe.sh --timeout 600 -- uv run pytest -q -m e2e

echo "=== Reproduce: build native Python extension (maturin) ==="
bash scripts/safe.sh --timeout 300 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml

echo "=== Reproduce: copy native .so into src/viterbo ==="
bash scripts/safe.sh --timeout 60 -- bash scripts/rust-build.sh --copy-only

echo "=== Reproduce: run data pipeline ==="
bash scripts/safe.sh --timeout 300 -- uv run --locked python -m viterbo.atlas.stage_build --config configs/atlas/full.json

echo "=== Reproduce: build thesis book (mdBook) ==="
bash scripts/safe.sh --timeout 600 -- mdbook build docs

echo "=== Reproduce: done. Artifacts under data/atlas/"
