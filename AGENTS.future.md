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
    - `utils/`: General helpers.
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
  - The documentation is hosted on GitHub Pages at https://joernstoehler.github.io/rust-viterbo/.
- Data artifacts (gitignored):
  - `data/<experiment>/<artifact>.<ext>[.json]`: Every artifact has a JSON sidecar next to it with provenance information (config, git commit, timestamp, command).
  - `data/downloads/`: Paper downloads (text sources + PDFs).
  - `docs/assets/`: Small data artifacts for publication (including interactive figures).
- Explicit, documented devops:
  - `AGENTS.md`: This file. Onboarding for all new agents.
  - `scripts/`: Devops scripts.
    - `safe.sh`: Must-use wrapper for potentially long-running commands (timeout + group kill).
    - `checks.sh`: Fast format/lint/typecheck/smoke tests for early feedback on code changes.
    - `ci.sh`: Manual full CI.
    - `reproduce.sh`: Full reproduction of the entire thesis from scratch. Useful to look up and document the overall dataflow.
    - `paper-download.sh`: Fetch paper sources and PDFs into `data/downloads/`.

## Tech Choices
- Orchestration in Python. Hotspots in Rust; always called from Python.
- PyO3 + maturin; module name: `viterbo._native`.
- Interop via NumPy (`pyo3‑numpy`) for now; convert to/from Torch tensors in Python.
- Geometry: `nalgebra`. Data wrangling: `polars`. RNG: `rand` in Rust, `numpy.random`/`random`/`torch.random` in Python.
- No Jupyter notebooks.
- Vibe-Kanban (VK) for managing tickets and worktrees.

## Ticketing and VK Workflow
- VK manages tickets in a kanban board. Accessible via mcp function calls only.
- Project owner starts "attempts" (agents) on tickets; VK provisions a git worktree for each agent, copies `data/`, runs a setup hook (`scripts/vk-setup.sh`), and starts the agent with the ticket description as first input.
- After every agent turn, VK commits the worktree automatically; Please update .gitignore early if needed, do not rely on uncommitted state.
- Project owner can post follow-up messages to the agent, agent can pause and ask for clarifications.
- After the project owner closes the ticket, VK merges the ticket branch back into main. Gitignored paths (`data/`, `target/`) never merge; Instead we regenerate on main or in worktrees by running the new/relevant sections of `bash scripts/reproduce.sh`.

## Command Line Quick Reference
- For any command that may run a long time or hang, wrap it in `scripts/safe.sh` with an explicit timeout to catch unexpected issues:
  - `bash scripts/safe.sh --timeout <seconds> -- <your command here>`
  - Example: `bash scripts/safe.sh --timeout 300 -- uv run python -m viterbo.pipeline.atlas.build --config configs/atlas/full.json`
  - The timeout is in seconds; pick a value beyond which you'd consider the command to have taken abnormally long.
  - The command is run in a subprocess group; on timeout, all subprocesses are killed.
  - The exit code of `safe.sh` is that of the command, or 124 on timeout.
- Manual CI before handing in work to the project owner for merge:
  - `bash scripts/safe.sh --timeout 300 -- bash scripts/ci.sh`
- Run ad-hoc python or bash code for multi-line operations:
  ```bash
  bash scripts/safe.sh --timeout 10 -- uv run python -c <<EOF
  import viterbo
  # your code here
  EOF
  ```
- Get feedback fast after working on code:
  - `bash scripts/safe.sh --timeout 10 -- bash scripts/checks.sh`
  - `bash scripts/safe.sh --timeout 10 -- uv run pytest -q tests/smoke/test_xyz.py::test_abc`
  - `bash scripts/safe.sh --timeout 60 -- cargo test -q -p viterbo`
  - `bash scripts/safe.sh --timeout 300 -- uv run pytest -q -m e2e tests/e2e/atlas/test_atlas_build.py::test_NaNs_absent`
- Avoid auto‑running all E2E tests. Select by hand; it’s way faster and clearer.

## Rust Conventions
- Use Rust for hotspots only; profile first.
- Use `nalgebra` for fixed-size geometry (e.g., `Vector4<f64>`).
- Use property tests (`proptest`) where appropriate to skip hand-written values.
- Use `criterion` for benchmarks; write results to `data/bench/` (gitignored).
- Expose functions to Python via PyO3 in `crates/viterbo-py`.
- Functional style preferred.
- Comment to reference tickets and thesis specs.
- comment to explain the why, not the what.
- Avoid over-abstraction; prefer simple, explicit, locally understandable code.

## Data and Pipeline Conventions
- Data artifacts go to `data/<experiment>/...` (gitignored).
- Every artifact `X.ext` has a provenance sidecar `X.ext.json`.
- Stages are modules `bash scripts/safe.sh --timeout 300 -- uv run python -m viterbo.<experiment>.stage_<name> --config configs/<experiment>/<config>.json`.
- The json config specifies all constants, paths, and parameters.
- Keep stages composable; reuse helpers; do not over‑abstract (YAGNI, KISS).
- Provide tiny test config variants for fast dev cycles (≤10s); Use E2E tests to assert on the outputs of the test configs.
- Rust kernels do not write provenance; Python orchestrator owns it.

## Python Conventions
- Use basic type hints where it disambiguates; Pyright basic only needed.
- Favor immutable/functional style; move imperative orchestration closer to the command line entry points.
- Use `numpy`, `torch`, `polars` for data wrangling.
- Always run via `scripts/safe.sh` and `uv run` to get timeouts and the right environment.
- Comment to reference tickets and thesis specs.
- Comment to explain the why, not the what.
- Repeat code rather than prematurely abstracting; stabilize common code only once experiments stabilize.
- Use `tests/scratch/` for on the fly testing that can be deleted once done. No need to maintain large sets of unit tests.
- Or move tests to `tests/smoke/` if important to keep around long-term, e.g. to detect future regressions.
- Use `tests/e2e/` with `@pytest.mark.e2e` to make assertions on the data artifacts produced by pipeline stages, especially test configs that run fast. Also add assertions into the production pipeline stages where appropriate to catch bugs that tests may miss.
- Don't use Jupyter notebooks. You do not have command line tools that can interact with them. Instead use multi-line python commands, or scratch/smoke tests, or small python scripts.

## Documentation Conventions
- High-level specs in `docs/src/thesis/` about the mathematics, algorithms, data formats, and experiment ideas.
- Meta documentation in `docs/src/meta/` about project-specific conventions, workflows, and reminders that fix encountered mistakes.
- Keep `AGENTS.md` lean and always relevant; move situational info to `docs/src/meta/` with clear "when to read" hints in `docs/src/meta/README.md`
- Use GitHub Pages to host the mdBook site at https://joernstoehler.github.io/rust-viterbo/.
- Write in a clear, unambiguous, specific, actionable, explicit style with low cognitive overhead, so that development agents can read text and get to work quickly without needing to think through ambiguities or infer implications that weren't spelled out.
- Use markdown comments `<!-- ... -->` to reference tickets and thesis specs.
- Use KaTeX-safe math only (no `\operatorname`); verify via GitHub preview.
- Publish tables/figures/interactive plots to `docs/assets/` for inclusion in the mdBook site.

## Escalation
- Escalate when: specs underspecified, solver/library choice blocks progress, runtime exceeds budget, or scope bloat requires a sub‑ticket.
- How: pause work, leave a concise summary + options.

## Design Principles and Goals of this Project
- High performance: use Rust for hotspots; profile first.
- Fast development cycles: use Python for orchestration; isolate experiments for parallel development; tiny test configs for fast feedback.
- Mathematical Correctness: spec in the thesis; correctness theorems and formal arguments/proofs; excessive tests of the algorithms and their edge cases.
- Reproducibility: provenance sidecars for all data artifacts; `reproduce.sh` to rebuild everything from scratch.
- Simplicity and Maintainability: favor explicit and simple code over clever abstractions; document the why, not the what; pick popular, well-known patterns, libraries and workflows.
- Self‑documenting project: record conventions, workflows, and reminders in `docs/src/meta/`; cross-reference tickets and thesis specs.
- AI Agents as first-class developers: design everything for easy onboarding of new agents; clear, specific, and actionable tickets; maintain always‑relevant knowledge in a lean `AGENTS.md`, and move situational info to `docs/src/meta/` with clear "when to read" hints in `docs/src/meta/README.md`; avoid overhead for agents, reduce tool friction and keep related information close together to minimize search time; 
- Continuous Improvement: accept feedback from agents and the project owner; refactor with breaking changes, rewrite documentation, open additional tickets when it raises the quality of the project for future agents.