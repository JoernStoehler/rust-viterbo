/-
File: SympPolytopes/Basic.lean
Purpose: Base definitions for convex polytopes and symplectic scaffolding in the Lean workspace.
Docs: docs/src/thesis/geom4d_polytopes.md, docs/src/meta/lean-onboarding.md
-/
import Mathlib.Init.Data.Rat.Basic
import Mathlib.Data.Finset.Card

namespace SympPolytopes

/-- Lightweight linear half-space defined over ℚ to keep proofs exact. -/
structure HalfSpace (n : Nat) where
  normal : Fin n → ℚ
  offset : ℚ

/-- Polytope encoded as a list of supporting half-spaces. -/
structure Polytope (n : Nat) where
  facets : List (HalfSpace n)

namespace Polytope

variable {n : Nat}

/-- Count the number of half-spaces that define `P`. -/
def facetCount (P : Polytope n) : Nat :=
  P.facets.length

/-- A convenience predicate for downstream algorithms requiring enough constraints. -/
def hasSufficientFacets (P : Polytope n) (k : Nat := n + 1) : Prop :=
  k ≤ P.facetCount

theorem hasSufficientFacets_mono {P : Polytope n} {k₁ k₂ : Nat}
    (hk : k₁ ≤ k₂) (h : P.hasSufficientFacets k₂) :
    P.hasSufficientFacets k₁ :=
  Nat.le_trans hk h

end Polytope

end SympPolytopes
