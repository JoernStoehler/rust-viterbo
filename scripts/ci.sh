#!/usr/bin/env bash
# ci.sh — manual CI entrypoint (requires safe.sh)
# Contract
# - Must be invoked under scripts/safe.sh (checks SAFE_WRAPPED=1).
# - Runs checks.sh, and optionally builds the native extension when WITH_NATIVE=1.
# - No internal timeouts; inherits top-level timeout from safe.sh.
set -euo pipefail

if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/ci.sh must be run under scripts/safe.sh (global timeout). See AGENTS.md → Command Line Quick Reference." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

bash scripts/checks.sh

if [[ "${WITH_NATIVE:-0}" == "1" ]]; then
  echo ">>> Build native extension with maturin..."
  uv run maturin develop -m crates/viterbo-py/Cargo.toml
fi

echo "CI: (optional) run selected E2E with: uv run pytest -m e2e -k atlas"
