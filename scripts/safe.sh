#!/usr/bin/env bash
set -euo pipefail

TIMEOUT=0
if [[ "${1:-}" == "--timeout" ]]; then
  TIMEOUT="$2"
  shift 2
fi
# Allow a standalone "--" separator between options and command
if [[ "${1:-}" == "--" ]]; then
  shift
fi

if [[ $# -eq 0 ]]; then
  echo "usage: $0 [--timeout SEC] <command...>" >&2
  exit 2
fi

cmd=("$@")
if (( TIMEOUT > 0 )); then
  cmd=(timeout --kill-after=10 "${TIMEOUT}s" "${cmd[@]}")
fi
# We go through `bash -lc` so we can (a) preserve the caller's command/quoting via "$@"
# and (b) use `exec -a safe-target …` to mark the entire process tree for cleanup.
setsid bash -lc 'exec -a safe-target "$@"' bash "${cmd[@]}" &
child=$!
# Get process group id of child
pgid="$(ps -o pgid= "$child" | tr -d ' ')"

# Wait and capture exit code
set +e
wait "$child"
rc=$?
set -e

# Ensure group cleanup
if [[ -n "${pgid:-}" ]]; then
  kill -TERM -"${pgid}" 2>/dev/null || true
  sleep 1
  kill -KILL -"${pgid}" 2>/dev/null || true
fi

exit "$rc"
