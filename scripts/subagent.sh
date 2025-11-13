#!/usr/bin/env bash
set -euo pipefail

# subagent.sh — fire-and-forget helper Codex turn (synchronous)
#
# Manual test checklist (wrap with group-timeout):
# 1. `group-timeout 30 bash scripts/subagent.sh --worktree /workspaces/rust-viterbo --prompt "echo hi"` runs a single turn and prints the final message.
# 2. `group-timeout 30 bash scripts/subagent.sh --worktree <path> --prompt-file prompt.txt` reads instructions from a file.
# 3. `group-timeout 30 bash scripts/subagent.sh --worktree <path> --prompt "test" -- -c reasoning_budget=low` forwards extra Codex CLI args.
# 4. On failure (non-zero exit), the script surfaces the stderr log path for debugging.

SCRIPT_NAME="$(basename "$0")"
if [[ "${GROUP_TIMEOUT_ACTIVE:-}" != "1" ]]; then
  printf 'error: %s must be run under group-timeout (global timeout).\n' "$SCRIPT_NAME" >&2
  exit 2
fi

usage() {
  cat <<'USAGE'
Usage: scripts/subagent.sh --worktree <path> [--prompt "text"|--prompt-file file] [--] [codex args...]
Runs a single Codex turn synchronously and prints the final message. Designed for scoped helper tasks.
USAGE
}

default_prompt() {
  local worktree="$1"
  cat <<EOF
You are a helper agent working inside $worktree.
- Perform the requested task quickly.
- Minimize side effects and keep reasoning concise.
- End with a clear final message summarizing results and follow-up commands (if any).
EOF
}

WORKTREE=""
PROMPT_TEXT=""
PROMPT_FILE=""
EXTRA_ARGS=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --worktree) WORKTREE="$2"; shift 2 ;;
    --prompt) PROMPT_TEXT="$2"; shift 2 ;;
    --prompt-file) PROMPT_FILE="$2"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    --) shift; EXTRA_ARGS+=("$@"); break ;;
    *) EXTRA_ARGS+=("$1"); shift ;;
  esac
done

if [[ -z "$WORKTREE" ]]; then
  echo "--worktree is required" >&2
  exit 2
fi
if [[ ! -d "$WORKTREE" ]]; then
  echo "worktree not found: $WORKTREE" >&2
  exit 1
fi
WORKTREE="$(realpath "$WORKTREE")"

if [[ -n "$PROMPT_FILE" ]]; then
  if [[ ! -f "$PROMPT_FILE" ]]; then
    echo "prompt file not found: $PROMPT_FILE" >&2
    exit 1
  fi
  PROMPT_TEXT="$(<"$PROMPT_FILE")"
fi

if [[ -z "$PROMPT_TEXT" ]]; then
  PROMPT_TEXT="$(default_prompt "$WORKTREE")"
fi

LOG_DIR="$(mktemp -d /tmp/subagent-XXXXXX)"
STDOUT_LOG="$LOG_DIR/stdout.jsonl"
STDERR_LOG="$LOG_DIR/stderr.log"
LAST_MSG="$LOG_DIR/final_message.md"

echo "[subagent] logs: $LOG_DIR"
set +e
codex exec --json --output-last-message "$LAST_MSG" --cd "$WORKTREE" "${EXTRA_ARGS[@]}" "$PROMPT_TEXT" \
  >"$STDOUT_LOG" 2>"$STDERR_LOG"
status=$?
set -e

if [[ -f "$LAST_MSG" ]]; then
  echo "----- Final message -----"
  cat "$LAST_MSG"
  echo "-------------------------"
fi

if [[ $status -ne 0 ]]; then
  echo "[subagent] codex exec exited with $status. See $STDERR_LOG" >&2
  exit "$status"
fi

echo "[subagent] turn completed successfully. Stdout log: $STDOUT_LOG"
