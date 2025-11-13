#!/usr/bin/env bash
set -euo pipefail
# group-timeout.sh — enforce process-group deadlines for repo commands
#
# Manual test checklist (run directly, not under another wrapper):
# 1. Success path: `bash scripts/group-timeout.sh 2 bash -c 'sleep 1'` → exits 0.
# 2. Timeout path: `bash scripts/group-timeout.sh 1 bash -c 'sleep 5'` → exits 124 and logs a timeout message.
# 3. Propagate exit codes: `bash scripts/group-timeout.sh 5 bash -c 'exit 42'` → exits 42.
# 4. Cleans grandchildren: `bash scripts/group-timeout.sh 2 bash -c 'sleep 5 & sleep 5 & wait'` → all sleeps die once the timeout hits.
# 5. Env flag: `bash scripts/group-timeout.sh 1 env | grep GROUP_TIMEOUT_ACTIVE` shows the variable is exported for child scripts.
#

usage() {
  cat >&2 <<'USAGE'
usage: group-timeout <seconds> [--] <command ...>
Runs <command> inside its own process group, killing the entire tree when the deadline expires.
USAGE
}

if [[ $# -lt 2 ]]; then
  usage
  exit 2
fi

limit="$1"
shift

if [[ "$limit" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  deadline="$limit"
else
  echo "group-timeout: <seconds> must be a positive number" >&2
  exit 2
fi

if [[ "${1:-}" == "--" ]]; then
  shift
fi

if [[ $# -eq 0 ]]; then
  usage
  exit 2
fi

cmd=("$@")
export GROUP_TIMEOUT_ACTIVE="1"
export GROUP_TIMEOUT_SECONDS="$deadline"

flag_file="$(mktemp -t group-timeout-flag.XXXXXX)"
cleanup_flag() { rm -f "$flag_file"; }
trap cleanup_flag EXIT

setsid bash -c 'exec "$@"' bash "${cmd[@]}" &
child=$!
if ! kill -0 "$child" 2>/dev/null; then
  echo "group-timeout: failed to start target command" >&2
  exit 1
fi

pgid="$(ps -o pgid= -p "$child" 2>/dev/null | tr -d ' ')"
if [[ -z "$pgid" ]]; then
  echo "group-timeout: could not determine process group" >&2
  kill "$child" 2>/dev/null || true
  exit 1
fi

(
  sleep "$deadline"
  if kill -0 "-$pgid" 2>/dev/null; then
    printf 'timeout' >"$flag_file"
    kill -TERM "-$pgid" 2>/dev/null || true
    sleep 5
    kill -KILL "-$pgid" 2>/dev/null || true
  fi
) &
timer=$!

set +e
wait "$child"
rc=$?
set -e

kill "$timer" 2>/dev/null || true
wait "$timer" 2>/dev/null || true

if [[ -s "$flag_file" ]]; then
  echo "[group-timeout] command exceeded ${deadline}s; terminated." >&2
  rc=124
fi

kill -TERM "-$pgid" 2>/dev/null || true
sleep 1
kill -KILL "-$pgid" 2>/dev/null || true

exit "$rc"
