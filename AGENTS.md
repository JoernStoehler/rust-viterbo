# AGENTS.md

Always‑relevant guide for coding agents. Keep it lean, explicit, correct, up‑to‑date, and actionable. If anything is unclear, stop and escalate to the ticket owner.

## Active Temporary Notices
- None

## Source of Truth and Layers
- Tickets (file‑based via `agentx`) are the source of truth.
- Thesis specs in `docs/src/thesis/` define algorithms and data at a higher level.
- Code/tests implement the specs; data artifacts are outputs.
- Flow: tickets → thesis → code/tests → data. Escalate to thesis/tickets first when blocked.
- Cross‑refs in code or markdown `<!-- comments -->`:
  - `Docs: docs/src/thesis/<path>#<anchor>`
  - `Ticket: <slug>`
  - `Code: <path>::<symbol>`
- All code files start with a short comment block explaining purpose, architecture rationale, and links for further reading.

## Components and Repo Map
- Rust core:
  - `crates/viterbo`: math and core algorithms (Rust lib). `nalgebra` for fixed‑size geometry; unit tests, property tests (`proptest`), and benches (`criterion`).
  - `crates/viterbo-py`: PyO3 glue exposing `_native`.
- Python orchestration:
  - `src/viterbo/` namespaces; experiments live in `src/viterbo/<experiment>/`.
  - Stages: `<experiment>/stage_<name>.py`.
  - Provenance: `provenance.py` writes JSON sidecars next to artifacts.
  - Thin wrappers around `_native` under `src/viterbo/rust/`.
  - Tests: `tests/smoke/`, `tests/scratch/` (ephemeral), `tests/e2e/` (on‑demand).
  - Configs: `configs/<experiment>/` (tiny test + full production).
- Docs:
  - `docs/src/thesis/` — high‑level specs.
  - `docs/src/meta/` — meta workflows and situational guidance.
  - `docs/book.toml` — mdBook config.
- Data:
  - `data/<experiment>/<artifact>.<ext>` plus `<ext>.run.json` sidecar (Git LFS).
  - `data/downloads/` paper sources/PDFs (Git LFS).
  - `docs/assets/` small publishable artifacts.
- Reproduction:
  - `scripts/reproduce.sh` regenerates all artifacts referenced by docs/thesis. Update it with every new/changed artifact.
- Devops scripts (`scripts/`):
  - `safe.sh` (timeout + group kill), `python-lint-type-test.sh`, `rust-fmt.sh`, `rust-test.sh`, `rust-clippy.sh`, `ci.sh`,
    `rust-bench*.sh`, `paper-download.sh`.

## Platform and Tooling
- Platform: Python orchestration; Rust for hotspots (called from Python).
- Native: PyO3 + maturin; module `viterbo_native` re‑exported as `viterbo._native`.
- Interop: NumPy (`pyo3‑numpy`); convert to/from Torch tensors in Python.
- Key libs: `nalgebra`, `polars`; RNG: Rust `rand`, Python `random`/`numpy.random`/`torch.manual_seed`.
- No Jupyter notebooks.
- Environment: single VS Code devcontainer on the owner’s Ubuntu desktop; no GitHub CI.
- Tooling/feedback loops:
  - Python: `bash scripts/python-lint-type-test.sh`
  - Rust: `bash scripts/rust-fmt.sh`, `bash scripts/rust-test.sh`, `bash scripts/rust-clippy.sh`
  - Docs: `mdbook build docs` (wrap long commands with `safe.sh`)
- Caches: `RUSTC_WRAPPER=sccache`, shared target dir `CARGO_TARGET_DIR=.persist/cargo-target`.
- Native build: `safe -t 300 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml`.

## Workspace Model and Boundaries
- Mental model:
  - One Git repository (“main repo”) plus additional Git worktrees under `.persist/agentx/worktrees/**` created by `agentx`.
  - Active contexts are the main repo root and any ticket worktree root; tools must not assume the current working directory equals the repo root.
  - Stable ticket entrypoint is `shared/tickets/<slug>/` at the main repo root and at each worktree root (exact symlink to `.persist/agentx/tickets`). Never elsewhere.
- Allowed writes by automation (`agentx` and its helpers):
  - `.persist/agentx/tickets/<slug>/**` (bundle, meta/body/messages)
  - `.persist/agentx/worktrees/<slug>/**` (worktree contents)
  - `<worktree>/.tx/**` (runtime scratch)
  - `shared/tickets` symlink at repo root and each worktree root (exact target: `.persist/agentx/tickets`)
- Naming and branches:
  - Slugs match `^[a-z0-9][a-z0-9._-]{0,63}$`.
  - Branch for a ticket: `ticket/<slug>`.
- Merges and cleanliness:
  - The `agentx merge` helper requires a clean target worktree and performs a fast‑forward only merge. For custom flows (rebase/squash), run `git` manually.
- Long‑running turns:
  - Turns can be long. Default run cap is 10 hours via `AGENTX_RUN_TIMEOUT=36000`; set `AGENTX_RUN_TIMEOUT=0` to disable explicitly.
- Hooks:
  - Hooks (`AGENTX_HOOK_*`) run if set and are bounded to 10s each; keep them quick and local. They may be empty.

## Safe Wrapper (timeouts & cleanup)
- Wrap long/unknown‑cost commands: `bash scripts/safe.sh --timeout <s> -- <cmd>`.
- Budgets (guide): 10–20s format/lint/small tests; 60–120s `cargo test` (single crate); 300–600s selected E2E/benches/mdBook.
- Policy: top‑level only; scripts expect `SAFE_WRAPPED=1`. Do not nest `safe.sh`.
- Reproduce script self‑wraps; ok to run directly or wrapped.
- On timeout: non‑zero exit; entire process group killed. Do not auto‑retry inside scripts.

### Agent Autonomy (verification defaults)
<!-- Ticket: 5ae1e6a6-5011-4693-8860-eeec4828cc0e -->
- Prefer fast local loops without asking the owner:
  - Python quick loop, Rust fmt/test/clippy, mdBook quick build.
  - Optional native build for code paths that depend on it.
- Escalate only for destructive actions, unusual long budgets, or external services.
- Always summarize commands and key logs when you ran something.

## Git Conventions
- Commit often; include `Ticket: <slug>` in messages.
- No pre‑commit hooks; rely on the quick loops and selective E2E.

## Command Line Quick Reference
- Wrap heavy/unknown runs: `safe -t <s> -- <cmd>`.
- Manual CI: `safe -t 300 -- bash scripts/ci.sh`.
- Python quick loop: `safe -t 10 -- bash scripts/python-lint-type-test.sh`.
- Rust loops: `safe -t 10 -- bash scripts/rust-fmt.sh`; `safe -t 120 -- bash scripts/rust-test.sh`; `safe -t 120 -- bash scripts/rust-clippy.sh`.
- Docs: `safe -t 120 -- mdbook build docs`.
- Native extension: `safe -t 300 -- uv run maturin develop -m crates/viterbo-py/Cargo.toml`.
- Target dir hygiene: default `.persist/cargo-target`; occasional lock waits are normal. Cleanup: `safe -t 60 -- cargo clean` (rare).

## Rust Conventions
- Use Rust for hotspots; profile first.
- `nalgebra` for fixed‑size geometry (e.g., `Vector4<f64>`).
- Property tests (`proptest`) where fitting; benches (`criterion`) to `data/bench/` (Git LFS) with summaries in docs when reviewers need diffable numbers.
- Expose to Python via PyO3 in `crates/viterbo-py`.
- Functional style preferred; explain “why” in comments; avoid over‑abstraction.

## Data and Pipeline Conventions
- Artifacts under `data/<experiment>/...` (Git LFS) plus `<ext>.run.json` sidecar.
- Sidecar schema: `config`, `git_commit`, `timestamp` via `viterbo.provenance.write(path, config)`.
- Stages run as Python modules:
  - `safe -t <s> -- uv run python -m viterbo.<experiment>.stage_<name> --config configs/<experiment>/<cfg>.json`
- Keep stages composable and explicit; prefer tiny test configs (≤10s) and E2E assertions on produced artifacts.
- Rust kernels do not write provenance; Python orchestrator owns it.
- Build caches live in `.persist/cargo-target`; never under `data/`.

## Seeding and Determinism (situational)
- Put `"seed"` in JSON configs.
- Python: seed `random`, `numpy`, `torch` (include CUDA when applicable).
- Rust: accept seed parameters as relevant; property tests use fixed seeds.

## Python Conventions
- Use type hints where they disambiguate (Pyright basic).
- Favor immutable/functional style; keep imperative orchestration near entry points.
- Repeat code freely in experiments; only stabilize shared code once ≥2 experiments depend on it.
- Tests: keep important smoke tests; use `tests/scratch/` for disposable checks; E2E assert on artifacts.

## Testing Policy
- Rust cores: unit tests + property tests required; `criterion` benches under `data/bench/` (Git LFS).
- Python orchestration: smoke + selective E2E on tiny configs; add unit tests for non‑trivial logic.
- CI defaults: run the quick loops; add benches → docs stage when needed.

## Documentation Conventions
- Specs live in `docs/src/thesis/`; meta workflows in `docs/src/meta/`.
- Reading rule: agents read AGENTS.md end‑to‑end in one pass before starting work.
- Authoring rules for this file (critical):
  - Total file length ≤ 250 lines; no “Quick Start” or duplicate summaries.
  - Avoid duplication; improve existing sections or link anchors instead.
  - Prefer `README.md` or `agentx --help` for human orientation; keep this file the canonical contract.
  - Retrieval‑friendly structure: unique `##` headers; begin sections with short mental‑model bullets.
- Publish docs via `scripts/publish.sh` to GitHub Pages when needed.

## Escalation
- Escalate when specs are underspecified, solver/library choice blocks progress, runtime exceeds budget, or scope bloat requires a sub‑ticket.
- Pause work, summarize options concisely, and propose next steps.

## Design Principles
- Performance: use Rust for hotspots; profile first.
- Speed: Python orchestration; isolate experiments; tiny test configs for fast cycles.
- Correctness: specify in thesis; argue correctness; write robust tests for algorithmic cores.
- Reproducibility: provenance sidecars for all data artifacts; `reproduce.sh` rebuilds everything.
- Simplicity: explicit code over clever abstractions; explain the “why”.
- Self‑documenting: keep conventions/workflows in docs; cross‑reference tickets and thesis.
- Agents first: optimize for onboarding; keep `AGENTS.md` lean; move only situational material to `docs/src/meta/`.

## Ticketing Workflow (agentx)
<!-- Ticket: 5ae1e6a6-5011-4693-8860-eeec4828cc0e -->
Minimal CLI with folder‑based “ticket bundles”.

- Model (1–1–1–1):
  - One ticket bundle ↔ one git branch ↔ one git worktree ↔ one Codex session.
  - Bundles: `.persist/agentx/tickets/<slug>/` (symlinked into each worktree at `shared/tickets/`).
  - Contents: `meta.yml` (authoritative `status`, optional `owner`, `depends_on`, `dependency_of`), `body.md`, and message files: `...-provision.md`, `...-tNN-start.md`, `...-tNN-final.md`/`...-tNN-abort.md`.
- Naming:
  - Slug: `^[a-z0-9][a-z0-9._-]{0,63}$` (lowercase; short, filesystem‑safe).
  - Branch: `ticket/<slug>`; worktree path: `.persist/agentx/worktrees/<slug>/`.
- Symlink policy (strict):
  - `shared/tickets` exists exactly at the main repo root and at each worktree root, and must point to `.persist/agentx/tickets`. If a different path exists there, fix/remove it; `agentx` will not overwrite.
- Event rules (strict):
  - Exactly one `provision` before any turn.
  - For each turn N: exactly one `tNN-start` and exactly one terminal event: `tNN-final` OR `tNN-abort`.
  - Turns strictly increase: `t01, t02, …`. Use `agentx start` to begin the next turn.
- Status semantics:
  - `open` (provisioned), `active` (between start and terminal), `stopped` (last terminal was abort), `done` (last terminal was final).
- Commands (slug‑only):
  - `agentx provision <slug> [--body-file path] [--inherit-from <slug>] [--base <ref>] [--copy src[:dst]]...`
  - `agentx start <slug>` — starts a turn; writes `tNN-start`; writes `tNN-final` on success.
  - `agentx abort <slug>` — writes `tNN-abort`, sets `status=stopped`, kills tmux window.
  - `agentx await <slug> [--timeout N]` — waits until `status != active`.
  - `agentx read <slug> [--events N] [--json]` — prints `meta.yml`, `body.md` path, last N events.
  - `agentx info <slug> [--fields a,b,c]`, `agentx list [--status s]`, `agentx merge <from> [<into>]`, `agentx doctor <slug>`.
- Hooks (optional per worktree):
  - Run if set; each hook is bounded to 10s by default. Keep them local and fast.
  - Example: `AGENTX_HOOK_PROVISION="bash scripts/agentx-hook-provision.sh"`
  - Example: `AGENTX_HOOK_START="safe --timeout 10 -- bash scripts/python-lint-type-test.sh && safe --timeout 10 -- bash scripts/rust-fmt.sh"`
- Configuration (env):
  - `AGENTX_TICKETS_DIR=/workspaces/rust-viterbo/.persist/agentx/tickets`
  - `AGENTX_WORKTREES_DIR=/workspaces/rust-viterbo/.persist/agentx/worktrees`
  - `LOCAL_TICKET_FOLDER=./shared/tickets`
  - `AGENTX_RUN_TIMEOUT=36000` (10h default for turns; set `0` to disable explicitly).
- Agent checklist:
  - Read `shared/tickets/<slug>/meta.yml` (status, owner, deps).
  - Read `shared/tickets/<slug>/body.md` once for context.
  - Tail recent messages: `ls shared/tickets/<slug>/ | sort | tail -n 10`. Do not edit existing message files.
- Final message:
  - Explain what changed, how to validate (exact commands), and what’s next. `agentx` captures it as `tNN-final.md`.
- Conventions:
  - Commands accept a slug only; do not pass file paths or session ids.
  - Order is defined by UTC timestamp prefixes; ignore mtimes.
  - `meta.yml` is authoritative and intentionally small.
- Merge helper policy:
  - Requires clean target worktree and uses `git merge --ff-only`. If fast‑forward is not possible (or a custom flow is needed), run manual `git` commands instead of the wrapper.

## API Policy (Internal Only)
- No stable public API; all Rust modules are internal.
- Prefer clearer, better APIs over compatibility; breaking changes are fine if they improve quality or align with thesis/specs.
- Avoid legacy shims unless a ticket asks for a staged transition.
- Use `viterbo::api` and `viterbo::prelude` for curated internal imports.
- Treat PyO3 as internal unless a ticket declares external guarantees.
