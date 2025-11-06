#!/usr/bin/env bash
set -euo pipefail

TIMEOUT=0

usage() {
  cat >&2 <<EOF
usage: safe [-t SEC|--timeout SEC] [--] <command ...>
Runs a command in its own process group and cleans it up on exit.
EOF
}

# Parse flags: support --timeout SEC and -t SEC, stop on -- or first non-flag
while [[ $# -gt 0 ]]; do
  case "${1:-}" in
    --timeout)
      [[ $# -ge 2 ]] || { echo "error: --timeout requires an argument" >&2; exit 2; }
      TIMEOUT="$2"; shift 2
      ;;
    -t)
      [[ $# -ge 2 ]] || { echo "error: -t requires an argument" >&2; exit 2; }
      TIMEOUT="$2"; shift 2
      ;;
    --)
      shift
      break
      ;;
    -h|--help)
      usage; exit 0
      ;;
    -*)
      echo "error: unknown option: $1" >&2
      usage
      exit 2
      ;;
    *)
      break
      ;;
  esac
done

if [[ $# -eq 0 ]]; then
  usage
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
