#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

bash scripts/checks.sh

if [[ "${WITH_NATIVE:-0}" == "1" ]]; then
  echo ">>> Build native extension with maturin..."
  uv run maturin develop -m crates/viterbo-py/Cargo.toml
fi

echo "CI: (optional) run selected E2E with: uv run pytest -m e2e -k atlas"
