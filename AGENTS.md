# AGENTS.md

This is the always‑relevant guide for coding agents. Keep it lean, specific, and actionable. If anything is unclear, stop and escalate to the ticket owner.

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

## Components and Repo Map
- High Performance Geometry and Algorithms in Rust:
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
  - `docs/src/meta/`: Meta documentation about project-specific conventions, workflows, and other development knowledge.
  - `docs/book.toml`: mdBook config.
- Data artifacts (gitignored; small publishable assets in `docs/assets/` are versioned):
  - `data/<experiment>/<artifact>.<ext>` with sidecar `data/<experiment>/<artifact>.<ext>.run.json`.
  - `data/downloads/`: Paper downloads (text sources + PDFs).
  - `docs/assets/`: Small data artifacts for publication (including interactive figures).
- Explicit, documented devops:
  - `AGENTS.md`: This file. Onboarding for all new agents.
  - `scripts/`: Devops scripts.
    - `safe.sh`: Must-use wrapper for potentially long-running commands (timeout + group kill).
    - `checks.sh`: Fast format/lint/typecheck/smoke tests for early feedback on code changes.
    - `ci.sh`: Manual full CI.
    - `reproduce.sh`: Reproduction entrypoint (as defined in README). Builds the code, runs tests (including E2E), regenerates data artifacts, and builds the mdBook. Also serves as a readable reference of the project’s dataflow.
    - `paper-download.sh`: Fetch paper sources and PDFs into `data/downloads/`.

## Platform and Tooling
- Platform:
  - Orchestration in Python; Rust for hotspots (called from Python).
  - PyO3 + maturin; native module name is `viterbo_native` (re-exported as `viterbo._native`).
  - Interop via NumPy (`pyo3‑numpy`) for now; convert to/from Torch tensors in Python.
  - Geometry: `nalgebra`. Data wrangling: `polars`. RNG: `rand` in Rust, `random`, `numpy.random`, and `torch.manual_seed(...)` in Python.
  - No Jupyter notebooks.
  - Vibe‑Kanban (VK) provisions the environment and worktree; agents do not perform manual setup unless a ticket explicitly asks for it.
- Tooling:
  - Python 3.11+ runtime; examples use `safe.sh --timeout 60 -- uv run ...` for command execution.
  - Rust stable toolchain (see `rust-toolchain.toml`), with `rustfmt`, `clippy`.
  - Fast feedback: `bash scripts/checks.sh` runs ruff format/check, pyright (basic), pytest (non‑e2e), and cargo check/test.
  - Optional native build is available via maturin (only if a ticket requires native code changes; see Quick Reference).

## Ticketing and VK Workflow
- VK manages tickets in a kanban board. Accessible via mcp function calls only.
- Project owner starts "attempts" (agents) on tickets; VK provisions a git worktree for each agent, copies `data/`, runs a setup hook (`scripts/vk-setup.sh`), and starts the agent with the ticket description as first input.
- After every agent turn, VK commits the worktree automatically; Please update .gitignore early if needed, do not rely on uncommitted state.
- Project owner can post follow-up messages to the agent, agent can pause and ask for clarifications.
- After the project owner closes the ticket, VK merges the ticket branch back to main. Gitignored paths (`data/`, `target/`) never merge; Instead we regenerate on main or in worktrees by running the new/relevant sections of `bash scripts/reproduce.sh`.
- (situational) Humans may run a local VK server: `bash scripts/vk.sh` (serves on port 3000). Agents interact with VK via MCP tools.

## Git Conventions
- Commit often; include `Ticket: <uuid>` in commit messages.
- No pre‑commit hooks; rely on `bash scripts/checks.sh` and selective E2E runs for validation.

## Command Line Quick Reference
- For any command that may run a long time or hang, wrap it in `scripts/safe.sh` with an explicit timeout to catch unexpected issues:
  - Pattern: `bash scripts/safe.sh --timeout <seconds> -- <your command here>` or in short `safe -t <seconds> -- <your command here>`
  - Example: `bash scripts/safe.sh --timeout 10 -- uv run python -m viterbo.atlas.stage_build --config configs/atlas/test.json`
  - Timeouts are in seconds; pick a value that is safely above expected runtime but low enough to catch bugs or mistakenly started operations.
- Manual CI before handing in work to the project owner for merge:
  - `safe --timeout 300 -- bash scripts/ci.sh`
- Get feedback fast after working on code:
  - `safe --timeout 10 -- bash scripts/checks.sh`
  - `safe --timeout 10 -- uv run pytest -q tests/smoke/test_xyz.py::test_abc`
  - `safe --timeout 60 -- cargo test -q -p viterbo`
  - `safe --timeout 300 -- uv run pytest -q -m e2e tests/e2e/test_atlas_build.py::test_build_dataset_tiny`
  - Atlas data (full): `safe --timeout 300 -- uv run python -m viterbo.atlas.stage_build --config configs/atlas/full.json`
- Avoid auto‑running all E2E tests. Select by hand; it’s way faster and clearer.
- Native build: `safe -t 300 -- uvx maturin develop -m crates/viterbo-py/Cargo.toml`.

## Rust Conventions
- Use Rust for hotspots only; profile first.
- Use `nalgebra` for fixed-size geometry (e.g., `Vector4<f64>`).
- Use property tests (`proptest`) where appropriate to skip hand-written values.
- Use `criterion` for benchmarks; write results to `data/bench/` (gitignored).
- Expose functions to Python via PyO3 in `crates/viterbo-py`.
- Functional style preferred.
- Comment to reference tickets and thesis specs.
- Comment to explain the why, not the what.
- Avoid over-abstraction; prefer simple, explicit, locally understandable code.

## Data and Pipeline Conventions
- Data artifacts go to `data/<experiment>/...` (gitignored; small publishable assets go to `docs/assets/`, which is versioned).
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
- Rust cores (algorithms): unit tests + property tests required; benchmarks with `criterion` under `data/bench/`.
- Python orchestration: smoke tests and selective E2E on tiny configs; add unit tests when logic is non‑trivial.
- CI defaults: `scripts/checks.sh` (ruff/pyright/pytest non‑e2e + cargo) and on‑demand E2E by selection (`-m e2e -k ...`).
- Default: prefer smoke + E2E over broad Python unit test suites unless justified by complexity.

## Documentation Conventions
- High-level specs in `docs/src/thesis/` about the mathematics, algorithms, data formats, and experiment ideas.
- Meta documentation in `docs/src/meta/` about project-specific conventions, workflows, and reminders that fix encountered mistakes.
- Keep `AGENTS.md` lean and always relevant; move situational info to `docs/src/meta/` with clear "when to read" hints in `docs/src/meta/README.md`
- Use GitHub Pages to host the mdBook site at https://joernstoehler.github.io/rust-viterbo/.
- Write in a clear, unambiguous, specific, actionable, explicit style with low cognitive overhead, so that development agents can read text and get to work quickly without needing to think through ambiguities or infer implications that weren't spelled out.
- Use KaTeX-safe math only (no `\\operatorname`); verify via GitHub preview.
- Publish small tables/figures/interactive plots for inclusion in the mdBook site.

## Escalation
- Escalate when: specs underspecified, solver/library choice blocks progress, runtime exceeds budget, or scope bloat requires a sub‑ticket.
- How: pause work, leave a concise summary + options.

## Design Principles
- High performance: use Rust for hotspots; profile first.
- Fast development cycles: use Python for orchestration; isolate experiments for parallel development; tiny test configs for fast feedback.
- Mathematical correctness: specify in the thesis; provide correctness arguments; write robust tests for algorithmic cores.
- Reproducibility: provenance sidecars for all data artifacts; `reproduce.sh` to rebuild everything from scratch.
- Simplicity and Maintainability: favor explicit and simple code over clever abstractions; document the why, not the what; pick popular, well-known patterns, libraries and workflows.
- Self‑documenting project: record conventions, workflows, and reminders in `docs/src/meta/`; cross-reference tickets and thesis specs.
- AI Agents as first-class developers: design everything for easy onboarding of new agents; clear, specific, and actionable tickets; maintain always‑relevant knowledge in a lean `AGENTS.md`, and move situational info to `docs/src/meta/` with clear "when to read" hints in `docs/src/meta/README.md`; avoid overhead for agents, reduce tool friction and keep related information close together to minimize search time; 
- Continuous Improvement: accept feedback from agents and the project owner; refactor with breaking changes, rewrite documentation, open additional tickets when it raises the quality of the project for future agents.
