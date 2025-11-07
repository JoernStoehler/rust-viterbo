#!/usr/bin/env bash
# rust-bench.sh — cargo bench convenience wrapper (requires safe.sh)
# Contract
# - Must be invoked under scripts/safe.sh (checks SAFE_WRAPPED=1).
# - No internal timeouts; inherits the top-level timeout from safe.sh.
# - Defaults Criterion output to data/bench (gitignored) unless CARGO_TARGET_DIR is provided.
# Usage:
#   safe -t 300 -- bash scripts/rust-bench.sh [-p viterbo] [-- <extra cargo bench args>]
# Examples:
#   safe -t 300 -- bash scripts/rust-bench.sh
#   safe -t 120 -- bash scripts/rust-bench.sh -- --no-run         # compile benches only
#   CARGO_TARGET_DIR=data/bench safe -t 300 -- bash scripts/rust-bench.sh
set -euo pipefail

if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/rust-bench.sh must be run under scripts/safe.sh (global timeout). See AGENTS.md → Command Line Quick Reference." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PKG="viterbo"
EXTRA=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--package) PKG="$2"; shift 2 ;;
    --) shift; EXTRA+=("$@"); break ;;
    *) EXTRA+=("$1"); shift ;;
  esac
done

# Default Criterion output location to data/bench unless caller overrides.
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-data/bench}"
mkdir -p "$CARGO_TARGET_DIR"

echo ">>> cargo bench (-p $PKG) target=$CARGO_TARGET_DIR ${EXTRA[*]:-}"
cargo bench -p "$PKG" "${EXTRA[@]:-}"
echo "Rust benches completed."
