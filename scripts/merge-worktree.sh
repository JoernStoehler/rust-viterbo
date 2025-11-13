#!/usr/bin/env bash
set -euo pipefail

# merge-worktree.sh — disciplined fast-forward helper for issue branches
#
# Manual test checklist (wrap with group-timeout):
# 1. `group-timeout 60 bash scripts/merge-worktree.sh --source /path/to/worktree --target main --dry-run`
#    → prints the actions it would take (rebase + fast-forward) without mutating git state.
# 2. `group-timeout 60 bash scripts/merge-worktree.sh --source /path/to/worktree --target /workspaces/rust-viterbo --skip-rebase`
#    → fast-forwards the checked-out target worktree after verifying ancestry.
# 3. `group-timeout 60 bash scripts/merge-worktree.sh --source /path/to/worktree --target main`
#    → rebases onto main, then fast-forwards main (either via the root worktree or by moving the branch if detached).

SCRIPT_NAME="$(basename "$0")"
if [[ "${GROUP_TIMEOUT_ACTIVE:-}" != "1" ]]; then
  printf 'error: %s must be run under group-timeout (global timeout).\n' "$SCRIPT_NAME" >&2
  exit 2
fi

usage() {
  cat <<'USAGE'
Usage: scripts/merge-worktree.sh --source <folder> --target <branch|folder> [--ignore-uncommitted] [--skip-rebase] [--dry-run]
Rebases the source worktree onto the target ref and fast-forwards the target branch/worktree if everything succeeds.
USAGE
}

SOURCE_PATH=""
TARGET_SPEC=""
IGNORE_UNCOMMITTED=0
SKIP_REBASE=0
DRY_RUN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source) SOURCE_PATH="$2"; shift 2 ;;
    --target) TARGET_SPEC="$2"; shift 2 ;;
    --ignore-uncommitted) IGNORE_UNCOMMITTED=1; shift ;;
    --skip-rebase) SKIP_REBASE=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown flag: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$SOURCE_PATH" || -z "$TARGET_SPEC" ]]; then
  usage
  exit 2
fi

if [[ ! -d "$SOURCE_PATH" ]]; then
  echo "--source must be an existing worktree folder" >&2
  exit 1
fi
SOURCE_PATH="$(realpath "$SOURCE_PATH")"
if ! git -C "$SOURCE_PATH" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "--source is not a git worktree: $SOURCE_PATH" >&2
  exit 1
fi

# Ensure cleanliness unless explicitly ignored.
if [[ "$IGNORE_UNCOMMITTED" -eq 0 ]]; then
  if [[ -n "$(git -C "$SOURCE_PATH" status --porcelain)" ]]; then
    echo "source worktree has uncommitted changes; commit or stash first (use --ignore-uncommitted to override)" >&2
    exit 1
  fi
fi

find_worktree_for_branch() {
  local branch="$1"
  local current_path=""
  local current_branch=""
  while IFS= read -r line; do
    if [[ "$line" == worktree* ]]; then
      current_path="${line#worktree }"
    elif [[ "$line" == branch* ]]; then
      current_branch="${line#branch }"
      current_branch="${current_branch#refs/heads/}"
      if [[ "$current_branch" == "$branch" ]]; then
        echo "$current_path"
        return 0
      fi
    fi
  done < <(git worktree list --porcelain)
  return 1
}

ROOT="$(git -C "$SOURCE_PATH" rev-parse --show-toplevel)"

TARGET_PATH=""
TARGET_BRANCH=""
if [[ -d "$TARGET_SPEC" ]]; then
  TARGET_PATH="$(realpath "$TARGET_SPEC")"
  TARGET_BRANCH="$(git -C "$TARGET_PATH" rev-parse --abbrev-ref HEAD)"
  if [[ "$IGNORE_UNCOMMITTED" -eq 0 ]]; then
    if [[ -n "$(git -C "$TARGET_PATH" status --porcelain)" ]]; then
      echo "target worktree has uncommitted changes; clean it or pass --ignore-uncommitted." >&2
      exit 1
    fi
  fi
else
  TARGET_BRANCH="$TARGET_SPEC"
  if ! git -C "$ROOT" show-ref --verify --quiet "refs/heads/${TARGET_BRANCH}"; then
    echo "target branch does not exist: $TARGET_BRANCH" >&2
    exit 1
  fi
  if TARGET_PATH="$(find_worktree_for_branch "$TARGET_BRANCH")"; then
    if [[ "$IGNORE_UNCOMMITTED" -eq 0 && -n "$(git -C "$TARGET_PATH" status --porcelain)" ]]; then
      echo "target worktree has uncommitted changes; clean it or pass --ignore-uncommitted." >&2
      exit 1
    fi
  else
    TARGET_PATH=""
  fi
fi

SOURCE_BRANCH="$(git -C "$SOURCE_PATH" rev-parse --abbrev-ref HEAD)"
SOURCE_COMMIT="$(git -C "$SOURCE_PATH" rev-parse HEAD)"
TARGET_COMMIT="$(git -C "$ROOT" rev-parse "$TARGET_BRANCH")"

echo "[merge] source branch: $SOURCE_BRANCH ($SOURCE_COMMIT)"
echo "[merge] target branch: $TARGET_BRANCH ($TARGET_COMMIT)"
[[ -n "$TARGET_PATH" ]] && echo "[merge] target worktree: $TARGET_PATH"

if [[ "$SKIP_REBASE" -eq 0 ]]; then
  echo "[merge] rebasing $SOURCE_BRANCH onto $TARGET_BRANCH"
  git -C "$SOURCE_PATH" rebase "$TARGET_BRANCH"
  SOURCE_COMMIT="$(git -C "$SOURCE_PATH" rev-parse HEAD)"
else
  echo "[merge] skipping rebase per flag"
fi

if ! git -C "$ROOT" merge-base --is-ancestor "$TARGET_COMMIT" "$SOURCE_COMMIT"; then
  echo "source commit $SOURCE_COMMIT is not a descendant of $TARGET_BRANCH; cannot fast-forward" >&2
  exit 1
fi

if [[ "$DRY_RUN" -eq 1 ]]; then
  echo "[merge] dry-run: source is ready to fast-forward $TARGET_BRANCH to $SOURCE_COMMIT"
  exit 0
fi

if [[ -n "$TARGET_PATH" ]]; then
  echo "[merge] fast-forwarding checked-out target worktree"
  git -C "$TARGET_PATH" merge --ff-only "$SOURCE_COMMIT"
else
  echo "[merge] updating branch refs/heads/$TARGET_BRANCH -> $SOURCE_COMMIT"
  git -C "$ROOT" branch -f "$TARGET_BRANCH" "$SOURCE_COMMIT"
fi

echo "[merge] done."
