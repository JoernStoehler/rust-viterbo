#!/usr/bin/env bash
# rust-setup.sh — ensure rustup components and shared target dir exist
set -euo pipefail

if ! command -v rustup >/dev/null 2>&1; then
  echo "error: rustup is missing; run rust-install-toolchain or rebuild the devcontainer." >&2
  exit 1
fi

rustup component add rustfmt >/dev/null 2>&1 || rustup component add rustfmt
rustup component add clippy >/dev/null 2>&1 || rustup component add clippy
rustup set profile default

# Ensure shared target dir for cargo exists when CARGO_TARGET_DIR is set
if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  mkdir -p "$CARGO_TARGET_DIR"
fi
