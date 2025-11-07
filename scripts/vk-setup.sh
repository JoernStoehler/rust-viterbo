#!/usr/bin/env bash
set -euo pipefail

# -----------------------------------------------------------------------------
# Vibe Kanban setup hook.
# Runs after VK provisions a new worktree (git worktree add + file copies)
# and before an agent receives the ticket payload. Responsibilities:
#   * Recreate high-signal directories (e.g., data/) and hydrate Git LFS
#     pointers so agents see the latest artifacts without manual steps.
#   * Ensure Python tooling (uv + .venv) is ready and matches uv.lock, so
#     agents can run `uv run …` commands immediately.
#   * Pre-fetch Rust crates so the first `cargo check/test` does not block on
#     network installs.
# -----------------------------------------------------------------------------

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

log() { printf '[vk-setup] %s\n' "$*"; }
die() { printf '[vk-setup][error] %s\n' "$*" >&2; exit 1; }

need_cmd() {
  local cmd="$1"
  local hint="${2:-}"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    if [[ -n "$hint" ]]; then
      die "Missing required command '$cmd'. $hint"
    else
      die "Missing required command '$cmd'."
    fi
  fi
}

need_cmd git
need_cmd git-lfs "Install Git LFS (https://git-lfs.com/) before launching VK agents."
need_cmd python3 "Install Python 3.11+ before launching VK agents."
need_cmd uv "Install uv: https://docs.astral.sh/uv/getting-started/installation/."
need_cmd cargo "Install Rust via rustup (https://rustup.rs/) before continuing."

log "Repository root: $ROOT_DIR"
log "Branch: $(git rev-parse --abbrev-ref HEAD)"
log "Commit: $(git rev-parse --short HEAD)"

log "Ensuring baseline directories exist (data/, docs/assets/)..."
mkdir -p data docs/assets

log "Ensuring Git LFS filters are installed locally..."
git lfs install --local >/dev/null

log "Hydrating data/ via Git LFS (safe to skip if empty)..."
data_lfs_count=$(git lfs ls-files --name-only data 2>/dev/null | wc -l | tr -d " ")
if [[ "${data_lfs_count:-0}" -gt 0 ]]; then
  if ! git lfs pull --include "data/**" --exclude ""; then
    log "git lfs pull failed (maybe offline); continuing with pointers only."
  fi
else
  log "No Git LFS-tracked data artifacts yet; skipping hydration."
fi

log "Ensuring uv virtual environment exists..."
if [[ ! -d ".venv" ]]; then
  uv venv --python python3
fi

log "Syncing Python dependencies from uv.lock (incl. dev extras)..."
uv sync --locked --extra dev

if [[ -f "Cargo.toml" ]]; then
  log "Prefetching Rust crates via cargo fetch (locked)..."
  cargo fetch --locked >/dev/null
fi

log "vk-setup completed."
