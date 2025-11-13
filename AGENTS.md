# AGENTS.md

This is the always‑relevant guide for coding agents. Keep it lean, clear, unambiguous, specific, correct, up-to-date and actionable. If anything is unclear, stop and escalate to the issue owner.

## Active Temporary Notices
- None

## Turn Start Checklist
At start of turn, run this command to capture the context that was handed to you:

```bash
pwd && date -Is && git status -sb
```

Before you modify an initially dirty file for the first time, make a gitignored timestamped backup (for example `cp path/file path/file.bak.$(date +%s)`) so you can later access the exact content the owner handed you.

## Source of Truth and Layers
- Issues (file-based via `agentx`) are the source of truth.
- Thesis specs in `docs/src/thesis/` define algorithms and data at a higher level.
- Code/tests implement the specs; data artifacts are outputs.
- Flow: issues → thesis → code/tests → data. If problems are encountered, escalate to the thesis spec and issues layers first.
- Stay local: never involve `origin/...` for day-to-day work. All coordination happens inside this clone plus its agentx worktrees.
- Cross‑refs in code or markdown `<!-- comments -->`:
  - `Docs: docs/src/thesis/<path>#<anchor>`
  - `Issue file: issues/<name>.md`
  - `Code: <path>::<symbol>`
- All code files start with a comment block that explains the purpose of the file, the why behind its architecture, and references useful further readings. These comment blocks help freshly onboarded agents get up to speed quickly. They are also colocated with the code to minimize search time, and to make maintenance both faster and more likely.

## Components and Repo Map
- High Performance Geometry and Algorithms in Rust with PyO3/maturin bindings:
  - `Cargo.toml` (workspace)
  - `crates/viterbo`: Math and core algorithms (Rust lib). Use `nalgebra` for fixed-size geometry. Uses unit tests, property tests (`proptest`), and benchmarks (`criterion`).
  - `crates/viterbo-py`: PyO3 glue exposing `_native` to Python.
- Orchestration and Data Science and Machine Learning in Python:
  - `pyproject.toml`: Python package + maturin build.
  - `src/viterbo/`: Python namespaces.
    - `<experiment>/`: We favor fast development using isolated experiments, with acyclic dependencies. Code repetition for parallel development is beneficial, only stabilize common code once the experiments that use it have stabilized.
    - `<experiment>/stage_<name>.py`: Pipeline stages (one file per entry point).
    - `provenance.py`: Write simple JSON sidecars next to artifacts.
    - `rust/`: Thin Python wrappers around `_native` (NumPy in/out).
  - `tests/`: Unit and E2E tests for the Python codebase.
    - `smoke/`: Fast smoke tests.
    - `scratch/`: Ephemeral smoke tests during development. Safe to delete.
    - `e2e/`: On‑demand E2E tests that assert on produced artifacts.
  - `configs/<experiment>/`: Pipeline configs in JSON. Typically a tiny test config and a full production config.
- Documentation of the thesis, and the development:
  - `docs/src/thesis/`: MSc mathematics thesis with high-level specs for algorithms, datasets, experiments and interpretation of results.
  - `docs/src/meta/`: Meta documentation about project-specific conventions, workflows, and other development knowledge. Basically anything that would be out of scope in `AGENTS.md`, but is situationally useful to look up. Include "when to read" hints in `.../meta/README.md`.
  - `docs/book.toml`: mdBook config.
- Data artifacts:
  - `data/<experiment>/<artifact>.<ext>` with sidecar `data/<experiment>/<artifact>.<ext>.run.json`. Both live in Git LFS; commit the artifact and its `.run.json` together to keep provenance aligned.
  - `data/downloads/`: Paper downloads (text sources + PDFs). Also under Git LFS so offline copies travel with the repo.
  - `docs/assets/`: Small publication artifacts (including interactive figures) that stay in the regular git history for easy diffs/review.
  - `data/` rides through Git LFS. Run `git lfs pull --include "data/**" --exclude ""` after switching branches (or after a fresh worktree) to hydrate the pointers locally. `scripts/reproduce.sh` is the single source of truth for regenerating *every* artifact that shows up in the docs/thesis (bench tables, figures, data files, etc.). Whenever you add or change an artifact, update `scripts/reproduce.sh` in the same issue so nobody ever has to guess whether it belongs there.
- Explicit, documented devops:
  - Reference: `docs/src/meta/tools.md` carries the full contract, flags, and maintainer notes for every script. Use the bullets below as the always-relevant shortlist, then hop to the tools doc when you need deeper context.
  - `AGENTS.md`: This file. Onboarding for all new agents.
  - `scripts/`: Devops scripts.
    - `group-timeout.sh`: Must-use timeout wrapper for potentially long-running commands (installs as `~/.local/bin/group-timeout`).
    - `background.sh`: Detach helper; logs to `/tmp/background-<ts>-<pid>/`.
    - `agentx.sh`: Codex session/worktree orchestrator (state in `~/.config/agentx/state.json`). Always launch/resume turns via `bash scripts/agentx.sh run --worktree <path> [...]` so hooks fire and bookkeeping stays accurate; any direct `codex` CLI you spawn will appear in `agentx list` as `status=unmanaged` until you clean it up. The default columns are `session_id,status,worktree,branch,pid,updated_at`; for quick triage run `bash scripts/agentx.sh list --fields session_id,status,pid,cmd --filter status=active,unmanaged`. The `list` command fuses tracked sessions, Codex `.jsonl` logs, and live `codex --yolo` processes, so unexpected rows mean an unmanaged session or stale log file that needs action. Full status semantics, hook details, and maintainer notes live in `docs/src/meta/tools.md#scripts-agentx.sh`.
    - `python-lint-type-test.sh`: Fast Ruff/Pyright/pytest (non-e2e) loop for Python code.
    - `rust-fmt.sh`: `cargo fmt --all --check`.
    - `rust-test.sh`: `cargo nextest run` (fallback to `cargo test`) wrapped via `group-timeout`.
    - `rust-clippy.sh`: `cargo clippy -p viterbo --all-targets -- -D warnings`.
    - `ci.sh`: Manual full CI.
    - `reproduce.sh`: Reproduction entrypoint (as defined in README). Builds the code, runs tests (including E2E), regenerates data artifacts, and builds the mdBook. Also serves as a readable reference of the project’s dataflow.
    - `rust-bench.sh`: Criterion benches (regular preset; exports curated JSON into `data/bench/criterion`). Set `BENCH_RUN_POSTPROCESS=1` to chain the docs stage automatically.
    - `rust-bench-quick.sh`: Criterion quick preset for local iteration (reduced warm-up/measurement; does not export).
    - `provision-worktree.sh`: Safe helper to clone new issue worktrees (validates cleanliness, hydrates LFS, runs provision hooks).
    - `merge-worktree.sh`: Rebase + fast-forward helper that syncs a issue branch into its target worktree/branch.
    - `subagent.sh`: Fire-and-forget Codex helper for scoped tasks (one synchronous turn; prints the final message inline). Use it when you need a quick delegated search/fix without juggling a second background session; syntax/flag reference is in `docs/src/meta/tools.md#scripts-subagentsh`. Decision rule: main issue turns must use `agentx run` (so session metadata stays consistent and you can update the issue body accordingly); single-turn helpers run via `subagent.sh` (they still show up in `agentx list` as their own session but finish immediately); bare `codex …` invocations are forbidden because they appear as `status=unmanaged` until you kill or wrap them.
    - Both Rust wrappers default `CARGO_TARGET_DIR` to the repo-local shared cache `.persist/cargo-target`. Keep it there so every worktree reuses the same compiled deps; only override when debugging deeply isolated builds.
    - Legacy cleanup: if a worktree sprouted `data/target*` directories, delete them and reset your env—those came from pointing `CARGO_TARGET_DIR` inside the repo.
    - `paper-download.sh`: Fetch paper sources and PDFs into `data/downloads/`.


## Platform and Tooling
- Platform:
  - Orchestration in Python; Rust for hotspots (called from Python).
  - PyO3 + maturin; native module name is `viterbo_native` (re-exported as `viterbo._native`).
  - Interop via NumPy (`pyo3‑numpy`) for now; convert to/from Torch tensors in Python.
  - Geometry: `nalgebra`. Data wrangling: `polars`. RNG: `rand` in Rust, `random`, `numpy.random`, and `torch.manual_seed(...)` in Python.
  - No Jupyter notebooks.
- agentx provisions the environment and worktree; agents do not perform manual setup unless a issue explicitly asks for it.
- Development environment: everything runs inside a single VS Code devcontainer on the project owner’s Ubuntu desktop. There is one clone of the repo, no GitHub-hosted CI, and all automation (agentx, scripts/python-lint-type-test.sh, etc.) executes inside that container. Assume local resources; escalate before assuming external services exist.
- Tooling:
  - Python 3.11+ runtime; examples use `group-timeout 60 uv run ...` for command execution.
  - Rust stable toolchain (see `rust-toolchain.toml`), with `rustfmt`, `clippy`.
  - Lean4 workspace under `lean/` for formal specs; run the helper scripts (`lean-setup`, `lean-lint`, `lean-test`) under `group-timeout` and read `docs/src/meta/lean-onboarding.md` before editing proofs.
  - Setup scripts (`scripts/bash-setup.sh`, `scripts/python-setup.sh`, `scripts/rust-setup.sh`, `scripts/lean-setup.sh`) run automatically during container provisioning and new worktree creation; rerun them manually if you need to rehydrate toolchains or caches.
  - Git LFS (latest 3.x). Run `git lfs install --local` once per worktree and `git lfs pull --include "data/**" --exclude ""` after switching branches so large artifacts are available locally.
  - Fast feedback: `bash scripts/python-lint-type-test.sh` (Python format/lint/type/test), then `bash scripts/rust-fmt.sh`, `bash scripts/rust-test.sh`, and `bash scripts/rust-clippy.sh` before running selective smoke/e2e tests.
  - Rust build cache strategy: sccache is enabled (`RUSTC_WRAPPER=sccache`) and all Rust builds default to a repo-local shared target dir `CARGO_TARGET_DIR=.persist/cargo-target` to maximize cross‑worktree cache hits for third‑party crates. Occasional “blocking waiting for file lock” is expected and safe; locks are kernel‑released on process exit/crash, and `group-timeout` ensures cleanup when a command exceeds its budget.
  - Native extension: build/refresh via `group-timeout 300 uv run maturin develop -m crates/viterbo-py/Cargo.toml`. CI also builds natively to catch drift early. We do not publish to PyPI; packaging-for-distribution assumptions do not apply in this repo.
  - PyO3 best practices: prefer modern signatures in `#[pymodule]` (`fn m(_py: Python, m: &Bound<'_, PyModule>)`) and avoid deprecated GIL ref shims. Do not add tests that assert the native `.so` stamp matches HEAD; rely on runtime symbol errors to signal rebuild needs. The abi3 module (`src/viterbo/viterbo_native*.so`) and a `.run.json` stamp are versioned to keep the repo self-contained for agents.

## Timeout Wrapper (`group-timeout`) & Background Jobs
- Purpose: `scripts/group-timeout.sh` applies explicit deadlines to every long-running command and kills the entire process group (children + grandchildren) when it overruns. This keeps Codex turns responsive and avoids orphaned jobs.
- How to use:
  - Pattern: `group-timeout <seconds> <command>`. The helper lives in `~/.local/bin/group-timeout` and exports `GROUP_TIMEOUT_ACTIVE=1` plus `GROUP_TIMEOUT_SECONDS=<seconds>` for downstream scripts.
  - Pick timeouts that comfortably exceed the happy-path runtime but still surface bugs quickly:
    - 10–20s: format/lint/typecheck, tiny smoke tests.
    - 60–120s: `cargo test` for a single crate, focused benches compile-only.
    - 300–600s: selected E2E, full benches, mdBook build.
- Scope and policy:
  - All scripts under `scripts/` expect `GROUP_TIMEOUT_ACTIVE=1`. They emit a warning (and refuse to run) when invoked without the wrapper.
  - Do not stack multiple `group-timeout` layers; pick one budget per top-level command.
- Background jobs: `scripts/background.sh <cmd…>` detaches helpers into `/tmp/background-<ts>-<pid>/` with `stdout.log`, `stderr.log`, and `exitcode.log`. `scripts/agentx.sh` relies on it to keep Codex sessions responsive. Use it yourself when you need to spawn long-lived helpers without blocking the main turn.
- Exception: `scripts/reproduce.sh` documents per-stage budgets internally (each stage wraps itself with `group-timeout`). Running it as-is is acceptable; wrapping the whole script is optional.
- On timeout: `group-timeout` returns 124, prints a warning, and SIGKILLs the remaining processes. Adjust the plan or escalate instead of blindly rerunning with a larger budget.

### Agent Autonomy (verification defaults)
- Do not ask the project owner before running fast verification. Prefer these focused loops:
  - Python quick loop: `group-timeout 10 bash scripts/python-lint-type-test.sh`
  - Rust quick loops:
    - Format: `group-timeout 10 bash scripts/rust-fmt.sh`
    - Tests: `group-timeout 120 bash scripts/rust-test.sh`
    - Clippy: `group-timeout 120 bash scripts/rust-clippy.sh`
  - mdBook quick build: `group-timeout 120 mdbook build docs`
  - Selected tests: `group-timeout 10 uv run pytest -q tests/smoke/test_xyz.py::test_abc`
  - Optional native build (for code paths that depend on it): `group-timeout 300 uv run maturin develop -m crates/viterbo-py/Cargo.toml`
- Only escalate before running when the action is potentially destructive, requires unusually long budgets beyond those above, or needs external services beyond our local toolchain.
- Always summarize what you ran and any failures; include exact commands and key logs. The owner’s time is valuable—minimize round trips.

## Issue Workflow
Issues live directly in the repo under `issues/<name>.md` (the `issues` symlink points at `.persist/issues`, so every worktree shares the same files). Pick descriptive filenames like `atlas-vis.md`; no UUID suffixes or nested folders needed.

### Header
Copy `docs/src/meta/issue-template.md`, then edit the YAML header before the Markdown body.
```
---
status: open        # open | closed
owners: []          # supervising humans (GitHub handle, name, etc.)
assignees: []       # active session_ids or shorthand for working agents
tags: []            # free-form labels, e.g., ["blocked", "milestone:atlas"]
created_at: 2025-11-13T00:00:00Z
updated_at: 2025-11-13T00:00:00Z
---
```

Update `created_at` once and bump `updated_at` whenever you materially change the issue.
- `owners`: humans steering/accepting the work.
- `assignees`: active Codex sessions (the IDs you see in `agentx list`). Remove them when a session hands off.
- `tags`: short free-form strings (`blocked`, `milestone:atlas`, `stage:vis`).

### Body layout
Stick to GitHub-style sections: **Context**, **Goals & Deliverables** (use `- [ ]` checklists), **Acceptance / Validation**, **Notes**. Keep prose succinct and link to specs or docs inline.

### Creating / editing issues
- `cp docs/src/meta/issue-template.md issues/<name>.md` → edit the header/body in place.
- Mention relevant worktrees/branches inside the body. Worktrees ↔ issues is many-to-many; document whatever mapping you use so successors can follow it.
- Keep `owners`, `assignees`, `tags`, and `updated_at` fresh whenever responsibility shifts.

### Inspecting issues
- `bash scripts/issue-list.sh` prints one tab-separated line per issue (status, owners, assignees, tags, path/title). Pipe it into `rg` or `awk` to filter by session ID, tag, or filename.
- Use `rg`/`fd` for full-text searches inside the `issues/` directory when you need context.

### Sessions & agentx
- `agentx` now manages Codex sessions only. Launch turns via `bash scripts/agentx.sh run --worktree … --prompt-file …`; if you omit `--prompt*`, the command exits with an error (no implicit prompts).
- `agentx list` continues to show tracked sessions plus unmanaged Codex processes. Keep `assignees` in sync with whatever shows up there—if your session ID is running, it should be listed in the relevant issues.
- Unmanaged rows (`status=unmanaged`) mean someone launched `codex` outside agentx; wrap that work or kill the PID before you leave the turn.

### Closing the loop
- When an issue completes, set `status: closed`, clear `assignees`, bump `updated_at`, and describe the outcome in the body.
- For follow-up work, open a new issue (or add a checklist item) instead of rewriting history.

- Issue peer reviews (use Codex to sanity-check large/ambiguous specs before coding or handoff):
  - Typical command (run from repo root):  
    ```
    codex --yolo --cd /workspaces/rust-viterbo \
      --model gpt-5-codex -c reasoning_budget='"medium"' \
      exec 'You are reviewing issue specs only ...' \
      > /tmp/codex-review-$(date +%s).txt
    ```
    Use shell redirection so long outputs never truncate inside the harness.
  - Model choice trade-offs: `gpt-5-codex` + medium reasoning is fast and good at shell/tool use; `gpt-5` + high reasoning yields slower but deeper critiques. Mix as needed (example: codex run for quick pass, then a slower follow-up if issues persist).
  - Prompts must forbid mutating commands and request critique dimensions (clarity, completeness, actionability, specificity). Keep sandbox read-only unless you have a strong reason otherwise.
  - When to run: before starting a new large issue, before requesting review on a complicated spec, or when scope creep is suspected. Document outcomes directly in the issue body (Notes section) so the next agent sees the verdict.
  - Optional QoL flags (check `codex --help` for availability): `--color=never` to keep logs clean, `--json` or `--output-last-message` for structured captures, `--quiet/--no-internal-output` to suppress tool chatter.

- Issue body structure and style:
  - Keep each issue body deterministic and high-signal; follow this outline unless the owner specifies another template:
    1. **Generating idea / context** — one short paragraph capturing why the work exists (source insight, bug, or hypothesis).
    2. **Goals & constraints** — explicit success criteria plus constraint list with forgiveness notes (what is non-negotiable vs. stretch).
    3. **Final deliverable** — bullet list of concrete artifacts (code, docs, data) that prove the goal is met.
    4. **High-level plan** — 3–6 ordered steps that link the generating idea to the deliverable (issues → thesis → code flow). Each step should be testable.
    5. **Mid-level plan & tradeoffs** — per-step detail covering major components, key decisions, and known tradeoffs (e.g., tooling choices, performance vs. scope). Reference specs/issues via the `Docs:/Issue:/Code:` comment convention when relevant.
    6. **Variations / adaptation hooks** — pre-approved pivots, fallback options, or monitoring notes that guide future agents if assumptions change.
  - Style rules: write in the same concise, explicit tone as AGENTS.md; favor lists over prose; note open questions; avoid duplicating AGENTS.md—link to sections instead. Treat the issue body as immutable once work starts unless the owner updates it.

- Final message:
  - End each turn with a concise final message that explains what changed, how to validate (exact commands), and what’s next (if anything). agentx stores it alongside the session; summarize the key points in the issue body so future agents inherit the context.

- Conventions:
  - Use lowercase, hyphenated filenames for issues (e.g., `atlas-vis.md`).
  - Keep the YAML header minimal and hand-editable; no automation touches it.
  - Document any cross-worktree or delegation relationships explicitly in the issue body instead of relying on hidden tooling.

## API Policy (Internal Only)
- We have no stable public API. All Rust modules are project‑internal.
- Prefer better, clearer APIs over compatibility. Breaking changes are not just allowed, they’re expected when they improve quality or align us with the thesis/specs.
- Don’t carry legacy shims or deprecations unless a issue explicitly asks for a staged transition. Keeping low churn for its own sake causes rot.
- Use `viterbo::api` and `viterbo::prelude` for convenience imports in internal code. These surfaces are curated for agents and may change at any time.
- If an external‑looking boundary appears (e.g., PyO3), treat it as internal too unless a issue declares support guarantees for a specific consumer.
- The documentation, code comments, tests and scripts are focused on the current commit only. Do not mention past versions and do not attempt to maintain legacy compatibility layers or fallbacks.

## Everyday Tips and Tricks
- All output of your commands is truncated with warning beyond 250 lines or 10kB. It's a hard-coded limit in codex cli's harness.
  - When reading a file, print line numbers so you see from where to resume reading if truncation occurs.
  - When running commands that will produce long output, `tee` it to `/tmp` so you can if necessary read the full output in chunks from the file.
  - When writing documentation or code, try to split them into smaller files to stay below the limits.
  - When writing documentation that cannot be split, use sections with headers that can be `rg`ed individually.
  - When searching relevant code, you may like `rg` and `fd`.

<!-- END OF AGENTS.md -->
