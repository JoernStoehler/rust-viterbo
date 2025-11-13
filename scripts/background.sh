#!/usr/bin/env bash
set -euo pipefail
# background.sh — detach long-running commands with log capture
#
# Manual test checklist:
# 1. `bash scripts/background.sh sleep 2` → prints PID and log paths; exit code file becomes `0` after ~2s.
# 2. `bash scripts/background.sh bash -c 'echo hi; echo err >&2; exit 7'` → stdout/stderr logs contain the text and exitcode file reads `7`.
# 3. Tail a running job: `bash scripts/background.sh bash -c 'for i in {1..5}; do echo $i; sleep 1; done'` then `tail -f <stdout.log>` to observe streaming output.
# 4. Kill via PID: start `bash scripts/background.sh sleep 30`, then `kill <pid>` and ensure the exitcode log reflects the terminating signal.
#

usage() {
  cat >&2 <<'USAGE'
usage: background <command ...>
Runs <command> detached from the current turn, teeing stdout/stderr/exitcode into /tmp logs.
USAGE
}

if [[ $# -eq 0 ]]; then
  usage
  exit 2
fi

cmd=("$@")
stamp="$(date +%Y%m%d-%H%M%S)"
log_dir="/tmp/background-${stamp}-${RANDOM}"
stdout_log="$log_dir/stdout.log"
stderr_log="$log_dir/stderr.log"
exit_log="$log_dir/exitcode.log"
pid_file="$log_dir/pid"

mkdir -p "$log_dir"
: >"$stdout_log"
: >"$stderr_log"
: >"$exit_log"
WORKDIR="$PWD"

export BACKGROUND_EXIT_LOG="$exit_log"
setsid bash -c '
  exit_log="$BACKGROUND_EXIT_LOG"
  cleanup() {
    local status="$1"
    printf "%s\n" "$status" >"$exit_log"
  }
  trap '\''status=$?; cleanup "$status"'\'' EXIT
  cd "$1" || exit 99
  shift
  "$@"
' bash "$WORKDIR" "${cmd[@]}" >"$stdout_log" 2>"$stderr_log" &
child=$!
start_rc=$?
if (( start_rc != 0 )); then
  echo "background: failed to start command" >&2
  exit 1
fi
printf '%s\n' "$child" >"$pid_file"

cat <<EOF
[background] started
  pid: $child
  logs: $log_dir
  stdout: $stdout_log
  stderr: $stderr_log
  exitcode: $exit_log
EOF
printf 'BACKGROUND_PID=%s\nBACKGROUND_LOG_DIR=%s\nBACKGROUND_STDOUT=%s\nBACKGROUND_STDERR=%s\nBACKGROUND_EXITCODE=%s\n' \
  "$child" "$log_dir" "$stdout_log" "$stderr_log" "$exit_log"
