# AGENTS.md

This is the always‑relevant guide for coding agents. Keep it lean, clear, unambiguous, specific, correct, up-to-date and actionable. If anything is unclear, stop and escalate to the ticket owner.

## Active Temporary Notices
- None

## Turn Start Checklist
Run these commands before making any edits so you know exactly where you are, what time the turn started, and which files are already dirty:

1. `pwd` — confirm you are inside the expected worktree.
2. `date -Is` — capture the exact start timestamp in ISO format for logs.
3. `git status -sb` — list staged/unstaged changes so you can back up or avoid overwriting uncommitted work.

If `git status -sb` shows modified files you didn’t create, make a gitignored timestamped backup (for example `cp path/file path/file.bak.$(date +%s)`) before touching them so you can access or recover the exact content the owner handed you.

## Source of Truth and Layers
- Tickets (file-based via `agentx`) are the source of truth.
- Thesis specs in `docs/src/thesis/` define algorithms and data at a higher level.
- Code/tests implement the specs; data artifacts are outputs.
- Flow: tickets → thesis → code/tests → data. If problems are encountered, escalate to the thesis spec and tickets layers first.
- Stay local: never involve `origin/...` for day-to-day work. All coordination happens inside this clone plus its agentx worktrees.
- Cross‑refs in code or markdown `<!-- comments -->`:
  - `Docs: docs/src/thesis/<path>#<anchor>`
  - `Ticket: shared/tickets/<slug>.md`
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
  - `data/` rides through Git LFS. Run `git lfs pull --include "data/**" --exclude ""` after switching branches (or after a fresh worktree) to hydrate the pointers locally. `scripts/reproduce.sh` is the single source of truth for regenerating *every* artifact that shows up in the docs/thesis (bench tables, figures, data files, etc.). Whenever you add or change an artifact, update `scripts/reproduce.sh` in the same ticket so nobody ever has to guess whether it belongs there.
- Explicit, documented devops:
  - `AGENTS.md`: This file. Onboarding for all new agents.
  - `scripts/`: Devops scripts.
    - `safe.sh`: Must-use wrapper for potentially long-running commands (timeout + group kill). Symlinked as `~/.local/bin/safe` for convenience.
    - `python-lint-type-test.sh`: Fast Ruff/Pyright/pytest (non-e2e) loop for Python code.
    - `rust-fmt.sh`: `cargo fmt --all --check`.
    - `rust-test.sh`: `cargo nextest run` (fallback to `cargo test`) under `safe`.
    - `rust-clippy.sh`: `cargo clippy -p viterbo --all-targets -- -D warnings`.
    - `ci.sh`: Manual full CI.
    - `reproduce.sh`: Reproduction entrypoint (as defined in README). Builds the code, runs tests (including E2E), regenerates data artifacts, and builds the mdBook. Also serves as a readable reference of the project’s dataflow.
    - `rust-bench.sh`: Criterion benches (regular preset; exports curated JSON into `data/bench/criterion`). Set `BENCH_RUN_POSTPROCESS=1` to chain the docs stage automatically.
    - `rust-bench-quick.sh`: Criterion quick preset for local iteration (reduced warm-up/measurement; does not export).
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
- agentx provisions the environment and worktree; agents do not perform manual setup unless a ticket explicitly asks for it.
- Development environment: everything runs inside a single VS Code devcontainer on the project owner’s Ubuntu desktop. There is one clone of the repo, no GitHub-hosted CI, and all automation (agentx, scripts/python-lint-type-test.sh, etc.) executes inside that container. Assume local resources; escalate before assuming external services exist.
- Tooling:
  - Python 3.11+ runtime; examples use `safe --timeout 60 -- uv run ...` for command execution.
  - Rust stable toolchain (see `rust-toolchain.toml`), with `rustfmt`, `clippy`.
  - Git LFS (latest 3.x). Run `git lfs install --local` once per worktree and `git lfs pull --include "data/**" --exclude ""` after switching branches so large artifacts are available locally.
  - Fast feedback: `bash scripts/python-lint-type-test.sh` (Python format/lint/type/test), then `bash scripts/rust-fmt.sh`, `bash scripts/rust-test.sh`, and `bash scripts/rust-clippy.sh` before running selective smoke/e2e tests.
  - Rust build cache strategy: sccache is enabled (`RUSTC_WRAPPER=sccache`) and all Rust builds default to a repo-local shared target dir `CARGO_TARGET_DIR=.persist/cargo-target` to maximize cross‑worktree cache hits for third‑party crates. Occasional “blocking waiting for file lock” is expected and safe; locks are kernel‑released on process exit/crash, and `scripts/safe.sh` timeouts ensure cleanup.
  - Native extension: build/refresh via `safe -t 300 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml`. CI also builds natively to catch drift early. We do not publish to PyPI; packaging-for-distribution assumptions do not apply in this repo.
  - PyO3 best practices: prefer modern signatures in `#[pymodule]` (`fn m(_py: Python, m: &Bound<'_, PyModule>)`) and avoid deprecated GIL ref shims. Do not add tests that assert the native `.so` stamp matches HEAD; rely on runtime symbol errors to signal rebuild needs. The abi3 module (`src/viterbo/viterbo_native*.so`) and a `.run.json` stamp are versioned to keep the repo self-contained for agents.

## Safe Wrapper (timeouts & cleanup)
- Purpose: apply explicit timeouts at the top level and clean up entire process groups if a command hangs or runs longer than intended.
- How to use:
  - Pattern: `bash scripts/safe.sh --timeout <seconds> -- <your command>` (short: `safe -t <seconds> -- <cmd>`).
  - Choose timeouts to be safely above expected runtime yet low enough to catch bugs and accidental heavy runs:
    - 10–20s: format/lint/typecheck, tiny smoke tests.
    - 60–120s: `cargo test` for a single crate, small benches compile-only.
    - 300–600s: selected E2E, full benches, mdBook build.
- Scope and policy:
  - Apply timeouts at the top level. Most scripts in `scripts/` assume they run under `safe.sh` and will exit if not (they check `SAFE_WRAPPED=1`, exported by `safe.sh`).
  - Do not nest `safe.sh` inside those scripts.
- Exception: `scripts/reproduce.sh` is human-facing and documents sensible per-stage timeouts; it self-wraps each stage with `safe.sh`. You may run it directly or wrap it at the top level—both are acceptable. Today it includes the Criterion benchmarks → docs-assets stage, so the mdBook always renders from freshly generated tables; treat future artifacts the same way.
- Environment markers set by `safe.sh`:
  - `SAFE_WRAPPED=1` for all children (used by scripts to validate top-level wrapping).
  - `SAFE_TIMEOUT=<seconds>` if a timeout was provided.
- On timeout: `safe.sh` returns a non-zero code and kills the process group. Do not auto-retry inside scripts; adjust the plan or escalate if the budget is unclear.

### Agent Autonomy (verification defaults)
- Do not ask the project owner before running fast verification. Prefer these focused loops:
  - Python quick loop: `safe --timeout 10 -- bash scripts/python-lint-type-test.sh`
  - Rust quick loops:
    - Format: `safe --timeout 10 -- bash scripts/rust-fmt.sh`
    - Tests: `safe --timeout 120 -- bash scripts/rust-test.sh`
    - Clippy: `safe --timeout 120 -- bash scripts/rust-clippy.sh`
  - mdBook quick build: `safe --timeout 120 -- mdbook build docs`
  - Selected tests: `safe --timeout 10 -- uv run pytest -q tests/smoke/test_xyz.py::test_abc`
  - Optional native build (for code paths that depend on it): `safe -t 300 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml`
- Only escalate before running when the action is potentially destructive, requires unusually long budgets beyond those above, or needs external services beyond our local toolchain.
- Always summarize what you ran and any failures; include exact commands and key logs. The owner’s time is valuable—minimize round trips.

## Ticketing Workflow (agentx)
See “Ticketing Workflow (agentx)” below for the file-based system.

## Git Conventions
- Commit often; include `Ticket: <slug>` in commit messages.
- No pre‑commit hooks; rely on `bash scripts/python-lint-type-test.sh`, `bash scripts/rust-fmt.sh`, `bash scripts/rust-test.sh`, `bash scripts/rust-clippy.sh`, and selective E2E runs for validation.

## Command Line Quick Reference
- Wrap long/unknown‑cost commands in `scripts/safe.sh` with an explicit timeout; see “Safe Wrapper” section for policy and budgets.
  - Example: `bash scripts/safe.sh --timeout 10 -- uv run python -m viterbo.atlas.stage_build --config configs/atlas/test.json`
- Manual CI before handing in work to the project owner for merge:
  - `safe --timeout 300 -- bash scripts/ci.sh`
- Rust build cache hygiene:
  - Default target dir is repo-local shared cache: `.persist/cargo-target` (set in devcontainer and wrappers). This enables sccache hits across worktrees.
  - Brief lock waits during overlapping builds are normal (“blocking waiting for file lock”). Locks are freed on process exit/crash or by `safe.sh` timeouts.
  - Cleanup when needed: `safe -t 60 -- cargo clean` (or remove `.persist/cargo-target` during downtime only).
- Get feedback fast after working on code:
  - `safe --timeout 10 -- bash scripts/python-lint-type-test.sh` (format/lint/type are intentionally non-fatal via `|| true` so you see all issues in one run; tests remain strict)
  - `safe --timeout 10 -- bash scripts/rust-fmt.sh`
  - `safe --timeout 120 -- bash scripts/rust-test.sh`
  - `safe --timeout 120 -- bash scripts/rust-clippy.sh`
  - `safe --timeout 10 -- uv run pytest -q tests/smoke/test_xyz.py::test_abc`
  - `safe --timeout 60 -- cargo test -q -p viterbo`
  - `safe --timeout 120 -- bash scripts/rust-test.sh -p viterbo -- -q`
  - `safe --timeout 300 -- uv run pytest -q -m e2e tests/e2e/test_atlas_build.py::test_build_dataset_tiny`
  - Atlas data (small): `safe --timeout 300 -- uv run python -m viterbo.atlas.stage_build --config configs/atlas/small.json`
  - Rust benches (quick): `safe --timeout 180 -- bash scripts/rust-bench-quick.sh`
  - Rust benches (regular → data/bench/criterion): `safe --timeout 300 -- bash scripts/rust-bench.sh`
  - Bench docs stage (CSV/Markdown refresh): `safe --timeout 120 -- uv run python -m viterbo.bench.stage_docs --config configs/bench/docs_local.json`
  - Native extension: `safe --timeout 300 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml`
- Avoid auto‑running all E2E tests. Select by hand; it’s way faster and clearer.
  - Native build: `safe -t 300 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml`.

## Rust Conventions
- Use Rust for hotspots only; profile first.
- Use `nalgebra` for fixed-size geometry (e.g., `Vector4<f64>`).
- Use property tests (`proptest`) where appropriate to skip hand-written values.
- Use `criterion` for benchmarks; write results to `data/bench/` (tracked via Git LFS) and keep summaries in docs when reviewers need diffable numbers.
- Expose functions to Python via PyO3 in `crates/viterbo-py`.
- Functional style preferred.
- Comment to reference tickets and thesis specs.
- Comment to explain the why, not the what.
- Avoid over-abstraction; prefer simple, explicit, locally understandable code.

## Data and Pipeline Conventions
- Data artifacts go to `data/<experiment>/...` (tracked via Git LFS); keep small publishable assets in `docs/assets/` when you need them diffable on GitHub without LFS.
- Every artifact `X.ext` has a provenance sidecar `X.ext.run.json`.
- Sidecar schema: `config`, `git_commit`, `timestamp`. Create via `viterbo.provenance.write(path, config)`.
- Stages run as Python modules.
  - Pattern: `safe --timeout <seconds> -- uv run python -m viterbo.<experiment>.stage_<name> --config configs/<experiment>/<config>.json`
- The json config specifies all constants, paths, and parameters.
- Keep stages composable; reuse helpers; do not over‑abstract (YAGNI, KISS).
- Provide tiny test config variants for fast dev cycles (≤10s); Use E2E tests to assert on the outputs of the test configs.
- Rust kernels do not write provenance; Python orchestrator owns it.
- Cargo build caches: keep builds in the shared repo-local cache `.persist/cargo-target` (exported by the devcontainer and every wrapper) so worktrees reuse compiled deps. Never point `CARGO_TARGET_DIR` under `data/`. If you need isolation, use another worktree or a temporary alternate cache under `.persist`.

## Seeding and Determinism (situational)
- Put a top‑level `"seed"` in JSON configs.
- Python: set `random`, `numpy`, and `torch` seeds; include CUDA seeding when applicable.
- Rust kernels accept seed parameters as relevant; property tests use fixed seeds.

## Python Conventions
- Use basic type hints where it disambiguates; Pyright basic only needed.
- Favor immutable/functional style; move imperative orchestration closer to the command line entry points.
- Comment to reference tickets and thesis specs.
- Comment to explain the why, not the what.
- Repeat code rather than prematurely abstracting; stabilize common code only once experiments stabilize.
- Stabilize shared code after 2+ stabilized experiments depend on it; avoid catch‑all `utils`.
- Use `tests/scratch/` for on the fly testing that can be deleted once done. No need to maintain large sets of unit tests.
- Or move tests to `tests/smoke/` if important to keep around long-term, e.g. to detect future regressions.
- Use `tests/e2e/` with `@pytest.mark.e2e` to make assertions on the data artifacts produced by pipeline stages, especially test configs that run fast. Also add assertions into the production pipeline stages where appropriate to catch bugs that tests may miss.
 

## Testing Policy
- Rust cores (algorithms): unit tests + property tests required; benchmarks with `criterion` under `data/bench/` (committed via Git LFS).
- Python orchestration: smoke tests and selective E2E on tiny configs; add unit tests when logic is non‑trivial.
- CI defaults: `scripts/python-lint-type-test.sh`, `scripts/rust-fmt.sh`, `scripts/rust-test.sh`, `scripts/rust-clippy.sh`, `scripts/rust-bench.sh` (+ `python -m viterbo.bench.stage_docs`), and on-demand E2E by selection (`-m e2e -k ...`).
- Default: prefer smoke + E2E over broad Python unit test suites unless justified by complexity.

## Documentation Conventions
- High-level specs in `docs/src/thesis/` about the mathematics, algorithms, data formats, and experiment ideas.
- Meta documentation in `docs/src/meta/` about project-specific conventions, workflows, and reminders that fix encountered mistakes.
- Keep `AGENTS.md` lean and always relevant; move situational further readings to `docs/src/meta/` with clear "when to read" hints in `docs/src/meta/README.md`
- Reading rule: agents read AGENTS.md end-to-end in one pass before starting work. Do not rely on progressive disclosure here; surface all essential conventions and workflows directly, with concise examples.
- Authoring rules for this file (very important):
  - Do not add “Quick Start” sections or separate summaries. This file is the single canonical contract and is read end‑to‑end.
  - Avoid duplicating content already covered in this file; instead, improve the existing section or link to anchors.
  - If you need orientation for humans, put it in `README.md` or `agentx --help`, not here.
  - Section size limit: keep every top‑level section (## …) ≤ 250 lines. If a section grows larger, split it into smaller subsections or move detail to `docs/src/meta/` and link back with a clear “when to read” hint. This ensures sections are easy to scan and compatible with our CLI’s chunked reads.
  - Retrieval‑friendly structure: give each section a unique, stable H2 header and begin with a short mental‑model bullet list so tools (and humans) can jump directly to the right anchor without paging the whole file.
- Use GitHub Pages to host the mdBook site at https://joernstoehler.github.io/rust-viterbo/ via `scripts/publish.sh`.
- Write in a clear, unambiguous, specific, actionable, explicit style with low cognitive overhead, so that development agents can read text and get to work quickly without needing to think through ambiguities or infer implications that weren’t spelled out.
- Use KaTeX-safe math only (no `\\operatorname`).
- Create small tables/figures/interactive plots for inclusion in the mdBook site via `docs/assets/`.
 - Algorithm pages include a terminal section titled “Clarifications (unstable, unsorted)” to park quick notes about code/spec divergences and open questions. Entries are intentionally ephemeral; once stabilized, fold them into the main text and remove from the list.

### Thesis Writing Conventions (mdBook)
- Section layout (keep it brief and consistent):
  1) one-paragraph context; 2) Setting and Notation; 3) Definitions; 4) Main Facts/Theorems (with footnote citations); 5) “What We Use Later” bullets for algorithms; 6) optional “Deviations and clarifications for review”.
- Citations: use non-obstructive footnotes with author–year text; prefer official DOI or arXiv link; no proposition/page numbers (they vary across PDFs). Example: `[^HK19]: Haim‑Kislev (2019) ...`.
- Cross‑refs: link to sibling chapters via relative paths; avoid duplicate headers; prefer section names over numbers.
- HTML comments at the top: include `Ticket: <slug>` and any brief editing notes for future agents.
- Rendering checks: after edits run the quick loops and docs build without asking:
  - `safe --timeout 10 -- bash scripts/python-lint-type-test.sh`
  - `safe --timeout 10 -- bash scripts/rust-fmt.sh`
  - `safe --timeout 120 -- bash scripts/rust-test.sh`
  - `safe --timeout 120 -- bash scripts/rust-clippy.sh`
  - `safe --timeout 120 -- mdbook build docs`

## Escalation
- Escalate when: specs underspecified, solver/library choice blocks progress, runtime exceeds budget, or scope bloat requires a sub‑ticket.
- How: pause work, leave a concise summary + options.

## Git History Cleanup (Quick Guide)
- Schedule downtime: pause all other agent attempts/worktrees besides your own and get the owner’s explicit go-ahead before rewriting history.
- Capture the baseline (`git rev-parse main`, `git count-objects -vH`, `git lfs ls-files | wc -l`) for provenance.
- Work in a mirror clone: `git clone --mirror /workspaces/rust-viterbo /tmp/history-cleanup.git && cd /tmp/history-cleanup.git`.
- Remove bench build artifacts by running\
  `git filter-repo --force --invert-paths --path data/bench/release --path data/bench/release/ --path data/bench/tmp --path data/bench/tmp/ --path data/bench/.rustc_info.json --path data/target --path data/target/ --message-callback 'return message + b"\n[chore] Drop legacy bench artifacts (Ticket <uuid>)\n"'`
- Verify before publishing: `git rev-list --objects --all | grep data/bench/release` (should be empty), `git fsck --full`, `git lfs fetch --all`, `git lfs fsck`.
- Push the result to a staging branch (e.g., `main-clean`) in `/workspaces/rust-viterbo`, have the owner force-push to GitHub and reset + rehydrate LFS (`git lfs pull --include "data/**" --exclude ""`) followed by the two quick loops (`bash scripts/python-lint-type-test.sh`, `bash scripts/rust-test.sh`).
- Record what you did (hashes + commands) in the ticket; no extra files need to be committed for the rewrite itself.

## Design Principles
- High performance: use Rust for hotspots; profile first.
- Fast development cycles: use Python for orchestration; isolate experiments for parallel development; tiny test configs for fast feedback.
- Mathematical correctness: specify in the thesis; provide correctness arguments; write robust tests for algorithmic cores.
- Reproducibility: provenance sidecars for all data artifacts; `reproduce.sh` to rebuild everything from scratch.
- Simplicity and Maintainability: favor explicit and simple code over clever abstractions; document the why, not the what; pick popular, well-known patterns, libraries and workflows.
- Self‑documenting project: record conventions, workflows, and reminders in `docs/src/meta/`; cross-reference tickets and thesis specs.
- AI Agents as first-class developers: design everything for easy onboarding of new agents; clear, specific, and actionable tickets; maintain always‑relevant knowledge in a lean `AGENTS.md`, and move situational info to `docs/src/meta/` with clear "when to read" hints in `docs/src/meta/README.md`; avoid overhead for agents, reduce tool friction and keep related information close together to minimize search time; 
- Continuous Improvement: accept feedback from agents and the project owner; refactor with breaking changes, rewrite documentation, open additional tickets when it raises the quality of the project for future agents.

## Ticketing Workflow (agentx)
This project uses a minimal CLI (`agentx`) with a file‑based ticket stub (front matter + log). Learn this model first; it’s small and predictable.

- Model (1–1–1–1):
  - One ticket file (`<slug>.md` + `<slug>.log.jsonl`) ↔ one git branch ↔ one git worktree ↔ one Codex session.
- Ticket files live in `.persist/agentx/tickets/` and are symlinked into each worktree at `shared/tickets/`.
- `.md` front matter (mutable): `status`, `owner`, `depends_on`, `dependency_of`, `turn_counter`, timestamps. Edit only the header; keep the Markdown body immutable after provisioning.
- `.log.jsonl` (append-only): chronological events (`provision`, `tNN-start`, `tNN-final`, `tNN-abort`) recorded as JSON lines.
- Entry point: run all ticket commands via `safe -t 60 -- uv script agentx.py <command> ...` from the repo root. (The CLI file lives at `/workspaces/rust-viterbo/agentx.py`.)
- Create/edit stubs manually: copy `docs/src/meta/ticket-template.md` to `shared/tickets/<slug>.md`, fill in the YAML header/body, then run `agentx provision` only when you actually need a worktree.

- Event rules (strict):
  - Exactly one `provision` before any turn.
  - For each turn N: exactly one `tNN-start` and exactly one terminal event: `tNN-final` OR `tNN-abort`.
  - Turns strictly increase: `t01, t02, …`. Use `agentx start` to begin the next turn.

- Status semantics:
  - `status=open`: provisioned, no active turn.
  - `status=active`: a turn is in progress (after `*-start`, before its terminal message).
  - `status=stopped`: last terminal was an abort.
  - `status=done`: last terminal was a final message.

- Commands (slug‑only):
    - Creates or overwrites the ticket stub (`shared/tickets/<slug>.md` + `.log.jsonl`) without touching git.
  - `agentx provision <slug> [--inherit-from <slug>] [--base <ref>] [--copy src[:dst]]...`
    - Creates the branch/worktree for the slug, symlinks `shared/tickets/`, and logs a `provision` event. Requires an existing ticket stub.
    - Always pass an explicit source via `--inherit-from <slug>` (uses that worktree’s HEAD) or `--base <slug|branch|commit>`. Remote refs such as `origin/main` are forbidden and the command errors if no base is provided.
  - `agentx start <slug> [--message "..."]`: provisions if needed, enqueues a new turn, logs `tNN-start`, and relies on the long‑running `agentx service` loop to actually launch Codex and record `tNN-final`.
  - `agentx service [--once]`: drains the queue and runs Codex turns inside tmux. Keep it running (usually in tmux) so queued starts actually execute.
  - `agentx abort <slug>`: logs `tNN-abort` for the active turn, sets `status=stopped`, and kills the tmux window.
  - `agentx await <slug> [--timeout N]`: returns when the YAML `status` field changes from `active`.
  - `agentx info <slug> [--fields a,b,c]`: prints selected fields from the YAML header.
  - `agentx list [--status s]`: lists tickets with basic fields.
  - `agentx doctor <slug>`: tmux/worktree diagnostics for the slug.
  - There is no CLI `read`/`tail` helper anymore—inspect tickets directly (`cat shared/tickets/<slug>.md`, `tail shared/tickets/<slug>.log.jsonl`) and commit any YAML header edits you make.

- Hooks (optional per worktree):
  - `AGENTX_HOOK_PROVISION` runs right after `git worktree add` during `provision` (inside the new worktree). Recommended value: `bash scripts/agentx-hook-provision.sh`.
  - `AGENTX_HOOK_START`, `AGENTX_HOOK_BEFORE_RUN`, `AGENTX_HOOK_AFTER_RUN` run inside the worktree around each Codex turn (triggered by the queue/service flow).

- Configuration (env) — set these in devcontainer.json `containerEnv` (or `remoteEnv`):
  - `AGENTX_TICKETS_DIR=/workspaces/rust-viterbo/.persist/agentx/tickets`
  - `AGENTX_WORKTREES_DIR=/workspaces/rust-viterbo/.persist/agentx/worktrees`
  - `LOCAL_TICKET_FOLDER=./shared/tickets`
  - Optional hooks: `AGENTX_HOOK_PROVISION="bash scripts/agentx-hook-provision.sh"`, `AGENTX_HOOK_START="safe --timeout 10 -- bash scripts/python-lint-type-test.sh && safe --timeout 10 -- bash scripts/rust-fmt.sh"`

- Agent checklist (always do this before acting):
  - New slug? Copy the ticket template to `shared/tickets/<slug>.md`, edit it, and only then touch git/provision.
  - Read `shared/tickets/<slug>.md` (YAML front matter + Markdown body). Keep `status` truthful if you change it, and avoid touching the body unless a ticket explicitly asks for a rewrite.
  - Tail `shared/tickets/<slug>.log.jsonl` to review the most recent events (`provision`, `tNN-start`, `tNN-final`/`tNN-abort`). Do not rewrite past log lines.

- Ticket peer reviews (use Codex to sanity-check large/ambiguous specs before coding or handoff):
  - Typical command (run from repo root):  
    ```
    codex --yolo --cd /workspaces/rust-viterbo \
      --model gpt-5-codex -c reasoning_budget='"medium"' \
      exec 'You are reviewing ticket specs only ...' \
      > /tmp/codex-review-$(date +%s).txt
    ```
    Use shell redirection so long outputs never truncate inside the harness.
  - Model choice trade-offs: `gpt-5-codex` + medium reasoning is fast and good at shell/tool use; `gpt-5` + high reasoning yields slower but deeper critiques. Mix as needed (example: codex run for quick pass, then a slower follow-up if issues persist).
  - Prompts must forbid mutating commands and request critique dimensions (clarity, completeness, actionability, specificity). Keep sandbox read-only unless you have a strong reason otherwise.
  - When to run: before starting a new large ticket, before requesting review on a complicated spec, or when scope creep is suspected. Document outcomes in the ticket log or update the ticket body immediately.
  - Optional QoL flags (check `codex --help` for availability): `--color=never` to keep logs clean, `--json` or `--output-last-message` for structured captures, `--quiet/--no-internal-output` to suppress tool chatter.

- Ticket body structure and style:
  - Keep each ticket body deterministic and high-signal; follow this outline unless the owner specifies another template:
    1. **Generating idea / context** — one short paragraph capturing why the work exists (source insight, bug, or hypothesis).
    2. **Goals & constraints** — explicit success criteria plus constraint list with forgiveness notes (what is non-negotiable vs. stretch).
    3. **Final deliverable** — bullet list of concrete artifacts (code, docs, data) that prove the goal is met.
    4. **High-level plan** — 3–6 ordered steps that link the generating idea to the deliverable (tickets → thesis → code flow). Each step should be testable.
    5. **Mid-level plan & tradeoffs** — per-step detail covering major components, key decisions, and known tradeoffs (e.g., tooling choices, performance vs. scope). Reference specs/tickets via the `Docs:/Ticket:/Code:` comment convention when relevant.
    6. **Variations / adaptation hooks** — pre-approved pivots, fallback options, or monitoring notes that guide future agents if assumptions change.
  - Style rules: write in the same concise, explicit tone as AGENTS.md; favor lists over prose; note open questions; avoid duplicating AGENTS.md—link to sections instead. Treat the ticket body as immutable once work starts unless the owner updates it.

- Final message:
  - End each turn with a concise final message that explains what changed, how to validate (exact commands), and what’s next (if anything). agentx captures it into the ticket as `tNN-final.md`.

- Conventions:
  - Commands accept a slug only. Do not pass file paths or session ids.
  - Order is defined by the log timestamps; ignore filesystem mtimes.
  - The YAML front matter is authoritative and intentionally small. agentx writes/reads only `status`, `turn_counter`, and timestamps; you may edit `owner`, `depends_on`, `dependency_of` when needed. agentx derives slug/branch/worktree/turns/timestamps from the ticket file and log.

## API Policy (Internal Only)
- We have no stable public API. All Rust modules are project‑internal.
- Prefer better, clearer APIs over compatibility. Breaking changes are not just allowed, they’re expected when they improve quality or align us with the thesis/specs.
- Don’t carry legacy shims or deprecations unless a ticket explicitly asks for a staged transition. Keeping low churn for its own sake causes rot.
- Use `viterbo::api` and `viterbo::prelude` for convenience imports in internal code. These surfaces are curated for agents and may change at any time.
- If an external‑looking boundary appears (e.g., PyO3), treat it as internal too unless a ticket declares support guarantees for a specific consumer.
- The documentation, code comments, tests and scripts are focused on the current commit only. Do not mention past versions and do not attempt to maintain legacy compatibility layers or fallbacks.

## Everyday Tips and Tricks
- All output of your commands is truncated with warning beyond 250 lines or 10kB. It's a hard-coded limit in codex cli's harness.
  - When reading a file, print line numbers so you see from where to resume reading if truncation occurs.
  - When running commands that will produce long output, `tee` it to `/tmp` so you can if necessary read the full output in chunks from the file.
  - When writing documentation or code, try to split them into smaller files to stay below the limits.
  - When writing documentation that cannot be split, use sections with headers that can be `rg`ed individually.
  - When searching relevant code, you may like `rg` and 

<!-- END OF AGENTS.md -->
