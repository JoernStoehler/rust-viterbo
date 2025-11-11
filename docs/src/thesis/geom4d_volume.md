# 4D Volume Algorithm {#geom4d-volume}

<!-- Ticket: 2224b2c6-4a0c-468d-a7a1-493eb2ee5ddd -->
<!-- Code: crates/viterbo/src/geom4/volume.rs -->

Context — The experiments now rely on exact 4D volumes for convex, star-shaped polytopes that already ship with explicit H/V conversions. We compared the first two implementation options: (1) call an existing library such as Qhull/VolEsti via FFI, or (2) implement a bespoke algorithm. Borrowing a library would drag in new build dependencies, FFI wrappers, and conflicting tolerance policies, so we instead coded a self-contained facet-fan algorithm that matches the repository’s explicit enumeration style.

## Setting and Notation
- Ambient space: \(\mathbb{R}^4\) with polytopes given as `Poly4`, i.e. cached half-spaces `n_i \cdot x \le c_i` and optional vertex lists.
- Polytopes are convex, star-shaped, and non-degenerate; constraints come from generators that already enforce boundedness.
- `enumerate_faces_from_h` yields all 0/1/2/3-faces by tracking which inequalities are tight at each vertex; indices reference the original half-spaces.
- We may push forward polytopes under invertible affine maps, so the volume routine must be invariant under volume-preserving transformations.

## Definitions
1. **Facet fan**: for a 3-face \(F\) we form tetrahedra with vertices `(facet_centroid, triangle_on_F)` where the triangles tessellate \(\partial F\). Coning those tetrahedra with an interior polytope point produces 4-simplices.
2. **Interior anchor**: the centroid of all vertices returned by the H→V enumeration. Convexity guarantees that this barycenter lies in the interior.^[^Gru03]
3. **Ordered ridge polygon**: each 2-face inherits a cyclic vertex order by projecting onto the local 2D tangent basis obtained via Gram–Schmidt inside the 4D ambient space.^[^Zie95] When the intersection of two facets only yields a line segment (two colinear vertices) we classify it as a 1-face, drop it from the ridge list, and keep scanning for the higher-dimensional combinations that bound simplicial/symmetric polytopes (e.g., the 4D cross polytope).
4. **Volume decomposition**: the polytope is the disjoint union (up to measure-zero overlaps) of 4-simplices formed by `(interior_anchor, facet_centroid, triangle vertices)` across every triangulated ridge.

## Main Facts / Theorems
1. **Correctness of the fan decomposition**. The surface integral form of Gauss’ divergence theorem states \( \operatorname{Vol}(P) = \tfrac{1}{4} \sum_F h_F \operatorname{Vol}_3(F) \) for interior anchor \(0\).^[_^Zie95] By subdividing each \(F\) into tetrahedra that share a point \(p_F\) we rewrite that sum as \( \sum_{F,\triangle\subset F} \operatorname{Vol}(\operatorname{conv}\{0,p_F,\triangle\})\). Translating by the actual centroid \(c_P\) preserves determinants, so summing 4-simplex determinants recovers the true volume without explicit facet areas.
2. **Robust ordering of ridge polygons**. Because each 2-face lies in a 2D affine plane, Gram–Schmidt on difference vectors returns an orthonormal basis of that plane. Projecting onto the basis, sorting by polar angle, and triangulating with a fan produces a manifold triangulation regardless of vertex order in the cache. The tolerance `FEAS_EPS = 1e-9` keeps numerics stable for the moderate coordinate ranges produced by the generators.
3. **Breadth-first reuse of enumerations**. The algorithm only needs the outputs of `enumerate_faces_from_h` (vertices + 2/3-faces). No convex-hull rebuild is necessary, so the asymptotic cost stays at \(O(H^4)\), matching the existing conversion routines.
4. **Affine invariance**. Each 4-simplex volume is `|det([v_1-c, v_2-c, v_3-c, v_4-c])|/24`, so composing `Poly4::push_forward` with a matrix of determinant 1 leaves every determinant unchanged. Unit tests assert invariance under shears with `det=1.0` and random translations.
5. **Failure modes surface early**. Degenerate 2-faces (fewer than three affinely independent vertices) or facets (<4 vertices or missing incident ridges) raise a `VolumeError`, feeding precise debug info back to experiment drivers before any silent mis-computation. Lower-dimensional facet intersections are filtered out before the ordering phase, so symmetric polytopes no longer trip over ridge bookkeeping—only genuinely collapsed faces bubble up.

## What We Use Later
- `viterbo::geom4::volume::{volume4, volume_from_halfspaces, VolumeError}` provide Rust callers with a fallible API that can be memoized alongside other `Poly4` data.
- PyO3 exposes `poly4_volume_from_halfspaces`, and `viterbo.rust.volume.volume_from_halfspaces` adds a typed Python helper; smoke tests cover the binding.
- Criterion benchmark `volume4_bench` samples bounded random polytopes of varying facet counts to watch for regressions in `scripts/rust-bench.sh`.
- Docs/tests reference hypercubes and simplices as canonical fixtures; invariance tests guard against accidental determinant scaling.

## Deviations and Notes for Review
- We clone 2-face vertex lists to keep the ordering logic simple. Should benchmarks show pressure, revisit this by storing indices instead of full vectors.
- The enumeration routine currently recomputes vertices from H-reps for each call. If repeated volume queries dominate a workload, promote the vertex list to a shared cache or accept vertices/V-rep as inputs to skip the extra pass.

[^Zie95]: Ziegler (1995), *Lectures on Polytopes*, Springer. Chapter 5 covers face lattices and volume formulas.
[^Gru03]: Grünbaum (2003), *Convex Polytopes* (2nd ed.), Springer. Proposition 2.2.6 shows that convex combinations of all vertices lie in the interior.
