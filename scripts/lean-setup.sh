#!/usr/bin/env bash
# lean-setup.sh — hydrate the Lean toolchain and Lake dependencies
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEAN_DIR="$ROOT_DIR/lean"

if [[ ! -d "$LEAN_DIR" ]]; then
  echo "error: Lean workspace not found at $LEAN_DIR" >&2
  exit 1
fi

need_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    cat >&2 <<EOF
error: required tool '$tool' not found in PATH.
Rebuild the devcontainer (once postCreate installs elan/lake) or install it manually if you are outside the container.
EOF
    exit 1
  fi
}

need_tool elan
need_tool lake

cd "$LEAN_DIR"
TOOLCHAIN="$(cat lean-toolchain)"
if ! elan toolchain list | grep -Fq "$TOOLCHAIN"; then
  elan toolchain install "$TOOLCHAIN"
fi
elan default "$TOOLCHAIN"
lake update
