#!/usr/bin/env bash
# python-setup.sh — ensure uv virtualenv and dependencies are synced
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v uv >/dev/null 2>&1; then
  echo "error: uv is not installed; rebuild the devcontainer or install uv manually." >&2
  exit 1
fi

if [[ ! -d ".venv" ]]; then
  echo ">>> Creating Python virtualenv (.venv)"
  uv venv
fi

echo ">>> Syncing Python dependencies via uv"
uv sync --extra dev --locked >/dev/null 2>&1 || uv sync --extra dev
