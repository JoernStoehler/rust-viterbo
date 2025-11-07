#!/usr/bin/env bash
# rust-fmt.sh — cargo fmt check (requires safe.sh)
set -euo pipefail
if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/rust-fmt.sh must be run under scripts/safe.sh" >&2
  exit 2
fi
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
echo ">>> cargo fmt --all --check"
cargo fmt --all --check
echo "Rust fmt check completed."

