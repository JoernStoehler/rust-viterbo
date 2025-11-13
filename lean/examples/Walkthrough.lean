/-
File: examples/Walkthrough.lean
Purpose: Guided example for agents learning the Lean workspace conventions.
Docs: docs/src/meta/lean-onboarding.md
-/
import SympPolytopes.Basic

open SympPolytopes

def simplex4 : Polytope 4 :=
  { facets := [] }

#eval simplex4.facetCount
