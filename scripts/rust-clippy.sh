#!/usr/bin/env bash
# rust-clippy.sh — cargo clippy convenience wrapper (requires group-timeout)
set -euo pipefail
SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
if [[ "${GROUP_TIMEOUT_ACTIVE:-}" != "1" ]]; then
  printf 'error: %s must be run under group-timeout (global timeout).\n' "$SCRIPT_NAME" >&2
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
echo ">>> cargo clippy (-p $PKG) --all-targets"
cargo clippy -p "$PKG" --all-targets -- -D warnings "${EXTRA[@]:-}"
echo "Rust clippy completed."
