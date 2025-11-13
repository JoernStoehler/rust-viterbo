/- 
File: lean/lakefile.lean
Purpose: Lake config for the Lean workspace that captures formal specs for the Viterbo project.
Docs: docs/src/meta/lean-onboarding.md, AGENTS.md#Platform and Tooling
-/
import Lake
open Lake DSL

package «symp» where
  -- Keep Lean builds reproducible and aligned with the repo cache layout.
  preferReleaseBuild := false

require mathlib from git
  "https://github.com/leanprover-community/mathlib4" @ "v4.9.0"

@[default_target]
lean_lib SympPolytopes where
  -- Each Lean lib mirrors a Rust/Python surface area to keep specs close to code.
  roots := #[`SympPolytopes]

lean_lib SympCertificates where
  roots := #[`SympCertificates]

/-- Executable used by `scripts/lean-test.sh` for fast smoke tests. -/
lean_exe SympLeanTests where
  root := `Tests.Main
