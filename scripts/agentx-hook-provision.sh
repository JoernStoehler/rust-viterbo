#!/usr/bin/env bash
set -euo pipefail
#
# agentx-hook-provision.sh
# Run inside a freshly created ticket worktree immediately after 'git worktree add'.
# Idempotent and fast. Keep this tiny and local-only.
#
# Recommended devcontainer setting (containerEnv/remoteEnv):
#   "AGENTX_HOOK_PROVISION": "bash scripts/agentx-hook-provision.sh"
#
# What it does:
# - Ensure the shared tickets symlink exists (agentx creates it; we verify).
# - Install Git LFS locally for this worktree (if available).
# - Hydrate LFS pointers for data/** (if available).
# - Print next-steps hints (quick loops).
#
# It must NOT:
# - Reach out to external services beyond local Git LFS pulls.
# - Create heavy environments or run long CI.

log() { printf '[agentx-provision] %s\n' "$*"; }
warn() { printf '[agentx-provision][warn] %s\n' "$*" >&2; }
die() { printf '[agentx-provision][err] %s\n' "$*" >&2; exit 1; }
has() { command -v "$1" >/dev/null 2>&1; }

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
# Required env (no implicit fallbacks)
AGENTX_TICKETS_DIR="${AGENTX_TICKETS_DIR:-}"
LOCAL_TICKET_FOLDER="${LOCAL_TICKET_FOLDER:-}"
[ -n "${AGENTX_TICKETS_DIR}" ] && [ -n "${LOCAL_TICKET_FOLDER}" ] || die "AGENTX_TICKETS_DIR and LOCAL_TICKET_FOLDER must be set (see AGENTS.md)."
SHARED_LINK="${ROOT}/${LOCAL_TICKET_FOLDER#./}"

# Ensure shared/tickets symlink
mkdir -p "$(dirname "$SHARED_LINK")"
ln -sfn "$AGENTX_TICKETS_DIR" "$SHARED_LINK"
log "symlink ok: ${LOCAL_TICKET_FOLDER} -> ${AGENTX_TICKETS_DIR}"

# Git LFS setup and hydrate data/** if available
if has git && has git-lfs; then
  git lfs install --local || warn "git lfs install failed (continuing)"
  if [ -d "${ROOT}/data" ]; then
    log "hydrating LFS pointers under data/**"
    git lfs pull --include "data/**" --exclude "" || warn "git lfs pull data/** failed (continuing)"
  fi
else
  warn "git-lfs not found; skipping LFS hydration"
fi

log "done."
