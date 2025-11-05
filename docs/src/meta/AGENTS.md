# AGENTS.md — rapid onboarding

**Mission:** push tickets → thesis → code/tests. No silent fixes. If confused, fix the spec or ticket first.

## Repo map
- `crates/viterbo`: math and core algorithms (Rust lib). Use `nalgebra` for fixed-size geometry.
- `crates/cli`: orchestration (Rust bin). Reads/writes `data/` and `docs/assets/`.
- `crates/safe-exec`: optional runner; use `scripts/safe.sh` instead when possible.
- `docs/`: mdBook site (thesis + meta). Publish small tables/figures to `docs/assets/`.
- `data/`: heavy outputs; gitignored; add a `provenance.json` per run.
- `scripts/`: devops (CI, safe run, VK integration, checks).

## Cross-references
- In code doc comments:
  - `TH: <anchor>` → thesis heading anchor in `docs/src/thesis/*.md`.
  - `VK: <uuid>` → Vibe Kanban ticket id.
- In thesis markdown:
  - `<!-- VK: <uuid> -->`, `<!-- Code: <path>::<symbol> -->`.

Run `bash scripts/check_refs.sh` before closing a ticket.

## Daily conventions
- Default to immutable FP style. Isolate imperative kernels when needed. Document invariants.
- Tests use `StdRng::seed_from_u64(SEED)`; record SEED on failures.
- Outputs:
  - heavy → `data/...`
  - publishable small → `docs/assets/...`
- Commands that may run long **must** go through `scripts/safe.sh`.
- Manual CI: `bash scripts/ci.sh`.
- No pre-commit hooks. Commit often. Reference `VK` in the message.

## Escalate when
- Solver/library choice blocks you.
- Runtime exceeds budget or needs GPU/CUDA setup.
- Lemmas/specs are underspecified.
- You are refactoring across crates or changing public APIs.

## Libraries (current stance)
- Geometry: `nalgebra` in `viterbo`.
- Data IO/wrangling: `polars`.
- Optimization: pick per-need later (`good_lp`, `clarabel`, `osqp`, `argmin`).
- Graphs: pick later (`petgraph` likely).
- RNG: `rand` (StdRng). Property tests optional.
- Bench: `criterion`.

Keep choices standard and common. Update this page if you change any default.
