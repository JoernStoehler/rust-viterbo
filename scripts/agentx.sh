#!/bin/bash
## AgentX — Ticket Management CLI (folder‑based, agent‑first)
##
## Use: 'agentx --help' (after install) or 'scripts/agentx.sh --help' (from repo).
##
## Purpose and Invariants (read this first)
## - AgentX manages “tickets” as folders and Git worktrees for an agent‑first workflow.
## - All state created by AgentX lives under this repository:
##   - Bundles:   .persist/agentx/tickets/<slug>/
##   - Worktrees: .persist/agentx/worktrees/<slug>/  (checked out to branch 'ticket/<slug>')
##   - Runtime:   <worktree>/.tx/
##   - Symlinks:  shared/tickets at repo root and at each worktree root -> .persist/agentx/tickets
## - AgentX must not write outside those locations (besides normal Git ops inside the worktree).
## - Slugs are short filesystem‑safe identifiers: ^[a-z0-9][a-z0-9._-]{0,63}$
## - Hooks (AGENTX_HOOK_*) always run if set, but are bounded to 10s each for hygiene.
## - Agent turns can be long; default run cap is 10 hours (AGENTX_RUN_TIMEOUT=36000). Set 0 to disable.
##
## Model (files + events):
## - Ticket = folder: .persist/agentx/tickets/<slug>/
##   - meta.yml  : minimal operational state (status, optional owner/depends_on/dependency_of).
##   - body.md   : spec text for humans (stable after provision unless asked).
##   - messages  : immutable event files named YYYYMMDDThhmmssZ-<event>.md
##                 where <event> ∈ { provision, tNN-start, tNN-final, tNN-abort } (NN=01..99).
## - Event rules:
##   - One 'provision' before any turn.
##   - Each turn N: exactly one 'tNN-start' and exactly one terminal: 'tNN-final' OR 'tNN-abort'.
##   - Turns strictly increase (t01, t02, …). No 'resume' event; a new turn starts with 't(N+1)-start'.
## - Filenames:
##   - UTC timestamp prefix controls ordering: YYYYMMDDThhmmssZ-...
##   - Same‑second collisions are bumped by 1s.
##
## meta.yml (minimal, human‑editable):
##   - Required: status (open|active|done|stopped)
##   - Optional: depends_on: [slug,...], dependency_of: [slug,...], owner: <string>
##   - Everything else (slug, branch, worktree, timestamps, turns) is derived by AgentX.

set -euo pipefail

# ---- Configuration (single source of truth) ----
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
INSTALL_NAME="${INSTALL_NAME:-agentx}"

# Persist across container rebuilds under repo-local .persist
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
PERSIST_ROOT_DEFAULT="${REPO_ROOT}/.persist/agentx"
AGENTX_TICKETS_DIR="${AGENTX_TICKETS_DIR:-${PERSIST_ROOT_DEFAULT}/tickets}"
AGENTX_TICKETS_MIGRATED="${AGENTX_TICKETS_MIGRATED:-${AGENTX_TICKETS_DIR}/migrated}"
AGENTX_WORKTREES_DIR="${AGENTX_WORKTREES_DIR:-${PERSIST_ROOT_DEFAULT}/worktrees}"
GLOBAL_TMUX_SESSION="${GLOBAL_TMUX_SESSION:-tickets}"
# Symlink path (created inside each worktree and the main repo)
LOCAL_TICKET_FOLDER=${LOCAL_TICKET_FOLDER:-"./shared/tickets"}
AGENTX_RUN_TIMEOUT="${AGENTX_RUN_TIMEOUT:-36000}"  # seconds; 0 disables run cap
AGENTX_RUNNER_QUEUE="${AGENTX_RUNNER_QUEUE:-${PERSIST_ROOT_DEFAULT}/queue}"
export AGENTX_RUNNER_QUEUE

# Optional hooks (env), run inside the worktree:
AGENTX_HOOK_START="${AGENTX_HOOK_START:-}"
AGENTX_HOOK_RESUME="${AGENTX_HOOK_RESUME:-}"
AGENTX_HOOK_BEFORE_RUN="${AGENTX_HOOK_BEFORE_RUN:-}"
AGENTX_HOOK_AFTER_RUN="${AGENTX_HOOK_AFTER_RUN:-}"
AGENTX_HOOK_PROVISION="${AGENTX_HOOK_PROVISION:-}"

# ---- Logging / utils ----
_log_info() { printf '[agentx] %s\n' "$*" >&2; }
_log_warn() { printf '[agentx][warn] %s\n' "$*" >&2; }
_log_err()  { printf '[agentx][err] %s\n' "$*" >&2; }
_die() { _log_err "$*"; exit 1; }
_require_cmd() { command -v "$1" >/dev/null 2>&1 || _die "Missing dependency: $1. Please install it and retry."; }

_validate_slug() {
  local s="${1:-}"
  [[ "$s" =~ ^[a-z0-9][a-z0-9._-]{0,63}$ ]] || _die "invalid slug: '$s'. Allowed: ^[a-z0-9][a-z0-9._-]{0,63}$"
}

_dest_abs_from_repo_root() {
  local rel="${1:-}"
  if [[ "$rel" = /* ]]; then printf '%s' "$rel"; else printf '%s/%s' "$REPO_ROOT" "${rel#./}"; fi
}

_ensure_symlink_exact() {
  # Create a symlink only if it does not exist. If it exists, it must be an exact symlink to target.
  # Never clobber mismatched paths.
  local dest="$1"; local target="$2"
  mkdir -p "$(dirname "$dest")"
  if [ -L "$dest" ]; then
    local cur; cur="$(readlink "$dest")" || cur=""
    if [ "$cur" != "$target" ]; then
      _die "symlink mismatch at '$dest' (found -> $cur, expected -> $target). Remove/fix it and retry."
    fi
  elif [ -e "$dest" ]; then
    _die "path exists and is not the expected symlink: '$dest'. Remove/fix it and retry."
  else
    ln -s "$target" "$dest"
  fi
}

_run_with_timeout() {
  # Prefer 'safe' when available; otherwise fall back to GNU timeout.
  local seconds="$1"; shift
  if [ "$seconds" -le 0 ]; then
    # No timeout requested
    bash -lc "$*"
    return $?
  fi
  if command -v safe >/dev/null 2>&1; then
    safe -t "$seconds" -- bash -lc "$*"
  else
    _require_cmd timeout
    timeout --kill-after=10 "${seconds}s" bash -lc "$*"
  fi
}

_runner_cmd() {
  _require_cmd python3
  PYTHONPATH="$REPO_ROOT/scripts" python3 "$REPO_ROOT/scripts/agentx_runner.py" "$@"
}

_ensure_folders() {
  mkdir -p "${AGENTX_TICKETS_DIR}" "${AGENTX_TICKETS_MIGRATED}" "${AGENTX_WORKTREES_DIR}" "${AGENTX_RUNNER_QUEUE}"
  # Ensure the repo-root symlink exists and points exactly to the tickets dir.
  local dest; dest="$(_dest_abs_from_repo_root "${LOCAL_TICKET_FOLDER}")"
  _ensure_symlink_exact "$dest" "${AGENTX_TICKETS_DIR}"
}
_timestamp() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }
_slug_to_branch(){ printf 'ticket/%s' "$1"; }
_slug_to_worktree(){ printf '%s/%s' "$AGENTX_WORKTREES_DIR" "$1"; }

# ---- YAML helpers (flat top-level scalars) ----
_yaml_get() {
  local f="$1"; local key="$2"
  [ -f "$f" ] || { echo ""; return 0; }
  awk -v key="$key" '
    $0 ~ "^[[:space:]]*"key":[[:space:]]*" {
      sub("^[[:space:]]*"key":[[:space:]]*","",$0);
      print $0; exit
    }
  ' "$f" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'
}
_yaml_set() {
  local f="$1"; local key="$2"; local val="$3"
  local tmp; tmp="$(mktemp)"
  awk -v key="$key" -v val="$val" '
    BEGIN{done=0}
    $0 ~ "^[[:space:]]*"key":[[:space:]]*" { if(!done){ print key": "val; done=1 }; next }
    { print $0 }
    END{ if(!done){ print key": "val } }
  ' "$f" >"$tmp"
  mv "$tmp" "$f"
}
_set_meta_many() {
  local slug="$1"; shift
  local meta="$(_meta_path "$slug")"
  mkdir -p "$(dirname "$meta")"
  touch "$meta"
  local kv
  for kv in "$@"; do
    local k="${kv%%=*}"
    local v="${kv#*=}"
    _yaml_set "$meta" "$k" "$v"
  done
}

# ---- Bundle helpers ----
_bundle_dir(){ printf '%s/%s' "$AGENTX_TICKETS_DIR" "$1"; }
_meta_path(){ printf '%s/meta.yml' "$(_bundle_dir "$1")"; }
_body_path(){ printf '%s/body.md' "$(_bundle_dir "$1")"; }
_list_messages(){
  local slug="$1"
  (cd "$(_bundle_dir "$slug")" 2>/dev/null && ls -1 2>/dev/null | grep -E '^[0-9]{8}T[0-9]{6}Z-(provision|t[0-9]{2}-(start|final|abort))\.md$' | sort) || true
}
_last_message_ts(){
  local slug="$1"
  local last; last="$(_list_messages "$slug" | tail -n1)"
  [ -n "$last" ] || { echo ""; return 0; }
  printf '%s' "${last%%-*}"
}
_next_ts(){
  local slug="$1"
  local now last next
  now="$(date -u +'%Y%m%dT%H%M%SZ')"
  last="$(_last_message_ts "$slug")"
  if [ -z "$last" ] || [[ "$now" > "$last" ]]; then
    printf '%s' "$now"; return 0
  fi
  next="$(date -u -d "${last:0:8} ${last:9:2}:${last:11:2}:${last:13:2} UTC + 1 second" +'%Y%m%dT%H%M%SZ' 2>/dev/null || true)"
  [ -n "$next" ] || next="$now"
  printf '%s' "$next"
}
_next_turn(){
  local slug="$1"
  local last_turn
  last_turn="$(_list_messages "$slug" | sed -n 's/^.*-t\([0-9][0-9]\)-.*$/\1/p' | tail -n1)"
  if [ -z "$last_turn" ]; then printf '01'; else printf '%02d' "$((10#$last_turn + 1))"; fi
}
_write_message_file(){
  local slug="$1"; local event="$2"; local turn="${3:-}"; local actor="${4:-agentx}"; local body="${5:-}"
  local dir="$(_bundle_dir "$slug")"
  mkdir -p "$dir"
  local ts fname
  ts="$(_next_ts "$slug")"
  case "$event" in
    provision) fname="${ts}-provision.md";;
    start)     fname="${ts}-t$(printf '%02d' "$turn")-start.md";;
    final)     fname="${ts}-t$(printf '%02d' "$turn")-final.md";;
    abort)     fname="${ts}-t$(printf '%02d' "$turn")-abort.md";;
    *) _die "unknown event: $event";;
  esac
  local tmp; tmp="$(mktemp "${dir}/.msg.XXXXXX")"
  {
    echo '---'
    echo "event: $event"
    if [ -n "$turn" ]; then echo "turn: $turn"; fi
    echo "ts: $(_timestamp)"
    echo "actor: $actor"
    echo '---'
    [ -n "$body" ] && printf '%s\n' "$body"
  } >"$tmp"
  mv "$tmp" "${dir}/${fname}"
}

# ---- usage ----
usage() {
  cat <<EOF
Usage: $(basename "$0") <command> [arguments]

Base (primitives, slug-only):
  provision|new <slug> [--body-file <path>] [--inherit-from <slug>] [--base <ref>] [--copy <path> ...]
      Create the ticket bundle and branch/worktree for <slug> without launching the agent.
  run <slug> [--message <text>]
      Start a new agent turn for <slug> inside tmux; records start/final messages and status.
  stop|abort <slug>
      Record an abort for the active turn (status=stopped) and kill the tmux window if present.

Convenience (agent lifecycle):
  start <slug> [--message <text>]
      provision (if absent) + run (first-time or subsequent).
  await <slug> [--timeout <seconds>]
      Wait until meta.yml status changes from 'active' or timeout.

Convenience (ticket bundles):
  read|tail <slug> [--events <N>] [--json]
      Print meta.yml, body.md location, and the last N message files (parsed if --json).
  info <slug> [--fields a,b,c]
      Show ticket metadata (from meta.yml).
  list [--status <status>] [--fields a,b,c]
      List tickets with optional filters (from meta.yml).

Convenience (git workflows):
  merge <from-slug> [<into-slug>]
      Merge the completed ticket <from-slug> into <into-slug>'s branch/worktree (or infer from CWD).

Debug (read-only):
  doctor <slug>
      Verbose health info: tmux session/window, pane PIDs, worktree presence, and mismatches.

Tooling:
  install
      Copy this script to $INSTALL_DIR/$INSTALL_NAME (defaults shown).
  help
      Show this help message.

Policies:
- Slugs: ^[a-z0-9][a-z0-9._-]{0,63}$. Branch: ticket/<slug>.
- Writes: only under .persist/agentx/{tickets,worktrees}/ and <worktree>/.tx/. 
- Symlink: 'shared/tickets' exists only at repo root and worktree roots and must point to .persist/agentx/tickets.
- Hooks: always run if set, each bounded to 10s.
- Runs: default cap AGENTX_RUN_TIMEOUT=36000s (10h). Set 0 to disable.
EOF
}

# ---- Commands ----

install(){
  local dir="${INSTALL_DIR}"
  local name="${INSTALL_NAME}"
  [ -d "$dir" ] || _die "install: directory not found: ${dir}. Create it or set INSTALL_DIR to an existing writable directory (e.g., \$HOME/.local/bin)."
  [ -w "$dir" ] || _die "install: no write permission to ${dir}. Set INSTALL_DIR to a writable directory or run with appropriate privileges."
  echo "Installing agentx to ${dir}/${name}..."
  cp "$0" "${dir}/${name}"
  chmod +x "${dir}/${name}"
  echo "Installed agentx."
}

provision() {
  local slug="${1:-}"; shift || true
  [ -n "$slug" ] || _die "provision: missing <slug>"
  _validate_slug "$slug"
  local inherit_from="" base_ref="" copies=()
  local body_file=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --body-file) shift; body_file="${1-}";;
      --inherit-from) shift; inherit_from="${1-}";;
      --base) shift; base_ref="${1-}";;
      --copy) shift; copies+=("${1-}");;
      *) _die "provision: unknown arg: $1";;
    esac; shift || true
  done
  _require_cmd git
  _ensure_folders

  local branch worktree now dir meta body
  branch="$(_slug_to_branch "$slug")"
  worktree="$(_slug_to_worktree "$slug")"
  now="$(_timestamp)"

  # Determine base source
  local base_worktree="" base_branch=""
  if [ -n "$inherit_from" ]; then
    base_worktree="$(_slug_to_worktree "$inherit_from")"
    base_branch="$(git -C "$base_worktree" rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
    [ -d "$base_worktree" ] || _die "provision: --inherit-from worktree does not exist: $base_worktree"
    [ -n "$base_branch" ] || _die "provision: could not detect inherit-from branch."
  fi

  # Prepare bundle folder
  dir="$(_bundle_dir "$slug")"; meta="$(_meta_path "$slug")"; body="$(_body_path "$slug")"
  if [ -d "$dir" ]; then _log_warn "Ticket bundle already exists: $dir"; else mkdir -p "$dir"; fi
  touch "$meta" "$body"
  # Minimal meta
  _set_meta_many "$slug" "status=open"
  if [ -n "$body_file" ] && [ -f "$body_file" ]; then
    cp -f "$body_file" "$body"
  elif [ ! -s "$body" ]; then
    printf '# Ticket: %s\n' "$slug" >"$body"
  fi

  # Create branch/worktree
  local root; root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
  [ -n "$root" ] || _die "Not in a git repository."
  if [ -d "$worktree" ]; then
    _log_warn "Worktree already exists: $worktree"
  else
    ( cd "$root"
      git fetch -q origin || true
      if git show-ref --verify --quiet "refs/heads/$branch"; then
        _die "provision: branch already exists: $branch"
      fi
      if [ -n "$base_branch" ]; then
        git worktree add "$worktree" -b "$branch" "$base_branch"
      else
        local base="${base_ref:-origin/main}"
        git worktree add "$worktree" -b "$branch" "$base" 2>/dev/null || git worktree add "$worktree" -b "$branch" main
      fi
    )
    _log_info "Created worktree: $worktree"
  fi
  mkdir -p "$(dirname "$worktree/$LOCAL_TICKET_FOLDER")"
  _ensure_symlink_exact "$worktree/$LOCAL_TICKET_FOLDER" "$AGENTX_TICKETS_DIR"

  # Optional provision hook (runs inside the new worktree)
  if [ -n "$AGENTX_HOOK_PROVISION" ]; then
    ( cd "$worktree" && _run_with_timeout 10 "$AGENTX_HOOK_PROVISION" ) || _log_warn "Hook failed: AGENTX_HOOK_PROVISION"
  fi

  # Copy paths strictly if requested
  if [ "${#copies[@]}" -gt 0 ]; then
    [ -n "$base_worktree" ] || _die "provision: --copy requires --inherit-from to define source worktree."
    local p src dst dstdir
    for p in "${copies[@]}"; do
      if [[ "$p" == *:* ]]; then src="${p%%:*}"; dst="${p#*:}"; else src="$p"; dst="$p"; fi
      [ -e "$base_worktree/$src" ] || _die "provision: copy source not found: $base_worktree/$src"
      dstdir="$(dirname "$worktree/$dst")"
      mkdir -p "$dstdir"
      cp -a "$base_worktree/$src" "$worktree/$dst"
      _log_info "Copied: $src -> $dst"
    done
  fi

  # Message: provision
  if ! ( _list_messages "$slug" | grep -q -- '-provision\.md$' ); then
    _write_message_file "$slug" "provision" "" "agentx" "provisioned branch=$branch worktree=$worktree"
  fi
  _log_info "Provisioned ticket '$slug' at $worktree on $branch."
}

run(){
  _require_cmd python3
  local slug="${1:-}"; shift || true
  [ -n "$slug" ] || _die "run: missing <slug>"
  _validate_slug "$slug"
  local message=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --message) shift; message="${1-}";;
      *) _die "run: unknown arg: $1";;
    esac; shift || true
  done
  if [ ! -d "$(_bundle_dir "$slug")" ]; then provision "$slug"; fi
  _runner_cmd run "$slug" --message "$message"
}

abort(){
  _require_cmd tmux
  local slug="${1:-}"; shift || true
  [ -n "${slug}" ] || _die "abort: missing <slug>"
  _validate_slug "$slug"
  local now="$(_timestamp)"
  if tmux list-windows -t "$GLOBAL_TMUX_SESSION" 2>/dev/null | awk '{print $2}' | sed 's/:$//' | grep -qx "$slug"; then
    tmux kill-window -t "${GLOBAL_TMUX_SESSION}:${slug}" || true
    _log_info "Killed tmux window '${GLOBAL_TMUX_SESSION}:${slug}'."
  else
    _log_warn "No tmux window '${GLOBAL_TMUX_SESSION}:${slug}' found."
  fi
  local nt; nt="$(_next_turn "$slug")"; local prev=$((10#$nt - 1))
  if [ "$prev" -ge 1 ]; then _write_message_file "$slug" "abort" "$prev" "agentx" ""; fi
  _set_meta_many "$slug" "status=stopped"
  _log_info "Ticket '$slug' marked stopped."
}

start(){
  _require_cmd git
  _ensure_folders
  local slug="${1:-}"; shift || true
  [ -n "${slug}" ] || _die "start: missing <slug>. See: agentx.sh --help"
  _validate_slug "$slug"
  local message=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --message) shift; message="${1-}";;
      *) _die "start: unknown arg: $1";;
    esac; shift || true
  done
  if [ ! -d "$(_bundle_dir "$slug")" ]; then provision "$slug"; fi
  local queue_path
  queue_path="$(_runner_cmd queue "$slug" --message "$message")"
  _log_info "Queued run for '$slug': ${queue_path}. Ensure 'bash scripts/agentx.sh service' is running."
}
stop(){ abort "$@"; }

service(){
  local once=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --once) once=1 ;;
      *) _die "service: unknown arg: $1" ;;
    esac
    shift || true
  done
  _ensure_folders
  if [ "$once" -eq 1 ]; then
    _runner_cmd service --once
  else
    _runner_cmd service
  fi
}

info(){
  local slug="${1:-}"; shift || true
  [ -z "${slug}" ] && _die "info: missing <slug>"
  _validate_slug "$slug"
  local fields_csv=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --fields) shift; fields_csv="${1-}";;
      *) _die "info: unknown arg: $1";;
    esac
    shift || true
  done
  local meta="$(_meta_path "$slug")"
  [ -f "$meta" ] || _die "info: ticket not found for slug '$slug'"
  local keys
  if [ -n "$fields_csv" ]; then IFS=',' read -r -a keys <<<"$fields_csv"; else keys=(slug status owner depends_on dependency_of branch worktree); fi
  local k
  for k in "${keys[@]}"; do
    case "$k" in
      slug) printf 'slug: %s\n' "$slug" ;;
      branch) printf 'branch: %s\n' "$(_slug_to_branch "$slug")" ;;
      worktree) printf 'worktree: %s\n' "$(_slug_to_worktree "$slug")" ;;
      *) printf '%s: %s\n' "$k" "$(_yaml_get "$meta" "$k")" ;;
    esac
  done
  printf 'bundle: %s\n' "$(_bundle_dir "$slug")"
}

await(){
  local slug="${1:-}"; shift || true
  [ -z "${slug}" ] && _die "await: missing <slug>"
  _validate_slug "$slug"
  local timeout=60
  while [ $# -gt 0 ]; do
    case "$1" in
      --timeout) shift; timeout="${1-}";;
      *) _die "await: unknown arg: $1";;
    esac; shift || true
  done
  local meta="$(_meta_path "$slug")"
  [ -f "$meta" ] || _die "await: ticket not found for slug '$slug'"
  local s="$(_yaml_get "$meta" "status")"
  if [ "$s" != "active" ]; then _log_info "Ticket not active (status=$s). Returning immediately."; exit 0; fi
  local start_ts end_ts now; start_ts="$(date +%s)"; end_ts=$(( start_ts + timeout ))
  while true; do
    now="$(date +%s)"; s="$(_yaml_get "$meta" "status")"
    if [ "$s" != "active" ]; then _log_info "Ticket status changed to '$s'."; exit 0; fi
    if [ "$now" -ge "$end_ts" ]; then _log_warn "Timeout while waiting for ticket to finish."; exit 1; fi
    sleep 2
  done
}

list(){
  local status_filter="" fields_csv=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --status) shift; status_filter="${1-}";;
      --fields) shift; fields_csv="${1-}";;
      *) _die "list: unknown arg: $1";;
    esac; shift || true
  done
  mkdir -p "$AGENTX_TICKETS_DIR"
  local metas
  mapfile -t metas < <(find "$AGENTX_TICKETS_DIR" -maxdepth 1 -mindepth 1 -type d | sort | sed 's#$#/#' | xargs -r -I{} bash -lc 'test -f "{}meta.yml" && echo "{}meta.yml"')
  local keys; if [ -n "$fields_csv" ]; then IFS=',' read -r -a keys <<<"$fields_csv"; else keys=(slug status owner); fi
  printf '%s' "${keys[0]}"
  local i; for ((i=1;i<${#keys[@]};i++)); do printf '\t%s' "${keys[$i]}"; done
  printf '\n'
  local f s g
  for f in "${metas[@]}"; do
    s="$(_yaml_get "$f" "status")"
    if [ -n "$status_filter" ] && [ "$s" != "$status_filter" ]; then continue; fi
    g="$(basename "$(dirname "$f")")"
    local row=()
    for k in "${keys[@]}"; do
      case "$k" in
        slug) row+=("$g");;
        status) row+=("$s");;
        *) row+=("$(_yaml_get "$f" "$k")");;
      esac
    done
    printf '%s' "${row[0]}"
    for ((i=1;i<${#row[@]};i++)); do printf '\t%s' "${row[$i]}"; done
    printf '\n'
  done
}

read_bundle(){
  local slug="${1:-}"; shift || true
  [ -n "$slug" ] || _die "read: missing <slug>"
  _validate_slug "$slug"
  local events=10 json=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --events) shift; events="${1-}";;
      --json) json=1;;
  *) _die "read: unknown arg: $1";;
    esac; shift || true
  done
  local dir="$(_bundle_dir "$slug")"; local meta="$(_meta_path "$slug")"; local body="$(_body_path "$slug")"
  [ -d "$dir" ] || _die "read: ticket not found for slug '$slug'"
  if [ "$json" -eq 1 ]; then
    printf '{\n'
    printf '  "meta": "%s",\n' "$meta"
    printf '  "body": "%s",\n' "$body"
    printf '  "events": [\n'
    local first=1
    while IFS= read -r f; do
      [ "$first" -eq 1 ] || printf ',\n'
      first=0
      local base; base="$(basename "$f")"
      local ts="${base%%-*}"; local rest="${base#*-}"; rest="${rest%.md}"
      local event; event="$rest"
      printf '    {"file":"%s","ts":"%s","event":"%s"}' "$f" "$ts" "$event"
    done < <(_list_messages "$slug" | tail -n "$events" | sed "s#^#${dir}/#")
    printf '\n  ]\n}\n'
  else
    printf 'meta: %s\nbody: %s\nevents (last %s):\n' "$meta" "$body" "$events"
    _list_messages "$slug" | tail -n "$events"
  fi
}

_tmux_ensure_session() {
  local sess="$1"
  if ! tmux has-session -t "$sess" 2>/dev/null; then
    # Create a detached session with a persistent shell window.
    # Do NOT kill the only window; tmux deletes the session when its last window exits.
    tmux new-session -d -s "$sess" -n "home"
    _log_info "Created tmux session '$sess'."
  fi
}

merge(){
  local child="${1:-}"; local parent="${2:-}"
  [ -n "$child" ] || _die "merge: missing <from-slug>"
  _validate_slug "$child"
  if [ -z "$parent" ]; then
    local cwd_root; cwd_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"
    [ -n "$cwd_root" ] || _die "merge: not inside a git worktree; specify <into> explicitly."
    local slug cand
    for cand in "$AGENTX_TICKETS_DIR"/*; do
      [ -d "$cand" ] || continue
      slug="$(basename "$cand")"
      if [ "$(_slug_to_worktree "$slug")" = "$cwd_root" ]; then parent="$slug"; break; fi
    done
    [ -n "$parent" ] || _die "merge: could not infer <into> from CWD. Pass it explicitly."
  fi
  _validate_slug "$parent"
  _require_cmd git
  local child_file="$(_meta_path "$child")"; [ -f "$child_file" ] || _die "merge: unknown child slug '$child'"
  local parent_file="$(_meta_path "$parent")"; [ -f "$parent_file" ] || _die "merge: unknown parent slug '$parent'"
  local c_status="$(_yaml_get "$child_file" 'status')"
  [ "$c_status" = "done" ] || _die "merge: from-ticket is not 'done' (status=$c_status)"
  local c_branch="$(_slug_to_branch "$child")"
  local p_worktree="$(_slug_to_worktree "$parent")"
  local p_branch="$(_slug_to_branch "$parent")"
  [ -d "$p_worktree" ] || _die "merge: into-worktree not found: $p_worktree"
  ( cd "$p_worktree"
    git fetch -q origin || true
    git checkout "$p_branch" >/dev/null 2>&1 || true
    # Enforce clean tree and fast-forward only to avoid messy merges.
    if ! git diff --quiet --ignore-submodules --; then
      _die "merge: target worktree has uncommitted changes. Commit/stash to proceed."
    fi
    _log_info "Merging (fast-forward) '$c_branch' into '$p_branch' in $p_worktree"
    git merge --ff-only "$c_branch"
    # Show the files changed by the fast-forward (if any)
    git diff --name-only HEAD@{1}..HEAD 2>/dev/null | sed 's/^/[changed] /' || true
  )
}

doctor(){
  local slug="${1:-}"; shift || true
  [ -n "$slug" ] || _die "doctor: missing <slug>"
  _validate_slug "$slug"
  local wt="$(_slug_to_worktree "$slug")"
  printf 'slug: %s\n' "$slug"
  printf 'worktree: %s\n' "$wt"
  if tmux has-session -t "$GLOBAL_TMUX_SESSION" 2>/dev/null; then
    if tmux list-windows -t "$GLOBAL_TMUX_SESSION" 2>/dev/null | awk '{print $2}' | sed 's/:$//' | grep -qx "$slug"; then
      printf 'tmux: window present (%s:%s)\n' "$GLOBAL_TMUX_SESSION" "$slug"
    else
      printf 'tmux: no window for slug\n'
    fi
  else
    printf 'tmux: session not present (%s)\n' "$GLOBAL_TMUX_SESSION"
  fi
  if [ -d "$wt" ]; then printf 'worktree: present\n'; else printf 'worktree: missing\n'; fi
  # Repo-root symlink check (informational)
  local root_link; root_link="$(_dest_abs_from_repo_root "${LOCAL_TICKET_FOLDER}")"
  if [ -L "$root_link" ]; then
    local t; t="$(readlink "$root_link")" || t=""
    if [ "$t" = "$AGENTX_TICKETS_DIR" ]; then
      printf 'root-link: ok (%s -> %s)\n' "$root_link" "$t"
    else
      printf 'root-link: mismatch (%s -> %s, expected -> %s)\n' "$root_link" "$t" "$AGENTX_TICKETS_DIR"
    fi
  else
    printf 'root-link: missing (%s)\n' "$root_link"
  fi
}

# ---- Main ----
COMMAND="${1:-help}"
shift || true

case "${COMMAND}" in
  install) install "$@" ;;
  provision|new) provision "$@" ;;
  start) start "$@" ;;
  stop|abort) stop "$@" ;;
  info) info "$@" ;;
  await) await "$@" ;;
  list) list "$@" ;;
  read|tail) read_bundle "$@" ;;
  run) run "$@" ;;
  merge) merge "$@" ;;
  service) service "$@" ;;
  doctor) doctor "$@" ;;
  help|--help|-h|"") usage ;;
  *) echo "Unknown command: ${COMMAND}"; usage; exit 1 ;;
esac
exit 0
