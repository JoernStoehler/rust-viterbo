#!/usr/bin/env bash
set -euo pipefail

# Documentation-oriented, full rebuild would be long in the future.
# For now, keep it fast and demonstrative.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== Reproduce: ensure Python env and package ==="
uv pip install -q -e .[dev]

echo "=== Reproduce: build (optional) native extension with maturin (skipped by default) ==="
echo "Skip: run 'uvx maturin develop -m crates/viterbo-py/Cargo.toml' if/when native bindings are needed."

echo "=== Reproduce: run tiny atlas pipeline ==="
bash scripts/safe.sh --timeout 60 -- uv run python -m viterbo.atlas.stage_build --config configs/atlas/test.json

echo "=== Reproduce: done. Artifacts under data/atlas/"

