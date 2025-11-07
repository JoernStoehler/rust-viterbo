# 2D Convex Polytopes — Design Notes {#geom2d}

<!-- Ticket: 372a-create-libraries -->
<!-- Code: crates/viterbo/src/poly2.rs::Poly2 -->

Goal: extremely fast, robust‑enough 2D routines for half‑space polytopes used at scale (≈1e9 instances) in oriented‑edge DFS. We bias for simple data layout and branch‑light code. Degeneracy handling is intentionally minimal per ticket.

## Representation
- Two-tier 2D H-rep:
  - Loose (`HPoly2`): bag of half‑spaces `n·x <= c`; no guarantees on order, normalization, or redundancy. Fast to build/compose.
  - Strict (`HPoly2Ordered`): unit normals, angle‑sorted by `atan2`, and parallels coalesced (keep most restrictive `c`). Preserves invariants on insert/merge and after push‑forward.
- Keep vectors contiguous; strict form is cache‑friendly and enables adjacency‑based algorithms.

## Core Ops (hot path)
- Push‑forward under affine map `y = Mx + t` (M invertible): `A' = A M^{-1}`, `b' = b - A' t`. Pure algebra; no vertex construction.
  - Code: `HPoly2::push_forward`, `HPoly2Ordered::push_forward`, `Affine2`.
- Intersect with a half‑space / polytope: append (`HPoly2`) or ordered insert/merge (`HPoly2Ordered`).
- Membership: `n·x <= c + eps` for all rows.
- Emptiness:
  - Loose (generic heuristic): test pairwise boundary intersections + probes (`HPoly2::is_empty`).
  - Strict (exact via HPI): classical half‑plane intersection with deque; includes a quick contradictory‑pares check for opposite parallels (`HPoly2Ordered::hpi`).

## Less‑frequent Ops
- Extremal value of an affine functional `f(x)=w·x+a`: compute on discovered boundary vertices (same candidate set as emptiness). Returns `(min, argmin, max, argmax)` or `None` if no finite vertex is found.
- Affine utilities: inverse, composition, fixed point.
- CZ‑index related rotation of an orientation‑preserving map: stub (`cz_index_rotation_stub`) until specified in thesis math section.

## Interop and Helpers
- Build from 2D points by convex hull (Andrew’s monotone chain) → outward half‑spaces. Useful when projecting 4D faces to 2D.
  - Code: `Poly2::from_points_convex_hull`.

## Conventions
- Tolerance: `eps = 1e-9` for predicates; scale‑agnostic inputs preferred.
- Orientation: standard 2D orientation; outward normal constructed by 90° CCW rotation of hull edges.
- Style: small, explicit functions; explain the “why” in file headers; property tests optional (smoke tests suffice here).

## Rationale and Trade‑offs
- H‑rep keeps push‑forwards and intersections O(m) without hulls/vertices.
- Strict vs loose split makes invariants explicit: algorithms that rely on angle order and adjacency run in O(m) after a one‑time O(m log m) normalization; loose remains flexible for fast construction and composition.
- Degenerate cases (parallel strips, nearly co‑incident lines) are rare on hot paths; when needed, strict HPI resolves edge cases.
