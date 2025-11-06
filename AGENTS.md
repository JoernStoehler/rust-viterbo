# AGENTS.md — rapid onboarding

This file is the always‑relevant guide for coding agents on this project — an MSc thesis in pure mathematics, using Rust to implement algorithms and data‑science artifacts. Keep this document short, unambiguous, specific, actionable, and low overhead.

## Active Temporary Notices
- None

## Repo map
- `crates/viterbo`: math and core algorithms (Rust lib). Use `nalgebra` for fixed-size geometry.
- `crates/cli`: orchestration (Rust bin). Reads/writes `data/` and `docs/assets/`.
- `scripts/safe.sh`: safe runner for long commands (group kill + timeout).
- `docs/`: mdBook site (thesis + meta). Publish small tables/figures to `docs/assets/`.
- `data/`: heavy outputs; gitignored; add a `provenance.json` per run.
- `scripts/`: devops (CI, safe run, VK integration, checks).

## Meta Documentation
- Self‑documenting project: agents must record project‑specific conventions, workflows, and reminders that fix encountered mistakes. Update docs as you work.
- What goes where:
  - Always relevant: this file (AGENTS.md at workspace root). Keep it lean and maintained.
  - Long‑term but only sometimes relevant: `docs/src/meta/*` (book “Meta”). Use speaking filenames grouped by “when needed”. `overview.md` is a thin index with one‑line “when to read” hints.
  - Temporary but universal: add a dated note to “Active Notices” at the top of this file; prune when stale.
- Do NOT write tutorials for common tools/libraries/conventions/workflows. Assume agents know them, just like you know them; record only project‑specific choices or reminders after observed mistakes.

## Source of truth and cross‑references
- We use Vibe Kanban (VK) to provision new agents with their own git worktree in which to work on an assigned ticket.
- We treat the tickets as the source of truth for our project.
- Closely derived and interacting with the tickets are the thesis specs in `docs/src/thesis/`. They define algorithms, architectures, data formats, etc. from the relevant high-level mathematical / project goal perspective. They aggregate different tickets into coherent wholes.
- Finally, the code, comments, tests, and data artifacts implement the specs and tickets.
- Our flow is thus: tickets => thesis => code/tests.
- When during work on the code/tests something is unclear or wrong, we escalate back to the ticket and fix it first. Agents are cheap to run, so we rather abandon an agent's attempt and start a new attempt after fixing the ticket or thesis spec to not repeat the same mistake or encounter the same blocker.
- If anything is unclear or unknown, stop and ask the project owner.
- To connect the three layers (code/tests, thesis, tickets), use these conventions in code comments or in markdown comments `<!-- ... -->`:
  - `Docs: docs/src/thesis/<path>#<anchor>` → the related thesis section(s).
  - `Ticket: <uuid>` → the related Vibe Kanban ticket uuid.
  - `Code: <path>::<symbol>` → the related code symbol.

## Daily conventions
- Default to immutable FP style. Isolate imperative kernels when needed. Document invariants.
- Tests use `StdRng::seed_from_u64(SEED)`; record SEED on failures.
- Outputs:
  - heavy → `data/...`
  - publishable small → `docs/assets/...`
  - provenance → every artifact gets `<artifact_stem>.provenance.json`, generated via the CLI `provenance` helper. Sidecars already include the git commit and callsite; add only run-specific parameters (no VK/ticket IDs here).
- Paper downloads (text sources + PDFs) live under `data/downloads/`; check there first before curling the web. Use `bash scripts/paper-download.sh --match "..."` for a single entry or `--all` to sync everything in `docs/src/thesis/bibliography.md`. The script fetches arXiv sources (with PDFs as fallback), writes manifest metadata, and calls `cli provenance` for each artifact automatically.
- Commands that may run long **must** go through `scripts/safe.sh`.
  - Example: `bash scripts/safe.sh --timeout 60 -- cargo test --workspace`
- Manual CI: `bash scripts/ci.sh`. No GitHub Actions.
- No pre-commit hooks. Commit often. Reference `Ticket: <uuid>` in the message.
- Docs (GitHub README): use GitHub KaTeX–safe math. Avoid macros like `\operatorname`; prefer `\mathrm{...}` or built-in operators. Verify via GitHub preview if unsure.

## Libraries and tech stack
- Geometry: `nalgebra` in `viterbo`.
- Data IO/wrangling: `polars` (required; no feature-gating).
- Optimization: pick per-need later (`good_lp`, `clarabel`, `osqp`, `argmin`).
- Graphs: pick later (`petgraph` likely).
- RNG: `rand` (StdRng). Property tests optional.
- Bench: `criterion`.

## Escalation
- When to escalate:
  - Lemmas/specs are underspecified.
  - Solver/library choice blocks you.
  - Runtime exceeds budget or needs GPU/CUDA setup.
  - You discover a blocker that is too large to fix within the ticket scope, so you need a new sub-ticket to be opened and completed before proceeding.
- How to escalate: stop your turn and notify the project owner with a concise summary and options (Ticket comment or direct channel).

## General Principles
- Keep decisions and choices standard and common. We want to be intuitive, predictable, and low-overhead for future agents.
- Update the meta documentation as you work, so that the project remains self-documenting.
- Escalate to the project owner when in doubt or blocked. Ask for forgiveness rather than permission, since it's easy to roll back changes or restart a ticket after improving the specs.
- Keep AGENTS.md lean for fast onboarding. Move info that is relevant only for some tickets into `docs/src/meta/` with clear “when to read” hints.
- Assume agents know common tools/libraries/conventions/workflows. Record only project-specific choices or reminders after observed mistakes.
- Write in a clear, unambiguous, specific, actionable, and low-overhead style.
