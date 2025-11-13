# Lean Workspace (`lean/`)

Purpose: centrally host the Lean4 specifications and experiments that back the Rust/Python implementations.

## Layout

- `lean-toolchain`: pins the exact Lean toolchain (managed by elan).
- `lakefile.lean` / `lake-manifest.json`: declares dependencies (`mathlib4`) and the logical libraries:
  - `SympPolytopes`: specs for convex polytopes, combinatorics, and symplectic structures (`SympPolytopes/` plus the root file `SympPolytopes.lean`).
  - `SympCertificates`: proof artifacts and (future) certificate export helpers (`SympCertificates/` plus the root file `SympCertificates.lean`).
- `Tests/`: entrypoints consumed by `scripts/lean-test.sh` for quick checks.
- `examples/`: lightweight, tutorial-style files new agents can open in VS Code to learn the conventions.

## Commands

All commands run from the repo root and should be wrapped in `group-timeout` just like the existing scripts.

```bash
group-timeout 30 bash scripts/lean-setup.sh     # ensure elan + deps
group-timeout 60 bash scripts/lean-lint.sh      # format + lint + type check
group-timeout 60 bash scripts/lean-test.sh      # build + run smoke tests
```

## Workflow

1. Install elan + lake (handled by `.devcontainer/postCreate.sh` once we rebuild the container).
2. Run `scripts/lean-setup.sh` to hydrate mathlib caches under `.persist`.
3. Edit files under `lean/SympPolytopes` or `lean/SympCertificates` and rely on the VS Code Lean extension for feedback.
4. Keep proofs cross-referenced with thesis chapters via `Docs:` comments at the top of each file.
5. Surface any reusable certificate formats under `SympCertificates` and document the pipeline before connecting it to Rust.
