#!/usr/bin/env bash
set -euo pipefail

# provision-worktree.sh — group-timeout helper to spawn issue worktrees
#
# Manual test checklist (wrap with group-timeout):
# 1. `group-timeout 30 bash scripts/provision-worktree.sh --source main --target /tmp/prov-test --branch prov-test`
#    → creates a new worktree off main and runs the provision hook.
# 2. `group-timeout 30 bash scripts/provision-worktree.sh --source . --target /tmp/prov-from-current`
#    → refuses to proceed if the current tree is dirty.
# 3. `group-timeout 30 bash scripts/provision-worktree.sh --source path/to/worktree --target /tmp/prov-inherit --skip-hook`
#    → clones from an existing worktree without running the hook.

SCRIPT_NAME="$(basename "$0")"
if [[ "${GROUP_TIMEOUT_ACTIVE:-}" != "1" ]]; then
  printf 'error: %s must be run under group-timeout (global timeout).\n' "$SCRIPT_NAME" >&2
  exit 2
fi

usage() {
  cat <<'USAGE'
Usage: scripts/provision-worktree.sh --source <branch|path|commit> --target <folder> [--branch <name>] [--skip-hook]
Creates a new git worktree rooted at <folder>, starting from the provided source ref/path, hydrates LFS, and runs the provision hook.
USAGE
}

SOURCE=""
TARGET=""
BRANCH_NAME=""
SKIP_HOOK=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source) SOURCE="$2"; shift 2 ;;
    --target) TARGET="$2"; shift 2 ;;
    --branch) BRANCH_NAME="$2"; shift 2 ;;
    --skip-hook) SKIP_HOOK=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown flag: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$SOURCE" || -z "$TARGET" ]]; then
  usage
  exit 2
fi

ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"
if [[ -z "$ROOT" ]]; then
  echo "must run inside a git repository" >&2
  exit 1
fi

resolve_ref() {
  local ref="$1"
  git -C "$ROOT" rev-parse "$ref"
}

BASE_REF=""
SOURCE_DESC=""

if [[ -d "$SOURCE" ]]; then
  SOURCE_DESC="$(realpath "$SOURCE")"
  if ! git -C "$SOURCE_DESC" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    echo "--source path is not a git worktree: $SOURCE_DESC" >&2
    exit 1
  fi
  if [[ -n "$(git -C "$SOURCE_DESC" status --porcelain)" ]]; then
    echo "source worktree has uncommitted changes: $SOURCE_DESC" >&2
    exit 1
  fi
  BASE_REF="$(git -C "$SOURCE_DESC" rev-parse HEAD)"
else
  SOURCE_DESC="$SOURCE"
  if ! BASE_REF="$(resolve_ref "$SOURCE")"; then
    echo "unable to resolve source ref: $SOURCE" >&2
    exit 1
  fi
fi

TARGET_PATH="$TARGET"
if [[ "$TARGET_PATH" != /* ]]; then
  TARGET_PATH="$ROOT/$TARGET_PATH"
fi

if [[ -e "$TARGET_PATH" ]]; then
  echo "target already exists: $TARGET_PATH" >&2
  exit 1
fi

TARGET_DIR="$(dirname "$TARGET_PATH")"
mkdir -p "$TARGET_DIR"

BRANCH="${BRANCH_NAME:-$(basename "$TARGET_PATH")}"
if git -C "$ROOT" show-ref --verify --quiet "refs/heads/${BRANCH}"; then
  echo "branch already exists: $BRANCH" >&2
  exit 1
fi

echo "[provision] source: $SOURCE_DESC (ref $BASE_REF)"
echo "[provision] target: $TARGET_PATH"
echo "[provision] branch: $BRANCH"

git -C "$ROOT" worktree add -b "$BRANCH" "$TARGET_PATH" "$BASE_REF"

pushd "$TARGET_PATH" >/dev/null
git lfs install --local >/dev/null 2>&1 || true
git lfs pull --include "data/**" --exclude "" || true

if [[ "$SKIP_HOOK" -eq 0 ]]; then
DEFAULT_AFTER_HOOK="bash \"$ROOT/scripts/provision-worktree-hook-after.sh\""
HOOK_CMD="${PROVISION_WORKTREE_HOOK_AFTER:-$DEFAULT_AFTER_HOOK}"
if [[ -n "$HOOK_CMD" ]]; then
  echo "[provision] running after-hook via PROVISION_WORKTREE_HOOK_AFTER"
  eval "$HOOK_CMD"
else
  echo "[provision] skipping after-hook; PROVISION_WORKTREE_HOOK_AFTER is empty"
fi
else
  echo "[provision] skipping provision hook (per flag)."
fi
popd >/dev/null

echo "[provision] worktree ready at $TARGET_PATH"
