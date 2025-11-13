/-
File: SympPolytopes/OrientedEdge.lean
Purpose: Skeleton for formalizing the oriented-edge search algorithm with simple invariants.
Docs: docs/src/thesis/oriented-edge-charts-and-rotation.md, docs/src/meta/lean-onboarding.md
-/
import Mathlib.Data.Real.Basic
import SympPolytopes.Basic

namespace SympPolytopes

/-- Minimal stub for a symplectic oriented edge. Refine as we encode CZ computations. -/
structure OrientedEdge where
  ambientDim : Nat
  faceIds : Finset ℕ
  direction : Fin ambientDim → ℚ

namespace OrientedEdge

def isTrivial (e : OrientedEdge) : Bool :=
  e.faceIds.card ≤ 1

@[simp] theorem isTrivial_of_empty {d : Nat} :
    (OrientedEdge.mk d ∅ (fun _ => 0)).isTrivial = true := by
  simp [OrientedEdge.isTrivial]

end OrientedEdge

end SympPolytopes
