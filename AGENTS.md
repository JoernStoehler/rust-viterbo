# AGENTS.md

This is the always‑relevant guide for coding agents. Keep it lean, clear, unambiguous, specific, correct, up-to-date and actionable. If anything is unclear, stop and escalate to the ticket owner.

## Active Temporary Notices
- None

## Source of Truth and Layers
- Tickets (Vibe Kanban) are the source of truth.
- Thesis specs in `docs/src/thesis/` define algorithms and data at a higher level.
- Code/tests implement the specs; data artifacts are outputs.
- Flow: tickets → thesis → code/tests → data. If problems are encountered, escalate to the thesis spec and tickets layers first.
- Cross‑refs in code or markdown `<!-- comments -->`:
  - `Docs: docs/src/thesis/<path>#<anchor>`
  - `Ticket: <uuid>`
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
- `data/` now rides through Git LFS instead of VK rsyncs. Run `git lfs pull --include "data/**" --exclude ""` after switching branches (or after a fresh worktree) to hydrate the pointers locally. `scripts/reproduce.sh` is the single source of truth for regenerating *every* artifact that shows up in the docs/thesis (bench tables, figures, data files, etc.). Whenever you add or change an artifact, update `scripts/reproduce.sh` in the same ticket so nobody ever has to guess whether it belongs there.
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
    - Both Rust wrappers default `CARGO_TARGET_DIR` if not set:
      - tests: `data/target/`
      - benches: `target/` (rsynced into `data/bench/criterion` afterward)
      - `data/target/` remains gitignored (even though the rest of `data/` is tracked via LFS) so transient Cargo outputs never pollute commits.
    - `paper-download.sh`: Fetch paper sources and PDFs into `data/downloads/`.
    - `vk.sh`: Local VK web server for the human project owner.
    - `vk-setup.sh`: VK worktree setup hook.

## Platform and Tooling
- Platform:
  - Orchestration in Python; Rust for hotspots (called from Python).
  - PyO3 + maturin; native module name is `viterbo_native` (re-exported as `viterbo._native`).
  - Interop via NumPy (`pyo3‑numpy`) for now; convert to/from Torch tensors in Python.
  - Geometry: `nalgebra`. Data wrangling: `polars`. RNG: `rand` in Rust, `random`, `numpy.random`, and `torch.manual_seed(...)` in Python.
  - No Jupyter notebooks.
- Vibe‑Kanban (VK) provisions the environment and worktree; agents do not perform manual setup unless a ticket explicitly asks for it.
- Development environment: everything runs inside a single VS Code devcontainer on the project owner’s Ubuntu desktop. There is one clone of the repo, no GitHub-hosted CI, and all automation (vk-setup, scripts/python-lint-type-test.sh, etc.) executes inside that container. Assume local resources; escalate before assuming external services exist.
- Tooling:
  - Python 3.11+ runtime; examples use `safe --timeout 60 -- uv run ...` for command execution.
  - Rust stable toolchain (see `rust-toolchain.toml`), with `rustfmt`, `clippy`.
  - Git LFS (latest 3.x). Run `git lfs install --local` once per worktree and `git lfs pull --include "data/**" --exclude ""` after switching branches so large artifacts are available locally.
  - Fast feedback: `bash scripts/python-lint-type-test.sh` (Python format/lint/type/test), then `bash scripts/rust-fmt.sh`, `bash scripts/rust-test.sh`, and `bash scripts/rust-clippy.sh` before running selective smoke/e2e tests.
  - Rust build cache strategy: sccache is enabled (`RUSTC_WRAPPER=sccache`) and all Rust builds default to a shared absolute target dir `CARGO_TARGET_DIR=/var/tmp/vk-target` to maximize cross‑worktree cache hits for third‑party crates. Occasional “blocking waiting for file lock” is expected and safe; locks are kernel‑released on process exit/crash, and `scripts/safe.sh` timeouts ensure cleanup.
  - Native extension: build/refresh via `safe -t 300 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml`. CI also builds natively to catch drift early. We do not publish to PyPI; packaging-for-distribution assumptions do not apply in this repo.

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
<!-- Ticket: 5ae1e6a6-5011-4693-8860-eeec4828cc0e -->
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

## Ticketing and VK Workflow
- VK manages tickets in a kanban board. Accessible via mcp function calls only.
- Project owner starts "attempts" (agents) on tickets; VK provisions a git worktree for each agent, runs a setup hook (`scripts/vk-setup.sh`), and starts the agent with the ticket description as first input. The hook recreates baseline dirs, installs Git LFS, and pulls tracked artifacts so agents begin with hydrated `data/` contents.
- After every agent turn, VK commits the worktree automatically; Please update `.gitignore` early if you plan to add files that need to be ignored; Do not rely on uncommitted state.
- Project owner can post follow-up messages to the agent, agent can pause and ask for clarifications.
- After the project owner closes the ticket, VK merges the ticket branch back to main. `target/` stays local-only, but everything under `data/` now merges through Git LFS, so always commit artifacts + provenance as part of the ticket.
- The human project owner runs a local VK server: `bash scripts/vk.sh` (serves on port 3000). Agents interact with VK via their MCP tools.

## Git Conventions
- Commit often; include `Ticket: <uuid>` in commit messages.
- No pre‑commit hooks; rely on `bash scripts/python-lint-type-test.sh`, `bash scripts/rust-fmt.sh`, `bash scripts/rust-test.sh`, `bash scripts/rust-clippy.sh`, and selective E2E runs for validation.
- VK automatically commits after every agent turn, but you can commit manually as needed.

## Command Line Quick Reference
- Wrap long/unknown‑cost commands in `scripts/safe.sh` with an explicit timeout; see “Safe Wrapper” section for policy and budgets.
  - Example: `bash scripts/safe.sh --timeout 10 -- uv run python -m viterbo.atlas.stage_build --config configs/atlas/test.json`
- Manual CI before handing in work to the project owner for merge:
  - `safe --timeout 300 -- bash scripts/ci.sh`
- Rust build cache hygiene:
  - Default target dir is global: `/var/tmp/vk-target` (set in devcontainer and wrappers). This enables sccache hits across VK worktrees.
  - Brief lock waits during overlapping builds are normal (“blocking waiting for file lock”). Locks are freed on process exit/crash or by `safe.sh` timeouts.
  - Cleanup when needed: `safe -t 60 -- cargo clean` (or remove `/var/tmp/vk-target` during downtime only).
- Get feedback fast after working on code:
  - `safe --timeout 10 -- bash scripts/python-lint-type-test.sh`
  - `safe --timeout 10 -- bash scripts/rust-fmt.sh`
  - `safe --timeout 120 -- bash scripts/rust-test.sh`
  - `safe --timeout 120 -- bash scripts/rust-clippy.sh`
  - `safe --timeout 10 -- uv run pytest -q tests/smoke/test_xyz.py::test_abc`
  - `safe --timeout 60 -- cargo test -q -p viterbo`
  - `safe --timeout 120 -- bash scripts/rust-test.sh -p viterbo -- -q`
  - `safe --timeout 300 -- uv run pytest -q -m e2e tests/e2e/test_atlas_build.py::test_build_dataset_tiny`
  - Atlas data (full): `safe --timeout 300 -- uv run python -m viterbo.atlas.stage_build --config configs/atlas/full.json`
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
- Use GitHub Pages to host the mdBook site at https://joernstoehler.github.io/rust-viterbo/ via `scripts/publish.sh`.
- Write in a clear, unambiguous, specific, actionable, explicit style with low cognitive overhead, so that development agents can read text and get to work quickly without needing to think through ambiguities or infer implications that weren't spelled out.
- Use KaTeX-safe math only (no `\\operatorname`).
- Create small tables/figures/interactive plots for inclusion in the mdBook site via `docs/assets/`.

### Thesis Writing Conventions (mdBook)
- Section layout (keep it brief and consistent):
  1) one-paragraph context; 2) Setting and Notation; 3) Definitions; 4) Main Facts/Theorems (with footnote citations); 5) “What We Use Later” bullets for algorithms; 6) optional “Deviations and clarifications for review”.
- Citations: use non-obstructive footnotes with author–year text; prefer official DOI or arXiv link; no proposition/page numbers (they vary across PDFs). Example: `[^HK19]: Haim‑Kislev (2019) ...`.
- Cross‑refs: link to sibling chapters via relative paths; avoid duplicate headers; prefer section names over numbers.
- HTML comments at the top: include `Ticket: <uuid>` and any brief editing notes for future agents.
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
- Schedule downtime: pause all VK attempts/worktrees besides your own and get the owner’s explicit go-ahead before rewriting history.
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

## API Policy (Internal Only)
- We have no stable public API. All Rust modules are project‑internal.
- Prefer better, clearer APIs over compatibility. Breaking changes are not just allowed, they’re expected when they improve quality or align us with the thesis/specs.
- Don’t carry legacy shims or deprecations unless a ticket explicitly asks for a staged transition. Keeping low churn for its own sake causes rot.
- Use `viterbo::api` and `viterbo::prelude` for convenience imports in internal code. These surfaces are curated for agents and may change at any time.
- If an external‑looking boundary appears (e.g., PyO3), treat it as internal too unless a ticket declares support guarantees for a specific consumer.
