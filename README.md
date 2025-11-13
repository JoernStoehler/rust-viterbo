<!-- Purpose: External-facing entry point for reviewers and researchers. Keep it concise, explain what the thesis is, how to reproduce it end-to-end, and how to extend it. Internal process details belong in AGENTS.md or docs/src/meta/. -->
# Probing Viterbo’s Conjecture — MSc Thesis

[![Thesis](https://img.shields.io/badge/Thesis-Online-2ea44f)](https://JoernStoehler.github.io/rust-viterbo/)

This repository hosts the MSc thesis text and every artifact needed to verify its claims: Rust + Python code, data pipelines, Lean specs, benchmark outputs, and the rendered mdBook.

## What this thesis answers
- Maps where convex polytopes in $\mathbb{R}^4$ violate or approach Viterbo’s systolic bound.
- Automates Reeb-orbit search (oriented-edge graph method, LP certificates) to measure minimal actions.
- Provides a reproducible dataset and runtime-efficient kernels so others can explore variants of the conjecture.

## Repository overview
- `docs/`: mdBook sources for the thesis (built version linked above).
- `src/`, `crates/`: Python orchestration + Rust kernels that generate every figure/table.
- `data/`: Git LFS snapshots of published artifacts (regenerate via `scripts/reproduce.sh`).
- `lean/`: Lean4 workspace for in-progress formal specs of symplectic polytopes and oriented-edge reasoning.
- `configs/`: JSON configs for experiments; edit or add new configs to drive fresh studies.

## Reproduce the thesis end-to-end
1. **Pick an environment**
   - *GitHub Codespaces*: create a codespace on `main`, wait for “✅ Post-create setup completed successfully.”
   - *Local VS Code dev container*: `git clone https://github.com/<you>/rust-viterbo`, open in VS Code, “Reopen in Container,” wait for the same success message. Install Git LFS first or run `git lfs install --local` after cloning.
2. **Hydrate LFS artifacts** (if not already present): `git lfs pull --include "data/**" --exclude ""`.
3. **Run the single entrypoint**:
   ```bash
   bash scripts/reproduce.sh
   ```
   The script creates the Python venv, syncs deps, runs lint/tests (Python + Rust plus the Lean quick loops described below), rebuilds the native extension, executes the tiny atlas pipeline, generates benches, and builds the mdBook.
4. **Preview the thesis**: `mdbook serve docs -p 4000` and open the printed URL.

## Build new research on top
- Read `AGENTS.md` for contributor conventions (issue workflow, quick loops, provenance rules).
- Lean workspace: use the helper commands under `group-timeout`:
```bash
group-timeout 30 bash scripts/lean-setup.sh
group-timeout 60 bash scripts/lean-lint.sh
group-timeout 60 bash scripts/lean-test.sh
```
These commands also run automatically during container provisioning and when new worktrees are created; rerun them manually whenever you need to refresh the Lean cache.
- Python/Rust development mirrors the thesis structure: add configs under `configs/<experiment>/`, stages in `src/viterbo/<experiment>/`, and kernels in `crates/viterbo`. Run the standard loops (`scripts/python-lint-type-test.sh`, `scripts/rust-*.sh`) before opening a PR.
- All new data artifacts live in `data/<experiment>/...` with JSON provenance sidecars via `viterbo.provenance.write`.

## License & citation
- MIT License — see `LICENSE`.
- Please cite the project via the thesis link above; individual chapter references live in `docs/src/thesis/`.
