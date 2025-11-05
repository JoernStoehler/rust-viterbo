#!/usr/bin/env bash
set -euo pipefail
TIMEOUT=0
if [[ "${1:-}" == "--timeout" ]]; then TIMEOUT="$2"; shift 2; fi
# Allow a standalone "--" separator between options and command
if [[ "${1:-}" == "--" ]]; then shift; fi
if [[ $# -eq 0 ]]; then echo "usage: $0 [--timeout SEC] <command...>"; exit 2; fi

# Start the target in a new session. Use `timeout` to enforce limits.
if [[ "$TIMEOUT" -gt 0 ]]; then
  setsid bash -lc "exec -a safe-target timeout --kill-after=10 ${TIMEOUT}s $* " &
else
  setsid bash -lc "exec -a safe-target $* " &
fi
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
