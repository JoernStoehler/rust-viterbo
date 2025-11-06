# AGENTS.md — onboarding after the Python‑first migration

This is the always‑relevant guide for coding agents after the tech‑stack refactor. Keep it lean, specific, and actionable. If anything is unclear, stop and escalate to the ticket owner.

## Active Temporary Notices
- None

## Source of Truth and Layers
- Tickets (Vibe Kanban) are the source of truth.
- Thesis specs in `docs/src/thesis/` define algorithms and data at a higher level.
- Code/tests implement the specs; data artifacts are outputs.
- Flow: tickets → thesis → code/tests → data. If code is unclear, fix the ticket/spec first.
- Cross‑refs in code or markdown comments:
  - `Docs: docs/src/thesis/<path>#<anchor>`
  - `Ticket: <uuid>`
  - `Code: <path>::<symbol>`

## Repo Map (post‑migration)
- `Cargo.toml` (workspace), `pyproject.toml` (Python package + maturin build)
- `crates/`
  - `crates/viterbo` — Rust math/geometry kernels (`nalgebra`). Unit/property tests + benches live here.
  - `crates/viterbo-py` — PyO3 glue exposing `_native` to Python. No provenance here.
- `src/viterbo/` (Python package; import as `viterbo`)
  - `pipeline/` — pipeline stages and entry modules (one file per stage; group by topic).
  - `dataset/` — schemas, IO helpers (Polars/PyArrow).
  - `rust/` — thin Python wrappers around `_native` (NumPy in/out).
  - `provenance.py` — helpers to write simple JSON sidecars (see “Provenance”).
  - `utils/` — logging, timers, seeds, small helpers.
  - `__main__.py` (optional router for `python -m viterbo ...`).
- `configs/` — pipeline configs in JSON (dev: tiny; prod: full); versioned by file name.
- `scripts/`
  - `safe.sh` — run long commands with timeout + group kill.
  - `checks.sh` — fast format/lint/typecheck/smoke tests only (no E2E).
  - `ci.sh` — manual CI (can toggle E2E explicitly).
  - `reproduce.sh` — rebuild kernels, run tiny pipelines, refresh warm caches.
  - `all_e2e.sh` — optional: run selected E2E tests sequentially.
  - `paper-download.sh` — fetch sources; calls Python provenance helpers.
- `tests/`
  - `smoke/` — fast Python smoke tests (always run in checks).
  - `e2e/` — on‑demand E2E that assert on produced artifacts.
  - `scratch/` — ephemeral tests for dev; safe to delete.
- `docs/` — mdBook, thesis, small publishable assets in `docs/assets/`.
- `data/` — heavy artifacts (gitignored). Sidecars live next to outputs.

## Tech Choices
- Orchestration in Python. Hotspots in Rust; always called from Python.
- PyO3 + maturin; module name: `viterbo._native`.
- Interop via NumPy (`pyo3‑numpy`) for now; convert to/from Torch tensors in Python.
- Geometry: `nalgebra`. Data wrangling: `polars` (no feature gating). RNG: `rand` in Rust, `numpy.random`/`random` in Python.
- No Jupyter notebooks.

## Quickstart (first 5 minutes)
- Build environment (inside the ticket worktree):
  - `bash scripts/safe.sh --timeout 180 -- uv sync`
  - `bash scripts/safe.sh --timeout 180 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml`
- Sanity import:
  - `bash scripts/safe.sh --timeout 30 -- uv run python -c "import viterbo, viterbo._native; print('ok')"`
- Run tiny pipeline (example: atlas):
  - `bash scripts/safe.sh --timeout 120 -- uv run python -m viterbo.pipeline.atlas.build --config configs/atlas/test.json`
- Optional E2E assert for the above:
  - `bash scripts/safe.sh --timeout 30 -- uv run pytest -q -m e2e tests/e2e/test_atlas_build.py::test_build_dataset_tiny`

## Developer Loop
- Default to Python for iteration; promote kernels to Rust when profiling shows hotspots.
- Use `safe.sh` for anything that may run > a few seconds or can hang.
- Fast signal:
  - `bash scripts/safe.sh --timeout 60 -- bash scripts/checks.sh`
- Targeted tests (recommended):
  - `bash scripts/safe.sh --timeout 45 -- uv run pytest -q tests/smoke/test_xyz.py::test_abc`
  - `bash scripts/safe.sh --timeout 120 -- uv run pytest -q -m e2e -k "atlas and tiny"`
- Avoid auto‑running E2E in the background. Select by hand; it’s faster and clearer.

## Pipelines (no DSL)
- A stage is a plain Python callable in `src/viterbo/pipeline/<topic>/<stage>.py` with signature `(config: dict, ctx: dict) -> Artifact` (pragmatic, no strict typing).
- Entry points are modules: `python -m viterbo.pipeline.<topic>.<entry> --config configs/<topic>/test.json`.
- Keep stages small and composable; reuse helpers; do not over‑abstract.
- Production vs tiny: provide a tiny config finishing in ≤10s and a full config for long runs.

## Provenance (intentionally simple, evolves later)
- Configs are JSON. Provenance is a JSON sidecar placed next to each artifact.
- Convention for any output `X.ext`:
  - Sidecar path: `X.ext.run.json` with fields:
    - `config` (the full config object as used after any in‑process edits),
    - `git_commit`, `timestamp`, `command`, `exit_code`,
    - optional: `inputs`, `outputs`, `version`, `seed`, `profile_path`, `notes`.
- Write via `viterbo.provenance.write(output_path, config: dict, extras: dict = {})`.
- Rust kernels do not write provenance; Python orchestrator owns it.
- Heterogeneous is OK for now; we will converge once needs are clear.

## Testing Strategy
- Python:
  - `tests/smoke/` is fast and always part of `checks.sh`.
  - `tests/e2e/` are on‑demand and assert on produced artifacts. Mark with `@pytest.mark.e2e`.
  - Prefer runtime `assert` in source for invariants over excessive unit tests.
  - Ephemeral tests go in `tests/scratch/` and may be deleted at will.
- Rust:
  - Unit/property tests live with kernels in `crates/viterbo/tests/`.
  - Criterion benches write results to `data/bench/` (gitignored).

## VK Workflow (ticket → worktree → merge)
- VK provisions each ticket into its own git worktree.
- Every agent turn ends with VK committing the worktree automatically; do not rely on uncommitted state.
- After an agent finishes, VK merges the ticket branch back to main. Gitignored paths (`data/`, `target/`) never merge; regenerate on main via `bash scripts/reproduce.sh`.

## Daily Conventions
- Favor immutable/functional style; isolate imperative kernels.
- Determinism: seed in Python and pass through to Rust; record the seed in provenance if relevant.
- Outputs:
  - heavy → `data/...` (with `*.run.json` sidecars),
  - publishable small → `docs/assets/...`,
  - downloads → `data/downloads/` (kept warm by `paper-download.sh`).
- Long commands must go through `scripts/safe.sh`.
- No notebooks.

## Manual CI
- `scripts/checks.sh` runs:
  - `ruff format` + `ruff check` on `src/` and `tests/`,
  - `pyright` (basic) on `src/` and `tests/`,
  - `cargo check` and fast `cargo test -q -p viterbo`,
  - `pytest -q -m "not e2e"` (smoke only).
- E2E never runs by default; enable explicitly (e.g., `bash scripts/all_e2e.sh` or a targeted pytest command).

## Libraries
- Python: `numpy`, `polars`, `pyarrow`, `pytest`, `uv`, `ruff`, `pyright`.
- Rust: `nalgebra`, `rand`, `proptest` (optional), `criterion`, `pyo3`, `pyo3‑numpy`.

## Escalation
- Escalate when: specs underspecified, solver/library choice blocks progress, runtime exceeds budget, or scope bloat requires a sub‑ticket.
- How: pause work, leave a concise summary + options on the ticket.

## What “Good” Looks Like
- One obvious place for each new stage/module.
- Tiny config completes in ≤10s; full config documented.
- Provenance sidecar present next to every artifact produced by pipelines.
- Checks pass quickly; E2E runs are explicit and selective.
- Rust hotspots have benchmarks; Python is readable with asserts and small helpers.

## Handy Commands
- Fast hygiene: `bash scripts/safe.sh --timeout 60 -- bash scripts/checks.sh`
- Tiny run: `bash scripts/safe.sh --timeout 120 -- uv run python -m viterbo.pipeline.atlas.build --config configs/atlas/test.json`
- Targeted E2E: `bash scripts/safe.sh --timeout 120 -- uv run pytest -q -m e2e -k "atlas and tiny"`
- All E2E (sequential): `bash scripts/safe.sh --timeout 900 -- bash scripts/all_e2e.sh`

