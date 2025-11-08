#!/usr/bin/env bash
# rust-test.sh — cargo check/test convenience wrapper (requires safe.sh)
# Contract
# - Must be invoked under scripts/safe.sh (checks SAFE_WRAPPED=1).
# - No internal timeouts; inherits the top-level timeout from safe.sh.
# Usage:
#   safe -t 120 -- bash scripts/rust-test.sh [-p viterbo] [-- <extra cargo args>]
# Examples:
#   safe -t 120 -- bash scripts/rust-test.sh
#   safe -t 180 -- bash scripts/rust-test.sh -p viterbo -- -q
set -euo pipefail

if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/rust-test.sh must be run under scripts/safe.sh (global timeout). See AGENTS.md → Command Line Quick Reference." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Default Cargo target dir for tests unless the caller overrides.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-data/target}"
mkdir -p "$CARGO_TARGET_DIR"

# Prefer sccache if available for faster compiles across worktrees.
if command -v sccache >/dev/null 2>&1; then
  export RUSTC_WRAPPER="${RUSTC_WRAPPER:-sccache}"
fi

PKG="viterbo"
EXTRA=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--package) PKG="$2"; shift 2 ;;
    --) shift; EXTRA+=("$@"); break ;;
    *) EXTRA+=("$1"); shift ;;
  esac
done

# Run via nextest if available; fall back to cargo test.
if command -v cargo-nextest >/dev/null 2>&1; then
  echo ">>> cargo nextest run (-p $PKG) ${EXTRA[*]:-}"
  if [[ ${#EXTRA[@]} -gt 0 ]]; then
    cargo nextest run -p "$PKG" -- "${EXTRA[@]}"
  else
    cargo nextest run -p "$PKG"
  fi
else
  echo ">>> cargo test (-p $PKG) ${EXTRA[*]:-}"
  if [[ ${#EXTRA[@]} -gt 0 ]]; then
    cargo test -p "$PKG" -- "${EXTRA[@]}"
  else
    cargo test -p "$PKG"
  fi
fi
echo "Rust tests completed."
