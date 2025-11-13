#!/usr/bin/env bash
# rust-fmt.sh — cargo fmt check (requires group-timeout)
set -euo pipefail
SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
if [[ "${GROUP_TIMEOUT_ACTIVE:-}" != "1" ]]; then
  printf 'error: %s must be run under group-timeout (global timeout).\n' "$SCRIPT_NAME" >&2
  exit 2
fi
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
echo ">>> cargo fmt --all --check"
cargo fmt --all --check
echo "Rust fmt check completed."
