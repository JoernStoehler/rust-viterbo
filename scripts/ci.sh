#!/usr/bin/env bash
# ci.sh — manual CI entrypoint (requires safe.sh)
# Contract
# - Must be invoked under scripts/safe.sh (checks SAFE_WRAPPED=1).
# - Runs python-lint-type-test + rust-lint-test, then (optionally) benches + native build.
# - No internal timeouts; inherits top-level timeout from safe.sh.
set -euo pipefail

if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/ci.sh must be run under scripts/safe.sh (global timeout). See AGENTS.md → Command Line Quick Reference." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

bash scripts/python-lint-type-test.sh
bash scripts/rust-lint-test.sh

if [[ "${RUN_BENCH_IN_CI:-1}" == "1" ]]; then
  echo ">>> CI running Criterion benches"
  BENCH_RUN_POSTPROCESS=0 bash scripts/rust-bench.sh
  echo ">>> CI rendering bench tables"
  if command -v uv >/dev/null 2>&1; then
    uv run python -m viterbo.bench.stage_docs --config configs/bench/docs_local.json
  else
    python3 -m viterbo.bench.stage_docs --config configs/bench/docs_local.json
  fi
else
  echo ">>> skipping benches (RUN_BENCH_IN_CI=$RUN_BENCH_IN_CI)"
fi

if [[ "${WITH_NATIVE:-0}" == "1" ]]; then
  echo ">>> Build native extension with maturin..."
  uv run maturin develop -m crates/viterbo-py/Cargo.toml
fi

echo "CI: (optional) run selected E2E with: uv run pytest -m e2e -k atlas"
