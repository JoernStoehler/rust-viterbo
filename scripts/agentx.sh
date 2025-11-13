#!/usr/bin/env bash
set -euo pipefail

# agentx.sh — Codex session/worktree orchestrator (Ticket workflow helper)
#
# Manual test checklist (plain bash is fine; wrap in group-timeout only if you want a guard):
# 1. `bash scripts/agentx.sh run --worktree /workspaces/rust-viterbo --prompt 'READONLY: hi'`
#      → prints `[background]… Session <id> running …` and a new entry appears in `agentx.sh list`.
#    1a. Repeat with `AGENTX_HOOK_BEFORE_TURN_BEGIN="echo hook"` in the environment to ensure hooks run
#        with a defined session id before Codex launches (guards against regression #2025-11-13).
# 2. `bash scripts/agentx.sh await --session <id>` (for the id created above)
#      → waits for `/tmp/background-*/exitcode.log`, then prints the exit code plus the captured final message.
# 3. `bash scripts/agentx.sh run --worktree /workspaces/rust-viterbo --prompt-file <sleep-prompt>`
#      followed quickly by `bash scripts/agentx.sh abort --session <id>`
#      → sends SIGTERM/SIGKILL to the recorded PID, sets status to `stopped`, and the exitcode log reflects the signal/non‑zero status.
# 4. `bash scripts/agentx.sh archive --session <inactive-id>`
#      → switches the entry to `archived` (fails if you target an active session).
# 5. `bash scripts/agentx.sh run --session <id> --worktree /tmp`
#      → errors with “worktree mismatch…” because the session belongs to `/workspaces/rust-viterbo`.
# 6. `bash scripts/agentx.sh view --session <id>` / `list`
#      → ensure JSON output/log paths match what was stored.
# 7. `bash scripts/agentx.sh await --session does-not-exist`
#      → should print `session not found`.
# 8. `bash scripts/agentx.sh list --filter status=unmanaged --fields session_id,status,pid,cmd`
#      → surfaces live Codex CLI processes that agentx is not managing (should show the CLI running this turn).
# 9. `bash scripts/agentx.sh list --filter status=log --fields session_id,worktree,session_file | head`
#      → proves ~/.codex/sessions files are parsed and listed as historical/log-only rows.

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required for scripts/agentx.sh" >&2
  exit 2
fi

CONFIG_DIR="${HOME}/.config/agentx"
STATE_FILE="${CONFIG_DIR}/state.json"
STATE_LOCK="${CONFIG_DIR}/state.lock"
SESSION_DIR_ROOT="${CONFIG_DIR}/sessions"
mkdir -p "$CONFIG_DIR" "$SESSION_DIR_ROOT"
touch "$STATE_LOCK"
if [[ ! -f "$STATE_FILE" ]]; then
  printf '{"sessions":{}}\n' >"$STATE_FILE"
fi

ts_utc() {
  date -u +%Y-%m-%dT%H:%M:%SZ
}

usage() {
  cat <<'USAGE'
Usage: scripts/agentx.sh <command> [options]
Commands:
  list [--fields a,b,...] [--sort-by field] [--filter key=value1,value2]
  view (--session <id> | --worktree <path>)
  run --worktree <path> [--session <id>] [--prompt "text"|--prompt-file file] [--message "text"] [--] [codex args...]
  abort --session <id>
  archive --session <id> [--delete-worktree]
  await --session <id>
  help

Notes:
  - Status values now include managed entries (active/inactive/stopped/archived),
    `unmanaged` (live Codex CLI processes outside agentx), and `log` (session
    metadata discovered from ~/.codex/sessions). Filter with
    `--filter status=active,inactive,unmanaged`, etc.
  - Extra fields like `cmd` and `session_file` can be displayed via --fields.
USAGE
}

refresh_state() {
  flock "$STATE_LOCK" /usr/bin/env python3 - "$STATE_FILE" <<'PY'
import json, os, sys, time
path = sys.argv[1]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)
changed = False
sessions = data.get("sessions", {})
now = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
for sess in sessions.values():
    if sess.get("status") != "active":
        continue
    pid = sess.get("pid")
    if not pid:
        continue
    try:
        pid_int = int(pid)
    except Exception:
        pid_int = None
    alive = False
    if pid_int is not None:
        alive = os.path.exists(f"/proc/{pid_int}")
    if alive:
        continue
    exit_path = sess.get("exitcode_log")
    exit_code = sess.get("last_exitcode")
    if exit_path and os.path.exists(exit_path):
        try:
            with open(exit_path, "r", encoding="utf-8") as ef:
                text = ef.read().strip()
            if text:
                exit_code = int(text)
        except Exception:
            pass
    sess["pid"] = None
    sess["status"] = "inactive"
    sess["last_exitcode"] = exit_code
    sess["turns_completed"] = int(sess.get("turns_completed", 0)) + 1
    sess["updated_at"] = now
    changed = True
if changed:
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(data, fh, indent=2, sort_keys=True)
PY
}

state_dump() {
  flock "$STATE_LOCK" /usr/bin/env python3 - "$STATE_FILE" <<'PY'
import json, sys
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    sys.stdout.write(json.dumps(json.load(fh)))
PY
}

get_session_json() {
  local session_id="$1"
  local json
  json="$(
    flock "$STATE_LOCK" /usr/bin/env python3 - "$STATE_FILE" "$session_id" <<'PY'
import json, sys
path, sid = sys.argv[1], sys.argv[2]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)
sess = data.get("sessions", {}).get(sid)
if sess is None:
    sys.exit(1)
print(json.dumps(sess))
PY
  )"
  printf '%s\n' "$json"
}

apply_session_patch() {
  local session_id="$1"
  local patch_json="$2"
  PATCH_JSON="$patch_json" flock "$STATE_LOCK" /usr/bin/env python3 - "$STATE_FILE" "$session_id" <<'PY'
import json, os, sys, time
path, sid = sys.argv[1], sys.argv[2]
patch = json.loads(os.environ.get("PATCH_JSON", "{}"))
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)
sessions = data.setdefault("sessions", {})
if sid not in sessions:
    raise SystemExit(1)
sessions[sid].update(patch)
sessions[sid]["updated_at"] = patch.get("updated_at", time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()))
with open(path, "w", encoding="utf-8") as fh:
    json.dump(data, fh, indent=2, sort_keys=True)
PY
}

insert_session() {
  local payload="$1"
  PAYLOAD="$payload" flock "$STATE_LOCK" /usr/bin/env python3 - "$STATE_FILE" <<'PY'
import json, os, sys
payload = json.loads(os.environ["PAYLOAD"])
path = sys.argv[1]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)
sessions = data.setdefault("sessions", {})
sessions[payload["session_id"]] = payload
with open(path, "w", encoding="utf-8") as fh:
    json.dump(data, fh, indent=2, sort_keys=True)
PY
}

session_exists() {
  local session_id="$1"
  if [[ -z "$session_id" ]]; then
    return 1
  fi
  if flock "$STATE_LOCK" /usr/bin/env python3 - "$STATE_FILE" "$session_id" <<'PY' >/dev/null
import json, sys
path, sid = sys.argv[1], sys.argv[2]
with open(path, "r", encoding="utf-8") as fh:
    data = json.load(fh)
if sid in data.get("sessions", {}):
    raise SystemExit(0)
raise SystemExit(1)
PY
  then
    return 0
  fi
  return 1
}

list_cmd() {
  refresh_state
  local fields_arg="${FIELDS:-session_id,status,worktree,branch,pid,updated_at}"
  local sort_arg="${SORT_FIELD:-updated_at}"
  local filters="${FILTERS:-}"
  FIELDS_ARG="$fields_arg" SORT_ARG="$sort_arg" FILTER_ARG="$filters" flock "$STATE_LOCK" /usr/bin/env python3 - "$STATE_FILE" <<'PY'
import json, os, sys, time
from pathlib import Path

STATE_PATH = sys.argv[1]
DEFAULT_FIELDS = ["session_id", "status", "worktree", "branch", "pid", "updated_at"]

def parse_fields():
    raw = os.environ.get("FIELDS_ARG", "")
    fields = [f.strip() for f in raw.split(",") if f.strip()]
    return fields or DEFAULT_FIELDS

def parse_filters():
    filters = {}
    for entry in os.environ.get("FILTER_ARG", "").strip().splitlines():
        if not entry:
            continue
        key, _, values = entry.partition("=")
        value_list = [v.strip() for v in values.split(",") if v.strip()]
        if value_list:
            filters.setdefault(key.strip(), set()).update(value_list)
    return filters

def load_state(path):
    with open(path, "r", encoding="utf-8") as fh:
        return json.load(fh)

def load_session_logs():
    base = Path.home() / ".codex" / "sessions"
    meta = {}
    if not base.exists():
        return meta
    for root, _dirs, files in os.walk(base):
        files.sort()
        for name in files:
            if not name.endswith(".jsonl"):
                continue
            path = Path(root) / name
            try:
                with open(path, "r", encoding="utf-8") as fh:
                    first_line = fh.readline().strip()
            except (OSError, UnicodeDecodeError):
                continue
            if not first_line:
                continue
            try:
                entry = json.loads(first_line)
            except json.JSONDecodeError:
                continue
            payload = entry.get("payload", {})
            sid = payload.get("id")
            if not sid or sid in meta:
                continue
            git_info = entry.get("git") or payload.get("git") or {}
            meta[sid] = {
                "session_id": sid,
                "cwd": payload.get("cwd") or payload.get("workdir") or "",
                "timestamp": payload.get("timestamp") or entry.get("timestamp") or "",
                "path": str(path),
                "branch": git_info.get("branch") or "",
            }
    return meta

def scan_process_table():
    procs, children = {}, {}
    try:
        hz = os.sysconf("SC_CLK_TCK")
    except (AttributeError, ValueError):
        hz = 100
    try:
        with open("/proc/uptime", "r", encoding="utf-8") as fh:
            uptime = float(fh.read().split()[0])
        boot_time = time.time() - uptime
    except Exception:
        boot_time = None
    for name in os.listdir("/proc"):
        if not name.isdigit():
            continue
        pid = int(name)
        cmd_path = f"/proc/{name}/cmdline"
        stat_path = f"/proc/{name}/stat"
        cwd_path = f"/proc/{name}/cwd"
        try:
            with open(cmd_path, "rb") as fh:
                raw = fh.read()
        except Exception:
            continue
        parts = [p.decode("utf-8", "replace") for p in raw.split(b"\0") if p]
        if not parts:
            continue
        try:
            with open(stat_path, "r", encoding="utf-8") as fh:
                stat_parts = fh.read().split()
        except Exception:
            continue
        if len(stat_parts) < 22:
            continue
        try:
            ppid = int(stat_parts[3])
        except Exception:
            ppid = 0
        try:
            start_ticks = int(stat_parts[21])
        except Exception:
            start_ticks = None
        start_iso = ""
        if boot_time is not None and start_ticks is not None:
            start_seconds = boot_time + (start_ticks / hz)
            start_iso = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(start_seconds))
        try:
            cwd = os.readlink(cwd_path)
        except Exception:
            cwd = ""
        exe = parts[0]
        is_app_server = any(part == "app-server" or part.endswith("app-server") for part in parts[1:])
        is_codex = ("codex" in exe) and ("--yolo" in parts) and not is_app_server
        procs[pid] = {
            "pid": pid,
            "ppid": ppid,
            "cmdline": parts,
            "cmd": " ".join(parts),
            "cwd": cwd,
            "start_iso": start_iso,
            "is_codex": is_codex,
        }
        children.setdefault(ppid, []).append(pid)
    return procs, children

def build_managed_tree(children, roots):
    managed = set()
    queue = list(pid for pid in roots if isinstance(pid, int))
    while queue:
        pid = queue.pop()
        if pid in managed:
            continue
        managed.add(pid)
        for child in children.get(pid, ()):
            queue.append(child)
    return managed

fields = parse_fields()
sort_field = os.environ.get("SORT_ARG") or "updated_at"
filters = parse_filters()
state = load_state(STATE_PATH)
session_meta = load_session_logs()

sessions = state.get("sessions", {})
rows = []
session_rows = {}
for sid, payload in sessions.items():
    row = dict(payload)
    row.setdefault("session_id", sid)
    status = row.get("status") or "unknown"
    row["status"] = status
    pid = row.get("pid")
    if isinstance(pid, str) and pid.isdigit():
        row["pid"] = int(pid)
    meta = session_meta.get(row["session_id"])
    if meta:
        row.setdefault("session_file", meta["path"])
        if not row.get("worktree"):
            row["worktree"] = meta.get("cwd") or row.get("worktree") or ""
        if not row.get("branch"):
            row["branch"] = meta.get("branch") or row.get("branch") or ""
        row.setdefault("log_cwd", meta.get("cwd") or "")
    else:
        row.setdefault("session_file", "")
    rows.append(row)
    session_rows[row["session_id"]] = row

log_only_rows = []
for sid, meta in session_meta.items():
    if sid in session_rows:
        continue
    log_only_rows.append({
        "session_id": sid,
        "status": "log",
        "worktree": meta.get("cwd") or "",
        "branch": meta.get("branch") or "",
        "pid": "",
        "updated_at": meta.get("timestamp") or "",
        "session_file": meta.get("path"),
    })

procs, children = scan_process_table()
managed_roots = {row.get("pid") for row in rows if row.get("status") == "active" and isinstance(row.get("pid"), int)}
managed_tree = build_managed_tree(children, managed_roots)

unmanaged_rows = []
for pid, info in procs.items():
    if not info.get("is_codex"):
        continue
    if pid in managed_tree:
        continue
    session_id = f"unmanaged:pid-{pid}"
    unmanaged_rows.append({
        "session_id": session_id,
        "status": "unmanaged",
        "worktree": info.get("cwd") or "",
        "branch": "-",
        "pid": pid,
        "updated_at": info.get("start_iso") or "",
        "cmd": info.get("cmd"),
        "session_file": "",
    })

rows.extend(unmanaged_rows)
rows.extend(log_only_rows)

def matches(row):
    for key, allowed in filters.items():
        val = str(row.get(key, ""))
        if val not in allowed:
            return False
    return True

filtered = [row for row in rows if matches(row)]
filtered.sort(key=lambda item: str(item.get(sort_field, "")), reverse=True)

if not filtered:
    print("No sessions recorded.")
    raise SystemExit(0)

widths = {field: max(len(field), *(len(str(row.get(field, ""))) for row in filtered)) for field in fields}
header = " ".join(field.ljust(widths[field]) for field in fields)
print(header)
print("-" * len(header))
for row in filtered:
    print(" ".join(str(row.get(field, "")).ljust(widths[field]) for field in fields))
PY
}

view_cmd() {
  refresh_state
  local session_id="${VIEW_SESSION:-}"
  local worktree="${VIEW_WORKTREE:-}"
  if [[ -z "$session_id" && -z "$worktree" ]]; then
    echo "view requires --session <id> or --worktree <path>" >&2
    exit 2
  fi
  if [[ -n "$session_id" ]]; then
    local json
    if ! json="$(get_session_json "$session_id" 2>/dev/null)"; then
      echo "session not found: $session_id" >&2
      exit 1
    fi
    echo "$json" | jq .
    return
  fi
  local path_real
  path_real="$(realpath -m "$worktree")"
  WORKTREE_REAL="$path_real" flock "$STATE_LOCK" /usr/bin/env python3 - "$STATE_FILE" <<'PY'
import json, os, sys
target = os.environ["WORKTREE_REAL"]
with open(sys.argv[1], "r", encoding="utf-8") as fh:
    data = json.load(fh)
found = [s for s in data.get("sessions", {}).values() if os.path.realpath(s.get("worktree", "")) == target]
if not found:
    raise SystemExit(1)
for entry in found:
    print(json.dumps(entry, indent=2))
PY
}


parse_background_output() {
  local output="$1"
  printf '%s\n' "$output" | awk -F= '/^BACKGROUND_/ {gsub(/\r/,""); print}'
}

extract_session_id() {
  local log_path="$1"
  /usr/bin/env python3 - "$log_path" <<'PY'
import json, sys
path = sys.argv[1]
try:
    with open(path, "r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                data = json.loads(line)
            except json.JSONDecodeError:
                continue
            if data.get("type") == "session_meta":
                payload = data.get("payload", {})
                sid = payload.get("id")
                if sid:
                    print(sid)
                    break
            if data.get("type") == "thread.started":
                thread_id = data.get("thread_id")
                if thread_id and "-" in thread_id:
                    print(thread_id.rsplit("-", 1)[-1])
                    break
except FileNotFoundError:
    pass
PY
}

ensure_session_dir() {
  local sid="$1"
  local dir="$SESSION_DIR_ROOT/$sid"
  mkdir -p "$dir"
  printf '%s' "$dir"
}

run_hook() {
  local var_name="$1"
  local dir="$2"
  local session_id="${3:-}"
  local cmd="${!var_name:-}"
  if [[ -z "$cmd" ]]; then
    return
  fi
  echo "[agentx] running $var_name"
  (
    export AGENTX_HOOK_SESSION_ID="$session_id"
    cd "$dir"
    bash -lc "$cmd"
  )
}

run_cmd() {
  local worktree="$RUN_WORKTREE"
  if [[ -z "$worktree" ]]; then
    echo "--worktree is required" >&2
    exit 2
  fi
  if [[ ! -d "$worktree" ]]; then
    echo "worktree not found: $worktree" >&2
    exit 1
  fi
  worktree="$(realpath "$worktree")"
  local slug
  slug="$(basename "$worktree")"
  local branch
  branch="$(git -C "$worktree" rev-parse --abbrev-ref HEAD 2>/dev/null || echo unknown)"
  if [[ -n "${RUN_PROMPT_FILE:-}" && ! -f "$RUN_PROMPT_FILE" ]]; then
    echo "prompt file not found: $RUN_PROMPT_FILE" >&2
    exit 1
  fi

  local session_dir_hint=""
  if [[ -n "${RUN_SESSION:-}" ]]; then
    if ! session_exists "$RUN_SESSION"; then
      echo "session not found: $RUN_SESSION" >&2
      exit 1
    fi
    local resume_json
    resume_json="$(get_session_json "$RUN_SESSION")"
    local existing_status existing_worktree existing_dir
    existing_status="$(echo "$resume_json" | jq -r '.status')"
    existing_worktree="$(echo "$resume_json" | jq -r '.worktree // ""')"
    existing_dir="$(echo "$resume_json" | jq -r '.session_dir // empty')"
    if [[ "$existing_status" == "active" ]]; then
      echo "session $RUN_SESSION is already active; abort or await before rerunning" >&2
      exit 1
    fi
    if [[ -n "$existing_worktree" ]]; then
      local existing_real
      existing_real="$(realpath -m "$existing_worktree")"
      if [[ "$existing_real" != "$worktree" ]]; then
        echo "worktree mismatch: session uses $existing_real but --worktree is $worktree" >&2
        exit 1
      fi
    fi
    if [[ -n "$existing_dir" && "$existing_dir" != "null" ]]; then
      session_dir_hint="$existing_dir"
    fi
  fi

  local prompt_text=""
  if [[ -n "${RUN_PROMPT_FILE:-}" ]]; then
    prompt_text="$(<"$RUN_PROMPT_FILE")"
  elif [[ -n "${RUN_PROMPT:-}" ]]; then
    prompt_text="$RUN_PROMPT"
  else
    echo "agentx run requires --prompt or --prompt-file" >&2
    exit 2
  fi
  if [[ -n "${RUN_MESSAGE:-}" ]]; then
    prompt_text+=$'\n\nTicket owner message:\n'"${RUN_MESSAGE}"
  fi

  local session_id="${RUN_SESSION:-}"
  run_hook "AGENTX_HOOK_BEFORE_TURN_BEGIN" "$worktree" "$session_id"

  local temp_dir last_message_path session_dir
  temp_dir="$(mktemp -d "${CONFIG_DIR}/run-XXXXXX")"
  last_message_path="$temp_dir/last_message.md"
  local cmd=(codex exec --json --output-last-message "$last_message_path" --cd "$worktree")
  cmd+=("${RUN_EXTRA_ARGS[@]}")
  if [[ -n "$session_id" ]]; then
    cmd+=(resume "$session_id")
  fi
  cmd+=("$prompt_text")
run_hook "AGENTX_HOOK_BEFORE_TURN_LAUNCH" "$worktree" "$session_id"

  local bg_output
  if ! bg_output="$(background "${cmd[@]}")"; then
    echo "failed to launch codex exec" >&2
    exit 1
  fi
  echo "$bg_output"
  local parsed
  parsed="$(parse_background_output "$bg_output")"
  local bg_pid stdout_log stderr_log log_dir exit_log
  while IFS='=' read -r key value; do
    case "$key" in
      BACKGROUND_PID) bg_pid="$value" ;;
      BACKGROUND_LOG_DIR) log_dir="$value" ;;
      BACKGROUND_STDOUT) stdout_log="$value" ;;
      BACKGROUND_STDERR) stderr_log="$value" ;;
      BACKGROUND_EXITCODE) exit_log="$value" ;;
    esac
  done <<<"$parsed"
  if [[ -z "$stdout_log" ]]; then
    echo "unable to parse background stdout log path" >&2
    exit 1
  fi

  local resolved_session_id="$session_id"
  local codex_exit_status=""
  if [[ -z "$resolved_session_id" ]]; then
    for attempt in {1..80}; do
      resolved_session_id="$(extract_session_id "$stdout_log")"
      if [[ -n "$resolved_session_id" ]]; then
        break
      fi
      if [[ -s "$exit_log" ]]; then
        codex_exit_status="$(cat "$exit_log" 2>/dev/null || true)"
        break
      fi
      sleep 0.25
    done
  fi

  if [[ -z "$resolved_session_id" ]]; then
    echo "failed to determine session id; see $stdout_log" >&2
    exit 1
  fi

  if [[ -z "$codex_exit_status" && -s "$exit_log" ]]; then
    codex_exit_status="$(cat "$exit_log" 2>/dev/null || true)"
  fi

  session_dir="$(ensure_session_dir "$resolved_session_id")"

  local now
  now="$(ts_utc)"
  if session_exists "$resolved_session_id"; then
    local patch_json
    patch_json="$(jq -n \
      --arg status "active" \
      --arg pid "$bg_pid" \
      --arg stdout "$stdout_log" \
      --arg stderr "$stderr_log" \
      --arg exitlog "$exit_log" \
      --arg prompt "$prompt_text" \
    --arg lastmsg "$last_message_path" \
      --arg updated "$now" \
      '{status:$status,pid:($pid|tonumber),stdout_log:$stdout,stderr_log:$stderr,exitcode_log:$exitlog,last_message_path:$lastmsg,prompt_snapshot:$prompt,updated_at:$updated}')"
    apply_session_patch "$resolved_session_id" "$patch_json"
  else
    local payload
    payload="$(jq -n \
      --arg sid "$resolved_session_id" \
      --arg wt "$worktree" \
      --arg br "$branch" \
      --arg slug "$slug" \
      --arg status "active" \
      --arg pid "$bg_pid" \
      --arg stdout "$stdout_log" \
      --arg stderr "$stderr_log" \
      --arg exitlog "$exit_log" \
      --arg prompt "$prompt_text" \
      --arg sessiondir "$session_dir" \
      --arg lastmsg "$last_message_path" \
      --arg created "$now" \
      --arg updated "$now" \
      '{session_id:$sid, worktree:$wt, branch:$br, slug:$slug, status:$status, pid:($pid|tonumber), stdout_log:$stdout, stderr_log:$stderr, exitcode_log:$exitlog, session_dir:$sessiondir, last_message_path:$lastmsg, prompt_snapshot:$prompt, created_at:$created, updated_at:$updated, turns_completed:0, last_exitcode:null}')"
    insert_session "$payload"
  fi
  echo "Session $resolved_session_id running (PID $bg_pid). Stdout: $stdout_log"

  run_hook "AGENTX_HOOK_AFTER_TURN_LAUNCH" "$worktree" "$resolved_session_id"
}

abort_cmd() {
  local session_id="$1"
  if [[ -z "$session_id" ]]; then
    echo "abort requires --session <id>" >&2
    exit 2
  fi
  local json
  if ! json="$(get_session_json "$session_id" 2>/dev/null)"; then
    echo "session not found: $session_id" >&2
    exit 1
  fi
  local pid exit_log
  pid="$(echo "$json" | jq -r '.pid // empty')"
  exit_log="$(echo "$json" | jq -r '.exitcode_log // empty')"
  if [[ -z "$pid" ]]; then
    echo "session not running (no pid)" >&2
    exit 1
  fi
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    sleep 1
    kill -KILL "$pid" 2>/dev/null || true
    echo "sent SIGTERM/SIGKILL to $pid"
  else
    echo "process $pid not running" >&2
  fi
  if [[ -n "$exit_log" ]]; then
    printf '143\n' >"$exit_log" 2>/dev/null || true
  fi
  apply_session_patch "$session_id" '{"status":"stopped","pid":null}'
  local worktree
  worktree="$(echo "$json" | jq -r '.worktree')"
  run_hook "AGENTX_HOOK_AFTER_TURN_ABORTED" "$worktree" "$session_id"
}

archive_cmd() {
  local session_id="$1"
  local delete_worktree="$2"
  if [[ -z "$session_id" ]]; then
    echo "archive requires --session <id>" >&2
    exit 2
  fi
  local json
  if ! json="$(get_session_json "$session_id" 2>/dev/null)"; then
    echo "session not found: $session_id" >&2
    exit 1
  fi
  local status
  status="$(echo "$json" | jq -r '.status')"
  if [[ "$status" == "active" ]]; then
    echo "session $session_id is still active; abort or await before archiving" >&2
    exit 1
  fi
  apply_session_patch "$session_id" '{"status":"archived","pid":null}'
  if [[ "$delete_worktree" == "1" ]]; then
    local worktree
    worktree="$(echo "$json" | jq -r '.worktree')"
    if [[ -n "$worktree" && -d "$worktree" ]]; then
      local real
      real="$(realpath "$worktree")"
      if [[ "$real" == "$(realpath /workspaces/rust-viterbo/.persist/agentx/worktrees)"/* ]]; then
        rm -rf "$real"
        echo "deleted worktree $real"
      else
        echo "refusing to delete non-agentx worktree $real" >&2
      fi
    fi
  fi
}

await_cmd() {
  local session_id="$1"
  if [[ -z "$session_id" ]]; then
    echo "await requires --session <id>" >&2
    exit 2
  fi
  local json
  if ! json="$(get_session_json "$session_id" 2>/dev/null)"; then
    echo "session not found: $session_id" >&2
    exit 1
  fi
  local exit_log last_message worktree
  exit_log="$(echo "$json" | jq -r '.exitcode_log')"
  last_message="$(echo "$json" | jq -r '.last_message_path')"
  worktree="$(echo "$json" | jq -r '.worktree')"
  if [[ -z "$exit_log" ]]; then
    echo "no exitcode log recorded" >&2
    exit 1
  fi
  echo "Waiting for session $session_id (exit log: $exit_log)"
  while true; do
    if [[ -s "$exit_log" ]]; then
      local code
      code="$(cat "$exit_log")"
      echo "Session finished with exit code: $code"
      break
    fi
    sleep 1
  done
  if [[ -f "$last_message" ]]; then
    echo "---- Final message ----"
    cat "$last_message"
    echo "-----------------------"
  fi
  local code_numeric="$code"
  if [[ ! "$code_numeric" =~ ^-?[0-9]+$ ]]; then
    code_numeric="null"
  fi
  local now
  now="$(ts_utc)"
  local patch_json
  if [[ "$code_numeric" == "null" ]]; then
    patch_json="$(jq -n --arg status "inactive" --arg updated "$now" '{status:$status,pid:null,last_exitcode:null,updated_at:$updated}')"
  else
    patch_json="$(jq -n --arg status "inactive" --arg updated "$now" --argjson exitcode "$code_numeric" '{status:$status,pid:null,last_exitcode:$exitcode,updated_at:$updated}')"
  fi
  apply_session_patch "$session_id" "$patch_json"
  if [[ "$code_numeric" == "null" || "$code_numeric" -ne 0 ]]; then
    run_hook "AGENTX_HOOK_AFTER_TURN_END_FAILURE" "$worktree" "$session_id"
  else
    run_hook "AGENTX_HOOK_AFTER_TURN_END_SUCCESS" "$worktree" "$session_id"
  fi
  refresh_state
}

COMMAND="${1:-}"
if [[ -z "$COMMAND" || "$COMMAND" == "help" ]]; then
  usage
  exit 0
fi
shift

case "$COMMAND" in
  list)
    FIELDS=""
    SORT_FIELD=""
    FILTERS=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --fields) FIELDS="$2"; shift 2 ;;
        --sort-by) SORT_FIELD="$2"; shift 2 ;;
        --filter) FILTERS+="${2}"$'\n'; shift 2 ;;
        --help|-h) usage; exit 0 ;;
        *) echo "unknown flag $1"; exit 2 ;;
      esac
    done
    list_cmd
    ;;
  view)
    VIEW_SESSION=""
    VIEW_WORKTREE=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --session) VIEW_SESSION="$2"; shift 2 ;;
        --worktree) VIEW_WORKTREE="$2"; shift 2 ;;
        --help|-h) usage; exit 0 ;;
        *) echo "unknown flag $1"; exit 2 ;;
      esac
    done
    view_cmd
    ;;
  run)
    RUN_WORKTREE=""
    RUN_PROMPT=""
    RUN_PROMPT_FILE=""
    RUN_MESSAGE=""
    RUN_SESSION=""
    RUN_EXTRA_ARGS=()
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --worktree) RUN_WORKTREE="$2"; shift 2 ;;
        --prompt) RUN_PROMPT="$2"; shift 2 ;;
        --prompt-file) RUN_PROMPT_FILE="$2"; shift 2 ;;
        --message) RUN_MESSAGE="$2"; shift 2 ;;
        --session) RUN_SESSION="$2"; shift 2 ;;
        --) shift; RUN_EXTRA_ARGS+=("$@"); break ;;
        *) RUN_EXTRA_ARGS+=("$1"); shift ;;
      esac
    done
    run_cmd
    ;;
  abort)
    SESSION_ID=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --session) SESSION_ID="$2"; shift 2 ;;
        --help|-h) usage; exit 0 ;;
        *) echo "unknown flag $1"; exit 2 ;;
      esac
    done
    abort_cmd "$SESSION_ID"
    ;;
  archive)
    SESSION_ID=""
    DELETE_FLAG="0"
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --session) SESSION_ID="$2"; shift 2 ;;
        --delete-worktree) DELETE_FLAG="1"; shift ;;
        --help|-h) usage; exit 0 ;;
        *) echo "unknown flag $1"; exit 2 ;;
      esac
    done
    archive_cmd "$SESSION_ID" "$DELETE_FLAG"
    ;;
  await)
    SESSION_ID=""
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --session) SESSION_ID="$2"; shift 2 ;;
        --help|-h) usage; exit 0 ;;
        *) echo "unknown flag $1"; exit 2 ;;
      esac
    done
    await_cmd "$SESSION_ID"
    ;;
  *)
    echo "unknown command: $COMMAND" >&2
    usage
    exit 2
    ;;
esac
