# 4D Convex Polytopes — Design Notes {#geom4d}

<!-- Ticket: 372a-create-libraries -->
<!-- Code: crates/viterbo/src/poly4.rs::Poly4 -->

Goal: clear, explicit 4D operations with both H/V representations and simple, reliable conversions. Scale is moderate (≈1e6), so naive enumeration is acceptable and keeps dependencies light.

## Representations
- H‑rep: half‑spaces `n·x <= c`, `n ∈ R^4`.
- V‑rep: vertices `x_i ∈ R^4`.
- `Poly4` stores both; each side may be empty and is filled on demand (`ensure_*`).

## Conversions
- H→V: enumerate all 4‑tuples of inequalities, solve the equalities, keep feasible points. Deduplicate by metric tolerance.
- V→H: enumerate 4‑tuples of vertices, form the unique hyperplane through them using a 3×4 cofactor‑based nullspace (no SVD). Keep only supporting planes (all vertices on one side); orient as `n·x <= c`.

These are O(N^4) but used infrequently; they keep the implementation compact and transparent.

## Faces
- Derive from H‑rep via vertex saturation:
  - 3‑faces (facets): vertices saturating a single inequality.
  - 2‑faces: vertices saturating a pair.
  - 1‑faces (edges): vertices saturating a triple.
  - 0‑faces: the vertices themselves.
- Return simple structs with facet indices and the corresponding vertex list. For downstream geometry we often only need vertices; equalities are kept as indices for traceability.

## Symplectic Helpers
- J‑matrix in 4D: `J = [[0, -I],[I, 0]]`.
- Symplectic check: `M^T J M ≈ J` (tolerance `1e-8`).
- Reeb flow on 3‑faces: `R_i = J n_i` (unnormalized). 1‑faces: stub until the derivation is written in the thesis.

## 2‑Face → 2D Mapping
- Given two facet indices `(i,j)`, the 2‑face is the plane orthogonal to `span{n_i,n_j}`.
- Build an orthonormal basis `(u1,u2)` of that plane via Gram–Schmidt in 4D; map `x ↦ y = Ux` with `U ∈ R^{2×4}` (inverse on the plane is `x = U^T y`).
- Orientation: we expose a boolean to pick the sign; a precise orientation convention (e.g., `(u1,u2,n_i,n_j)` positively oriented) can be fixed later if needed by algorithms. See “Open Questions”.
- Project the face’s vertices and construct a 2D polytope in H‑rep via `Poly2::from_points_convex_hull`, giving a faithful 2D model of the face.

## Affine Maps
- Push‑forward (H & V): algebraic transform on H‑rep and direct transform on vertices, requiring `M` invertible.
- Inversion: `(M,t) ↦ (M^{-1}, -M^{-1}t)`.

## Open Questions / Escalations
- Orientation of 2‑faces “as crossed by the Reeb flow”: likely induced by the ambient symplectic 2‑form; we left a design hook (sign boolean) and will add a proof‑based convention on request.
- Reeb flow on 1‑faces: placeholder stub until the derivation is added.

## Conventions
- Tolerance `eps = 1e-9` for feasibility/equality; `1e-8` for symplectic check.
- Keep code explicit and small; use math comments to tie back to this page.
- Tests: smoke tests for cubes/simplex; property tests optional.

