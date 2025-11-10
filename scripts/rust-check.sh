#!/usr/bin/env bash
# rust-check.sh — cargo check convenience wrapper (requires safe.sh)
set -euo pipefail
if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/rust-check.sh must be run under scripts/safe.sh" >&2
  exit 2
fi
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/workspaces/rust-viterbo/.persist/cargo-target}"
mkdir -p "$CARGO_TARGET_DIR"
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
echo ">>> cargo check (-p $PKG) ${EXTRA[*]:-}"
cargo check -p "$PKG" "${EXTRA[@]:-}"
echo "Rust check completed."
