#!/usr/bin/env bash
# rust-build.sh — build/copy the PyO3 native extension into src/viterbo/
# Contract
# - Must be invoked under scripts/safe.sh (checks SAFE_WRAPPED=1).
# - Default behavior: build via maturin develop, then copy the built .so
#   next to the Python package (src/viterbo/) so it travels with the repo.
# - Use --copy-only to skip the build and only copy from the current venv.
# - No internal timeouts; inherits top-level timeout from safe.sh.
set -euo pipefail

if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/rust-build.sh must be run under scripts/safe.sh (global timeout). See AGENTS.md → Command Line Quick Reference." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

COPY_ONLY=0
if [[ "${1:-}" == "--copy-only" ]]; then
  COPY_ONLY=1
fi

if [[ $COPY_ONLY -eq 0 ]]; then
  if ! command -v uv >/dev/null 2>&1; then
    echo "error: uv is required" >&2
    exit 2
  fi
  # Ensure venv and dev deps
  [[ -d ".venv" ]] || uv venv
  uv sync --extra dev --locked || uv sync --extra dev
  # Build/install the extension into the venv site-packages
  uv run maturin develop -m crates/viterbo-py/Cargo.toml
fi

# Locate the installed extension binary in the active venv and copy it
SO_PATH="$(
uv run python - <<'PY'
import importlib, pathlib
m = importlib.import_module('viterbo_native')
d = pathlib.Path(m.__file__).parent
cs = list(d.glob('viterbo_native*.so'))
print(str(cs[0]) if cs else '')
PY
)"
if [[ -z "${SO_PATH:-}" || ! -f "$SO_PATH" ]]; then
  echo "error: could not locate installed viterbo_native extension (.so)" >&2
  exit 1
fi
DEST_DIR="$ROOT_DIR/src/viterbo"
mkdir -p "$DEST_DIR"
cp -f "$SO_PATH" "$DEST_DIR/"

# Write sidecar stamp with provenance for CI freshness checks
STAMP_PATH="$DEST_DIR/$(basename "$SO_PATH").run.json"
GIT_COMMIT="$(git rev-parse HEAD || echo unknown)"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
RUSTC="$(rustc --version 2>/dev/null || echo '')"
SIZE_BYTES="$(stat -c%s "$DEST_DIR/$(basename "$SO_PATH")" 2>/dev/null || echo 0)"
if command -v sha256sum >/dev/null 2>&1; then
  SHA256="$(sha256sum "$DEST_DIR/$(basename "$SO_PATH")" | awk '{print $1}')"
else
  SHA256=""
fi
python3 - <<PY 2>/dev/null || uv run python - <<PY
import json, sys
payload = {
  "git_commit": "${GIT_COMMIT}",
  "timestamp": "${TIMESTAMP}",
  "rustc": "${RUSTC}",
  "filename": "$(basename "$SO_PATH")",
  "size_bytes": ${SIZE_BYTES},
  "sha256": "${SHA256}",
}
open("${STAMP_PATH}", "w", encoding="utf-8").write(json.dumps(payload, indent=2))
PY

echo "Copied $(basename "$SO_PATH") → src/viterbo/ (stamp: $(basename "$STAMP_PATH"))"
