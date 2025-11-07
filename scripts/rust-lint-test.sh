#!/usr/bin/env bash
# rust-lint-test.sh — Rust fmt/clippy/test loop (requires safe.sh)
# Contract
# - Must be invoked under scripts/safe.sh (checks SAFE_WRAPPED=1).
# - No internal timeouts; inherits top-level timeout from safe.sh.
# - Defaults CARGO_TARGET_DIR to data/target for cache locality unless overridden.
# Usage:
#   safe -t 180 -- bash scripts/rust-lint-test.sh [-p viterbo] [-- <extra cargo test args>]
set -euo pipefail

if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/rust-lint-test.sh must be run under scripts/safe.sh (global timeout). See AGENTS.md → Command Line Quick Reference." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-data/target}"
mkdir -p "$CARGO_TARGET_DIR"

PKG="viterbo"
EXTRA_TEST_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--package)
      PKG="$2"; shift 2 ;;
    --)
      shift; EXTRA_TEST_ARGS+=("$@"); break ;;
    *)
      EXTRA_TEST_ARGS+=("$1"); shift ;;
  esac
done

echo ">>> cargo fmt --all --check"
cargo fmt --all --check

echo ">>> cargo clippy (-p $PKG)"
cargo clippy -p "$PKG" --all-targets -- -D warnings

echo ">>> cargo test  (-p $PKG) ${EXTRA_TEST_ARGS[*]:-}"
cargo test -p "$PKG" "${EXTRA_TEST_ARGS[@]:-}"

echo "Rust lint/test completed."
