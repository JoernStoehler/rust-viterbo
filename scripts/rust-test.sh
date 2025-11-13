#!/usr/bin/env bash
# rust-test.sh — cargo check/test convenience wrapper (requires group-timeout)
# Contract
# - Must be invoked under group-timeout (checks GROUP_TIMEOUT_ACTIVE=1).
# - No internal timeouts; inherits the top-level timeout from group-timeout.
# Usage:
#   group-timeout 120 bash scripts/rust-test.sh [-p viterbo] [-- <extra cargo args>]
# Examples:
#   group-timeout 120 bash scripts/rust-test.sh
#   group-timeout 180 bash scripts/rust-test.sh -p viterbo
set -euo pipefail

SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
if [[ "${GROUP_TIMEOUT_ACTIVE:-}" != "1" ]]; then
  printf 'error: %s must be run under group-timeout (global timeout). See AGENTS.md → Command Line Quick Reference.\n' "$SCRIPT_NAME" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

# Default Cargo target dir for tests unless the caller overrides.
# Hygiene: keep caches OUT of the repo. Use a global temp dir by default; do not
# Shared cache lives at .persist/cargo-target (see AGENTS.md).
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/workspaces/rust-viterbo/.persist/cargo-target}"
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
