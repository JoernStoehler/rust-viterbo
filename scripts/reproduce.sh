#!/usr/bin/env bash
set -euo pipefail
# Reproduction entrypoint defined in README: builds code/tests/data and the book at this commit.
# Minimal by design; see README.md → “Reproduce” for the contract and usage.

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
bash scripts/safe.sh --timeout 300 -- bash scripts/checks.sh

echo "=== Reproduce: run end-to-end tests (pytest -m e2e) ==="
bash scripts/safe.sh --timeout 600 -- uv run pytest -q -m e2e

echo "=== Reproduce: build native Python extension (maturin) ==="
bash scripts/safe.sh --timeout 300 -- uvx maturin develop -m crates/viterbo-py/Cargo.toml

echo "=== Reproduce: run data pipeline ==="
bash scripts/safe.sh --timeout 300 -- uv run --locked python -m viterbo.atlas.stage_build --config configs/atlas/full.json

echo "=== Reproduce: build thesis book (mdBook) ==="
bash scripts/safe.sh --timeout 600 -- mdbook build docs

echo "=== Reproduce: done. Artifacts under data/atlas/"
