# Tools

This document is the canonical reference for every tool agents can invoke inside this repository. Use it in two ways:
1. **Day-to-day operations:** look up how to call a tool, what problems it solves, and which workflows depend on it.
2. **Maintenance and evolution:** when you add or change a tool, update this file so future agents inherit the correct purpose, design rationale, and contracts.

If a tool is not described here, treat it as unsupported until you add a full entry. AGENTS.md only carries a short reminder list; all authoritative detail lives in this file.

## Popular Tools

We omit explanations since agents are deeply familiar with these tools already.

- **Content and Name Search**: `rg` (ripgrep), `fd`
- **Structural Search**: `semgrep`, `ctags`
- **File Reading**: `head`, `tail`, `sed`, `nl`, `cat`
- **File Writing**: bash literal heredoc `<<\EOF`, `base64`, `git apply`, `printf %s`, `echo`

## Codex CLI Built-In Functions

These are the only host-provided tools available to agents. If a capability isn’t listed here you must implement it yourself.

### `functions.exec_command`
- Purpose: Run shell commands synchronously with full control over `cwd`, shell type, and login semantics.
- Input: `cmd` (string, required), `workdir` (string, required absolute path), `shell` (string, default `/bin/bash`), `login` (bool, default `true`), `max_output_tokens` (int, default 6000), `yield_time_ms` (int, default 0).
- Output: `stdout`, `stderr`, `exit_code`, plus a `session_id` that can be reused by `functions.write_stdin`.
- Behavior: Blocks until the process exits **and** both pipes close (unless `yield_time_ms` is set). Streams truncate at ~250 lines or 10 KiB; wrap long commands with `yield_time_ms` so you can poll progress before the truncation limit.

### `functions.write_stdin`
- Purpose: Keep interacting with a running `exec_command`: feed passwords, send incremental commands, or drain additional output.
- Input: `session_id`, optional `chars`, optional `max_output_tokens` / `yield_time_ms`.
- Behavior: Writes the provided data into the process, collects everything emitted since the previous poll, and reports whether the child has exited.

### `functions.list_mcp_resources`
- Purpose: Discover read-only resources exposed by MCP servers (tickets, specs, docs).
- Input: Optional `server` filter and pagination `cursor`.
- Output: Resource descriptors (`uri`, title, summary) which you then pass to `functions.read_mcp_resource`.

### `functions.list_mcp_resource_templates`
- Purpose: Enumerate parameterized resource templates.
- Input: Optional `server`, `cursor`.

### `functions.read_mcp_resource`
- Purpose: Fetch the full contents of a specific MCP resource.
- Input: `server` and `uri` from the listing calls.

### `functions.update_plan`
- Purpose: Maintain the multi-step execution plan Codex requires on non-trivial tasks.
- Input: Optional `explanation`, plus a `plan` array of `{ "step": "...", "status": "pending|in_progress|completed" }` entries (only one `in_progress` allowed).
- Behavior: Completely replaces the plan on each call; keep steps short, sequenced, and testable.

### `functions.view_image`
- Purpose: Attach a local image (plot, screenshot, etc.) to the conversation.
- Input: `path` to the image file.

### `functions.apply_patch`
- Purpose: Safely edit files without fighting shell quoting.
- Input: A unified diff that conforms to the harness grammar (`*** Begin Patch` … `*** End Patch`).
- Output: Either succeeds atomically or fails before touching the filesystem.
- Behavior: Accepts ASCII text of arbitrary length, rejects malformed diffs immediately, and prevents partial writes.
- Why it is the default for complex edits:
  1. **No shell quoting:** you paste exactly the text you want; no need to escape `$`, quotes, or heredoc sentinels.
  2. **Lower truncation risk:** the harness doesn’t chop apply_patch payloads the way it truncates stdout from heredocs or inline Python.
  3. **Reviewability:** the diff mirrors what ends up in git, so reviewers see the same structure.
  4. **Built-in guard rails:** if the file changed underneath you, the patch fails instead of corrupting state.

#### Why heredocs/inline scripts are fragile
- **`cat <<'EOF' > file`**: the entire command still has to survive the quoting rules of `functions.exec_command`, so a stray `'`, `$`, or backslash can break before the heredoc even starts; output truncates at ~10 KiB and the command fails silently if the delimiter appears in the body.
- **`python - <<'PY'` snippets**: long strings need heavy escaping and syntax errors leave partial files.
- **`printf`/`echo`**: fine for single lines, dangerous for structured edits.

## Custom Tools

Each entry describes the problem it solves, the observable pain when it is missing, and the workflows that rely on it.

### `group-timeout`
- **Problem:** Codex runs blocking commands; without an enforced timeout a hung `cargo` or `pytest` will freeze the entire turn.
- **How it manifests:** the agent’s thoughts stop streaming, nothing else can run, and reproducing the hang requires manual intervention from the project owner.
- **Solution:** `group-timeout <seconds> <command>` wraps every long-running command in its own process group, exports `GROUP_TIMEOUT_ACTIVE=1`, and kills the tree with SIGTERM/SIGKILL when the deadline hits. All repo scripts check `GROUP_TIMEOUT_ACTIVE` before doing work.

### `background`
- **Problem:** Agents need to hand control to Codex (`agentx run`, helper searches, log tailing) while continuing their own turn. `functions.exec_command` can detach with `yield_time_ms` but there are various troubles if stdout/stderr is inherited by grandchildren processes and doesn't close, and the function also cannot let processes linger after the turn ends.
- **How it manifests:** a usually background task is run synchronously, either due to a mistake of the agent or the rather finicky pitfalls in `functions.exec_command` wrt stdout/stderr of grandchildren, and the agent's turn stalls indefinitely until the project owner manually intervenes. timeouts prevent indefinite hangs but still do not allow the agent to start a background command and consult the logs and then take further actions while the background command continues to run.
- **Solution:** `background <command> [args...]` forks immediately, prints the PID and log file paths under `/tmp/${timestamp}-${pid}/{stdout,stderr,exitcode}.log`, and lets agents or orchestrators (like `agentx.sh`) monitor or terminate the job later with `kill`.

### `scripts/python-lint-type-test.sh` / `scripts/rust-*.sh`
- **Problem:** Without a canonical fast loop, each agent invents their own lint/test invocation, leading to missed steps, distractions and simply unnecessary overhead.
- **Behavior:** Every script assumes it runs under `group-timeout`, and executes the standard command sequence (Ruff format/lint, Pyright, pytest smoke; `cargo fmt`, `cargo nextest`, `cargo clippy`, benches, etc.). Invoke via `group-timeout 30 bash scripts/<name>.sh`.
- **Note:** Manual standard commands such as `group-timeout 10 uv run pytest q k tests/test_example.py` allow to deviate from the canonical loop when advantageous, e.g. for even leaner edit-and-verify cycles.

### `scripts/ci.sh`
- **Problem:** There needs to be a quality gate before merging changes into main. It has to be documented, reproducible, and thorough. Timing targets must be documented to notice and fix slowdowns in test suites, builds, and docs generation.
- **Behavior:** Runs the full suite of linting, type checking, testing, benchmarks, and docs build. Specifies independent upper-bound timeouts for each stage using `group-timeout`. Fails on first error.
- **Usage:** `bash scripts/ci.sh`

### `scripts/reproduce.sh`
- **Problem:** Reviewers need to regenerate the entire thesis pipeline (build deps, run tests, reproduce artifacts, render docs) without tribal knowledge.
- **Behavior:** Runs every stage with recorded arguments (venv creation, `uv sync`, Python lint/test loop, Rust fmt/test/clippy, benches, docs build, data pipelines). Serves equally as documentation and as the end-to-end smoke test before publishing. Specifies upper-bound timeouts for each stage via `group-timeout`.
- **Usage:** `bash scripts/reproduce.sh`

### `.devcontainer/{Dockerfile,devcontainer.json,postCreate.sh}`
- **Problem:** Divergent local environments make reproducing results or debugging others’ work painful, and don't allow agents to reason concretely, reliably and quickly about the environment that their code has to run in.
- **Behavior:** Defines the single supported environment; `postCreate.sh` wires persistent caches (`.persist`), installs system dependencies, and drops helper binaries into `~/.local/bin/`. All agents run inside this devcontainer to guarantee parity.

### `scripts/provision-worktree.sh`
- **Problem:** Provisioning a new worktree by hand leads to missing hooks, stale dependencies, or half-configured environments.
- **Behavior:** Validates the source tree is clean, creates the new worktree/branch, hydrates LFS and caches, runs `hook-provision.sh`. This ensures every agent starts in a ready-to-go state.
- **Usage:** `scripts/provision-worktree.sh --source <branch|folder|commit> --target <folder> [--branch <name>] [--skip-hook]`

### `scripts/merge-worktree.sh`
- **Problem:** Hand-merging worktrees invites forgotten rebases, stray commits, or unclean directories.
- **Behavior:** Checks both directories, rebases the worktree branch onto the target, performs a validation, then fast-forwards. Options like `--ignore-uncommitted`, `--skip-rebase`, and `--dry-run` let you adjust the workflow consciously instead of forgetting steps.
- **Usage:** `scripts/merge-worktree.sh --source <folder> --target <branch|folder> [--ignore-uncommitted] [--skip-rebase] [--dry-run]`

### `scripts/agentx.sh`
- **Problem:** Agents need a first-class CLI to manage Codex sessions (list/view/run/archive/abort) without diving into Python internals.
- **Behavior:** Maintains session metadata (`session`, `worktree`, `pid`, `turn`, config overrides, archive state) under `~/.config/agentx/state.json`, updates the table on each command, and launches Codex turns via `group-timeout` + `background`.
- **Model:** Codex CLI manages the sessions, i.e. the context history an agent has access to. Sessions consist of turns, each turn starts with an externally provided prompt, is followed by reasoning trace, tool invocations, tool outputs, and ends with an agent-written final message. We don't allow reusing a session in a different worktree to avoid confusion. Active sessions have an ongoing incomplete turn, a PID, can be aborted, and their worktree exists on disk. Inactive sessions have no ongoing turn, no PID, can be started, and their worktree exists on disk. Archived sessions are inactive and may have their worktree deleted to save space.
- **Usage:** `scripts/agentx.sh <command> [args...]` where the following commands are supported:
  - `list [--fields <field1,field2,...>] [--sort-by <field1,...>] [--filter <field>=<value1,...>]`: list all sessions with optional field selection, sorting, and filtering.
    Available fields: `session_id` (UUID), `worktree` (string), `branch` (string), `git_state` (unicode symbols), `status` (active,inactive,archived), `turns_completed` (integer), `last_updated` (timestamp), `created_at` (timestamp), `pid` (integer or null).
    Default is to show all fields, sort by `last_updated` descending, and filter to active and inactive sessions only.
  - `view --session <session_id>`: short-cut to show only one session.
  - `view --worktree <path>`: short-cut to show only the sessions for a given worktree.
  - `run --worktree <path> [--session <session_id>] [--prompt "<prompt>"|--prompt-file <file>] [args...]`: start a new or continue an existing session in the given worktree with optional extra arguments passed to the codex cli command (e.g. `... -c reasoning_budget=low`).
  - `abort --session <session_id>`: send SIGTERM to the active session's PID to force-stop the ongoing turn mid-action. No final message will be produced for the aborted agent turn.
  - `archive --session <session_id> [--delete-worktree]`: mark the session as archived to prevent further runs; optionally delete the worktree to save space. not intended to be reversed.
  - `await --session <session_id>`: block until the given session's active turn completes or is aborted; prints the final message or an abort notice. Requires being wrapped in `group-timeout`.
- **Note:** Agents should always use `scripts/agentx.sh run ...` to start or continue sessions instead of invoking Codex CLI directly. This ensures proper session tracking, timeouts, and backgrounding.
- **Note:** For a simpler synchronous interface, use `scripts/subagent.sh` instead.

### `scripts/subagent.sh`
- **Problem:** Delegating a scoped task (search, grep-intensive triage, fix linter errors) to a helper agent should not pollute the main session or risk runaway jobs.
- **Behavior:** Requires `group-timeout`, spawns a fresh Codex session, runs exactly one turn with the provided prompt/config overrides, waits for completion, prints the final message/log summary.
- **Usage:** `group-timeout <seconds> bash scripts/subagent.sh --worktree <path> [--prompt "<prompt>"|--prompt-file <file>] [args...]` where extra args are passed to Codex CLI (e.g. `... -c reasoning_budget=low`).

### `scripts/paper-download.sh`
- **Problem:** Searching, pulling, parsing bibliography references manually is error-prone and redundant work.
- **Behavior:** Given `--id`, `--match`, `--arxiv`, or `--url`, downloads sources/PDFs into `data/downloads/<identifier>/` alongside provenance sidecars. `--all` iterates over `docs/src/thesis/bibliography.md` so the repo always contains the exact versions cited in the thesis.
- **Usage:** `scripts/paper-download.sh [--id <doi|arxiv-id>] [--match "<title|author|year>"] [--arxiv <arxiv-id>] [--url <url>] [--all]`

### `scripts/publish.sh`
- **Problem:** We publish docs via GitHub Pages without CI; the build/push workflow must be reproducible.
- **Behavior:** Builds the mdBook (`mdbook build docs`), stages a temporary `gh-pages` worktree, copies `docs/book/`, commits with a timestamped message, and pushes to the `gh-pages` branch. Wrap it in `group-timeout` to keep the workflow predictable.
- **Usage:** `group-timeout 60 bash scripts/publish.sh`

## Legacy Reference

### `scripts/agentx.py`
- Purpose: Historical Python implementation of the agent orchestrator. It embodied a similar session/worktree/PID/turn metadata model and invoked Codex directly. Keep it around only for archeology; all active development uses `scripts/agentx.sh`.

### `scripts/safe.sh`
- Purpose: Early attempt at a timeout wrapper. Superseded by `group-timeout`.