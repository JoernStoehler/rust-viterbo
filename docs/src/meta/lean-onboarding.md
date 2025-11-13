# Lean Onboarding (situational)

- **When to read:** whenever you touch `lean/`, need to run Lean quick loops, or investigate proofs that back the Rust/Python code.
- **What you get:** environment expectations, command references, and cross-links back to the thesis/specs.

## Goals

1. Keep formal specifications close to the production code while allowing slow, highly-trusted derivations.
2. Provide a predictable workflow for agents who are new to Lean but comfortable with the rest of the stack.
3. Document how the Lean workspace participates in the reproducibility story (repro script, CI, provenance).

## Environment

- Lean toolchain pinned via `lean/lean-toolchain` (currently `leanprover/lean4:v4.9.0`).
- Dependencies resolved with Lake; mathlib is vendored via `lean/lake-manifest.json`.
- `elan` and `lake` must be available in the devcontainer; once `.devcontainer/postCreate.sh` installs them you simply rebuild the container to pick them up.
- Cache directories live under `.persist` (same pattern as Rust/Python) to avoid re-downloading olean artifacts.

## Commands

All commands run from the repo root and **must** be wrapped in `group-timeout`.

| Command | Purpose |
| --- | --- |
| `group-timeout 30 bash scripts/lean-setup.sh` | Ensures elan toolchain + Lake deps are ready; safe to run repeatedly. |
| `group-timeout 60 bash scripts/lean-lint.sh` | Formats, lints, and builds `SympPolytopes`/`SympCertificates`. |
| `group-timeout 60 bash scripts/lean-test.sh` | Builds and runs `SympLeanTests` (smoke coverage for specs). |

These setup commands also run automatically during container provisioning and new worktree creation; rerun them manually whenever you change the toolchain pin or suspect stale caches.
## Directory Map

- `lean/SympPolytopes`: canonical home for specs that describe the math and algorithms (e.g., oriented edge search). Keep file-level headers in sync with the thesis pages they refine.
- `lean/SympCertificates`: scaffolding for turning proofs into data artifacts Rust can consume (JSON/protobuf to be defined per issue).
- `lean/Tests`: IO entrypoints referenced by the scripts above (quick to run, suitable for CI/repro).
- `lean/examples`: scratch/tutorial files for onboarding; remove or graduate them as they mature.

## Contribution Rules

1. Reference issues and thesis chapters using the same `Docs:` convention we use elsewhere.
2. When adding new mathlib dependencies, run `group-timeout 30 lake update` so `lake-manifest.json` stays current.
3. Symplectic/polytopal axioms should be isolated in clearly named modules to keep downstream proofs stable even if we swap representations later.
4. If you export certificates or proof objects, document the schema inside the thesis chapter that consumes them and link back from the Lean file header.
