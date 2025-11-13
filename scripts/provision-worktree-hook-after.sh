#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
: "${ROOT:?ROOT must resolve}"

mkdir -p "$ROOT/.persist/issues"
ln -sfn "$ROOT/.persist/issues" "$ROOT/issues"

if command -v git >/dev/null 2>&1 && command -v git-lfs >/dev/null 2>&1; then
  git -C "$ROOT" lfs install --local >/dev/null 2>&1 || true
  if [[ -d "$ROOT/data" ]]; then
    git -C "$ROOT" lfs pull --include "data/**" --exclude "" || true
  fi
fi

cd "$ROOT"
bash "$ROOT/scripts/python-setup.sh"
if command -v elan >/dev/null 2>&1 && command -v lake >/dev/null 2>&1; then
  bash "$ROOT/scripts/lean-setup.sh"
fi
