/-
File: Tests/Main.lean
Purpose: IO entrypoint for quick Lean regression checks hooked to scripts/lean-test.sh.
Docs: docs/src/meta/lean-onboarding.md
-/
import SympPolytopes.OrientedEdge
import SympCertificates.Basic

open SympPolytopes

def runOrientedEdgeSmoke : IO Unit := do
  let edge : OrientedEdge :=
    { ambientDim := 4
      faceIds := {}
      direction := fun _ => 0 }
  if edge.isTrivial then
    IO.println "oriented edge smoke check passed"
  else
    throw <| IO.userError "expected trivial edge"

def runCertificateSmoke : IO Unit := do
  let cert := SympCertificates.mkPolygonCertificate 4 6
  if cert.facetLowerBound < 4 then
    throw <| IO.userError "bogus certificate lower bound"
  else
    IO.println s!"certificate: {cert.name}"

def main : IO Unit := do
  runOrientedEdgeSmoke
  runCertificateSmoke
