<!-- Author: Codex -->

<!-- Ticket: Please add the VK ticket UUID here once available. -->

<!-- Docs: docs/src/thesis/Ekeland-Hofer-Zehnder-Capacity.md -->
<!-- Docs: docs/src/thesis/viterbo-conjecture-counterexample.md -->
<!-- Docs: docs/src/thesis/bibliography.md -->

# Minkowski Billiard Algorithm for c_EHZ on Lagrangian Products

<!-- Why: We want an implementation-ready, correctness-first specification of the Minkowski billiard algorithm for convex Lagrangian products K×T \subset (\mathbb{R}^4, \omega_0). The text mirrors the oriented-edge specification: progressive disclosure, precise invariants, explicit trade-offs, and actionable pseudocode. -->

## Goal
- Compute the Ekeland–Hofer–Zehnder (EHZ) capacity of the 4D convex, star-shaped, non-degenerate polytope \(P = K \times T\) where \(K,T \subset \mathbb{R}^2\) are convex bodies containing the origin, by minimizing the length of closed \((K,T)\)-Minkowski billiard trajectories.
- Produce a minimizing trajectory certificate (bounce sequence with coordinates and momenta) that can be cross-checked against the general oriented-edge Reeb search.
- Exploit the product structure to cut the search to 2D primitives, yielding a simpler implementation and numerically better conditioning than the 4D oriented-edge graph when it applies.

## Reader Roadmap
1. **Short on time?** Read [Algorithm Summary](#algorithm-summary) and [Implementation Plan](#implementation-plan) only.
2. **Need the math?** Read [Setting and Guarantees](#setting-and-guarantees) and [Reflection Law](#reflection-law) before jumping to the summary.
3. **Implementer?** Read everything, paying extra attention to [Geometry Preprocessing](#geometry-preprocessing), [Trajectory Families](#trajectory-families), and [Pseudocode](#pseudocode).

## Setting and Guarantees
1. **Ambient data.** Work in \(\mathbb{R}^4 = \mathbb{R}^2_q \times \mathbb{R}^2_p\) with the standard symplectic form \(\omega_0 = dq_1 \wedge dp_1 + dq_2 \wedge dp_2\) and Liouville form \(\lambda_0 = \tfrac{1}{2} \sum_i (p_i\,dq_i - q_i\,dp_i)\).
2. **Lagrangian product.** Input polytopes \(K,T \subset \mathbb{R}^2\) are convex, star-shaped, contain 0, and are given both as strict H-representations (`HPoly2Ordered`) and V-representations with cyclic vertex order (`VPoly2`). The product \(P = K \times T\) inherits these properties and is our 4D polytope.
3. **Reeb dynamics.** On \(\partial P\), the Reeb vector field splits: the \(q\)-motion stays in \(\partial K\) while the conjugate momentum lives on \(\partial T\).
4. **Minkowski billiards.** Rudolf (2022/24) shows that \(c_{EHZ}(K \times T)\) equals the minimal \(T\)-Minkowski length of a closed \((K,T)\)-billiard, i.e. a polygonal path \(q_0,\ldots,q_{m-1}\) on \(\partial K\) obeying the reflection law for the norm with unit ball \(T\).
5. **Bounce bound.** In \(\mathbb{R}^2\), every minimizing \((K,T)\)-billiard has either 2 or 3 bounces (Bezdek–Lángi, Macbeath type extension). Degenerate support lines can be handled by a limiting argument; we detect them numerically and fall back to the general oriented-edge search if the certificate is ambiguous.

## Reflection Law
1. Define the gauge \(g_T(v) = \inf\{ \lambda > 0 : v \in \lambda T\}\) and the polar body \(T^{\circ} = \{ y : x\cdot y \le 1 \text{ for all } x \in T \}\). The gauge equals the support function of the polar: \(g_T(v) = h_{T^{\circ}}(v)\).
2. A polygon \(q_0,\ldots,q_{m-1}\) on \(\partial K\) with edges \(v_i = q_{i+1}-q_i\) (indices modulo m) is a \((K,T)\)-Minkowski billiard iff there exist momenta \(p_i \in \partial T\) and scalars \(\lambda_i > 0\) such that
   \[
   p_{i+1} - p_i = \lambda_i n_K(q_i), \quad v_i = \nabla h_{T^{\circ}}(p_i) = \nabla g_T^*(p_i),
   \]
   where \(n_K(q_i)\) is the outward unit normal of \(K\) at \(q_i\).
3. The EHZ action equals the \(T\)-Minkowski length:
   \[
     A(q_0,\ldots,q_{m-1}) = \sum_{i=0}^{m-1} g_T(v_i) = \sum_{i=0}^{m-1} h_{T^{\circ}}(q_{i+1}-q_i).
   \]

## Algorithm Summary
1. **Precompute** the strict facet lists of \(K\) and \(T^{\circ}\), support/gauge lookup tables, and polar adjacencies (Section [Geometry Preprocessing](#geometry-preprocessing)).
2. **Enumerate 2-bounce candidates.** For every direction \(u\) represented by a facet normal of \(K\) or \(T^{\circ}\), solve a parallel support pair problem to obtain antipodal points \(q_0,q_1\). Their action is \(2\,g_T(q_1-q_0)\).
3. **Enumerate 3-bounce candidates.** For every cyclic triple of facets of \(K\) and compatible triple of edges of \(T^{\circ}\), solve the discrete reflection system to recover \((q_i,p_i)\) and evaluate the action.
4. **Validate** each candidate (positivity of \(\lambda_i\), all points inside facets, total winding one) and keep the minimal action.
5. **Return** the best action and its certificate. If no valid candidate survives numerical tolerances, fall back to the oriented-edge graph search for this instance and file a ticket.

## Geometry Preprocessing
1. **Facet normalization.** Convert each supporting line of \(K\) to \((n_i, c_i)\) with \(\|n_i\|=1\) and \(n_i \cdot x \le c_i\). Ensure cyclic ordering (counter-clockwise) as described in `docs/src/thesis/geom2d_polytopes.md`.
2. **Dual body.** Build \(T^{\circ}\) explicitly. Because \(T\) is a polygon, \(T^{\circ}\) is also a polygon whose vertices are intersections of consecutive supporting lines of \(T\).
3. **Lookup tables.**
   - `support_K(u)` for every facet normal `u` and every normal needed from \(T^{\circ}\).
   - `g_T(v)` evaluated on edge direction templates (differences of vertices of \(K\)).
   - Gradient map samples: for each vertex \(p\) of \(T\), store the adjacent edge directions to parameterize motion along \(\partial T\).
4. **Tolerances.** Fix two epsilons: `eps_normal` for parallel checks and `eps_action` for comparing actions. Propagate them into validation logic; document them in code comments referencing this doc.

## Trajectory Families
### Two-bounce (diameter) branch
1. Choose a direction \(u \in \mathbb{S}^1\).
2. Compute the supporting points \(q_0 \in H_K(u)\) and \(q_1 \in H_K(-u)\), where \(H_K(u) = \{x: n\cdot x = h_K(u)\}\).
3. The candidate is valid if these lines intersect \(\partial K\) in segments parallel to \(u\) (typical when \(u\) equals a facet normal) and if the Minkowski diameters match: \(g_T(q_1-q_0) = g_T(q_0 - q_1)\).
4. The action is \(A = 2 g_T(q_1-q_0)\). Store \((q_0,q_1, u)\) and the implied momentum pair \((p_0,p_1) = (p,-p)\) where \(p \in \partial T\) maximizes \(p \cdot (q_1-q_0)\).

### Three-bounce (Fagnano-type) branch
1. Choose an ordered triple of facets \((F_a,F_b,F_c)\) of \(K\) with outward normals \((n_a,n_b,n_c)\) arranged counter-clockwise. Skip triples with nearly parallel adjacent normals (degenerate triangle) by checking \(|n_i \times n_{i+1}| > eps_{\text{normal}}\).
2. Choose an ordered triple of momenta edges \((E_a,E_b,E_c)\) on \(\partial T\) (or equivalently vertices of \(T^{\circ}\)) such that walking along the dual edges follows the same orientation. Each edge supplies a unit tangent \(t_i\) and its associated gradient direction \(v_i = \nabla h_{T^{\circ}}(p_i)\).
3. Solve the linear system for \(q_a,q_b,q_c\) given by the supporting lines `n_i · q_i = c_i` and the closure condition \(v_a + v_b + v_c = 0\) (polygon closes). Because each \(v_i\) lies in the cone generated by adjacent normals, the system reduces to a 2×2 solve.
4. Recover \(\lambda_i\) by projecting \(p_{i+1}-p_i\) onto \(n_i\). Reject candidates with \(\lambda_i \le 0\).
5. Action equals \(A = g_T(v_a) + g_T(v_b) + g_T(v_c)\). Store \((q_i, p_i, \lambda_i)\) for validation.

### Fallback branch
If neither branch yields a certificate (rare), call the oriented-edge algorithm on \(P\) and log the failure so we can debug the preconditions. This guarantees completeness at the cost of performance.

## Numerical Solvers
1. **Support intersections.** Implement `intersect_support(K, u)` that returns the point on \(\partial K\) where \(n\cdot x = h_K(u)\) along the averaged adjacent vertex indices. Use the ordered vertex list so the result is \(O(1)\).
2. **Triangle solve.** For three facets, we intersect adjacent supporting lines to get vertices, then solve a 2×2 linear system for barycentric weights ensuring closure: find scalars \(\alpha_i>0\) with \(\sum \alpha_i = 1\) and \(\sum \alpha_i v_i = 0\). Implementation detail: express \(v_i\) in the basis \((e_x,e_y)\) and solve using `nalgebra::Matrix2::lu().solve()`.
3. **Gauge evaluation.** Because \(g_T\) is polyhedral, evaluating \(g_T(v)\) equals computing \(\max_j n_j^{T^{\circ}} \cdot v\). Precompute the normals matrix to keep this \(O(m_T)\) with good cache behavior.
4. **Robustness.** Clamp negative \(\lambda_i\) produced by floating noise to zero and treat them as invalid. Keep diagnostics so we can reproduce failing cases.

## Certificates and Verification
1. Return `(action, trajectory)` where `trajectory` stores the bounce points, momenta, action contributions, and \(\lambda_i\).
2. Verify certificate before returning:
   - Each \(q_i\) satisfies its supporting equation within `eps_geom`.
   - The polygon closes: \(\|\sum v_i\| < eps_geom\).
   - Rotation number equals one (check signed area of the polygon).
   - The action recomputed via the Liouville integral on \(P\) matches \(A\) within `eps_action`.
3. Optionally cross-check by feeding the certificate into the oriented-edge engine restricted to the faces of \(P\) that contain it; mismatches indicate bugs.

## Complexity
- Let \(m_K\) and \(m_T\) denote the number of facets/vertices in \(K\) and \(T\). Two-bounce enumeration costs \(O(m_K + m_T)\). Three-bounce enumeration costs \(O(m_K^3 + m_T^3)\), but we prune aggressively: only triples whose normals interleave (alternating acute angles) are tried, reducing practice to \(O(m_K m_T)\).
- Memory footprint is \(O(m_K + m_T)\) besides the certificate.

## Implementation Plan
1. **Rust module.** Add `crates/viterbo/src/billiard/mod.rs` with three submodules:
   - `prelude.rs`: shared types (`BilliardTrajectory`, tolerances, gauges`).
   - `two_bounce.rs`: deterministic scan over normals.
   - `three_bounce.rs`: triple enumeration with solvers and pruning hooks.
2. **Python glue.** In `src/viterbo/rust/billiard.py`, expose a `compute_billiard_capacity(K, T)` helper returning action + certificate, using PyO3 bindings.
3. **Config plumbing.** Extend experiment configs so stages can request the billiard solver when `polytope.structure == "lagrangian_product"`.
4. **Fallback hook.** Keep the oriented-edge algorithm wired as `fallback_orbit` for verification and degenerate cases.

## Pseudocode
```rust
pub fn minkowski_billiard_capacity(k: &StrictPoly2, t: &StrictPoly2) -> Result<Candidate> {
    let prep = Geometry::new(k, t)?;
    let mut best = Candidate::infinite();

    for dir in prep.iter_support_directions() {
        if let Some(cand) = two_bounce::solve(&prep, dir) {
            best.minimize(cand);
        }
    }

    for triple in prep.iter_interleaved_triples() {
        if let Some(cand) = three_bounce::solve(&prep, triple) {
            best.minimize(cand);
        }
    }

    best.validate(&prep)?;
    if best.is_finite() {
        Ok(best)
    } else {
        oriented_edge::fallback(k, t)
    }
}
```

## Validation Strategy
1. **Analytic checks.** Compare against known capacities:
   - \(K=T=B_2\): expect \(c_{EHZ}=\pi\).
   - Rectangles \([ -a,a ] \times [ -b,b ]\) with dual rectangles: action equals \(4 \min(a,b)\).
   - The polytopes used in `docs/src/thesis/viterbo-conjecture-counterexample.md`.
2. **Cross-validation.** For random rational polytopes up to 12 facets, compare with the oriented-edge result. All mismatches must be explained (degeneracy vs bug).
3. **Property tests.** Perturb the input by a small homothety and ensure action scales linearly.
4. **E2E stage.** Add a smoke test in `tests/smoke/test_billiard.py` that exercises the Python wrapper on a tiny sample.

## Open Questions
1. Precise citation for the “2 or 3 bounce” theorem in the polyhedral case. The smooth proof extends, but we still need to document an explicit combinatorial argument.
2. Numerical stability when \(K\) or \(T\) is nearly smooth: do we need extended precision, or is 64-bit floating-point enough when combined with renormalization?
3. How aggressively should we prune facet triples? Interleaving normals is sufficient but not necessary; we may add a mixed-volume bound once we profile real data.
4. Should we cache the certificates for reuse across nearby parameter sweeps (important when scanning rotations)? This impacts memory budgeting in `scripts/reproduce.sh`.

## Clarifications (unstable, unsorted)
<!-- Purpose: park quick notes about code/spec divergences or open questions so agents can proceed without blocking on full edits. Treat entries as provisional; once stabilized, fold them into the main text and remove from the list. -->
- Cross-checking with oriented-edge: when a billiard certificate is ambiguous due to degeneracy (parallel supports with ties), we temporarily fall back to the oriented-edge solver and annotate the run; this keeps pipelines moving without committing to tie-breaking here.
- Units and normalization: verify the exact constant between Minkowski length and action used in code when both factors are polytopes; once fixed, add a short lemma here and remove this note.
