
<!-- Ticket: <slug> -->
<!-- Why: Specify an implementer‑ready LP/QP pathway for computing/estimating c_EHZ of 4D convex polytopes from facet data. Complements the oriented‑edge graph search and the Minkowski billiard chapter. -->
<!-- Editing notes: Keep statements precise; equations KaTeX‑safe; footnotes author–year. Cross‑link to sibling chapters. -->

# LP/QP Programs for c_EHZ on Convex Polytopes in R^4

Purpose. This chapter turns the Haim–Kislev combinatorial formula for the Ekeland–Hofer–Zehnder capacity into optimization programs that an implementer can assemble from facet data. We give the exact nonconvex QP arising from the formula, practically useful convex relaxations (LP/SDP) that certify bounds, and an implementation and validation plan that integrates with the oriented‑edge and billiard solvers.

Scope. Convex, star‑shaped, non‑degenerate polytopes in (R^4, ω_0). Inputs are in H‑representation with outward unit facet normals and support numbers. See the background on c_EHZ and Reeb dynamics in Ekeland‑Hofer‑Zehnder‑Capacity.md and the oriented‑edge and billiard algorithm chapters:
- Docs: thesis/capacity-algorithm-oriented-edge-graph.md
- Docs: thesis/capacity-algorithm-minkowski-billiard.md

## Setting and Notation

- Fix the standard symplectic form ω(u,v) = u^T J v on R^4 with
  J = [[0, I_2], [-I_2, 0]].
- Let K ⊂ R^4 be a convex polytope with 0 ∈ int K and H‑representation
  K = { x ∈ R^4 : ⟨a_i, x⟩ ≤ h_i, i = 1,…,m },
  where each a_i ∈ R^4 is the outward unit normal of facet i and h_i = h_K(a_i) > 0 is the (oriented) support number.
- Stack A ∈ R^{m×4} with rows a_i^T, h ∈ R^m with entries h_i. Define W := A J A^T ∈ R^{m×m} with entries W_{ij} = ω(a_i,a_j).

Remark (translation/scaling invariance). c_EHZ is translation invariant and 2‑homogeneous under scaling. If 0 ∉ int K, translate once to put 0 inside and recompute h_i; if K is scaled by s>0, then h_i ← s h_i and c_EHZ(sK) = s^2 c_EHZ(K).[^HK19]

## Literature: exact combinatorial program and “simple loop”

Haim–Kislev give a finite‑dimensional, implementable formula for polytopes; it rests on a “simple loop” property: there exists a minimal‑action generalized closed characteristic that visits the interior of each facet at most once (no facet revisits).[^HK19] In particular:

- Define the feasible weight polytope
  B_K := { β ∈ R_+^m : A^T β = 0, h^T β = 1 }.
- Then the EHZ capacity of K is
  c_EHZ(K) = 1 / (2· M_K),
  where
  M_K := max_{β ∈ B_K, σ ∈ S_m}  Σ_{1 ≤ j < i ≤ m} β_{σ(i)} β_{σ(j)} · ω(a_{σ(i)}, a_{σ(j)}).

This is Equation (1) in Haim–Kislev’s result, restated explicitly also in Leipold–Vallentin (their Eq. (1)) in the P(A,b) convention.[^HK19][^LV24] The permutation σ encodes the (single‑visit) order in which the minimizing loop traverses the facets. A reduction narrows σ to cycles of a directed graph built from facet adjacencies, which is useful algorithmically and connects to our oriented‑edge search.[^HK19, Remark 3.11]

Complexity. Computing c_EHZ of polytopes is NP‑hard (even for simplices) by reduction to a quadratic assignment structure hidden in M_K.[^LV24] Practical algorithms therefore combine exact solves on small/structured inputs with certified convex bounds and specialized search.

## Exact nonconvex QP (fixed order) and global mixed QP/QAP

We separate the permutation (order) and the nonnegative weights β.

### Program A — nonconvex QP for a fixed order

Inputs: A ∈ R^{m×4}, h ∈ R^m, W = A J A^T, an index order σ of a subset S ⊆ {1,…,m}. Entries with i ∉ S are allowed but will get β_i=0.

Decision variables:
- β ∈ R_+^m (nonnegative facet weights).

Constraints:
- A^T β = 0  (four linear equalities; closure)
- h^T β = 1  (normalization)
- β_i = 0 for i ∉ S (if restricting to a chosen subset/order).

Objective (maximize):
- Q(β;σ) := Σ_{1 ≤ j < i ≤ m} β_{σ(i)} β_{σ(j)} · W_{σ(i),σ(j)}.

Return c(σ) := 1/(2· Q*(σ)) with Q*(σ) the optimal objective value. The exact capacity is c_EHZ(K) = min_σ c(σ) when σ ranges over all orders allowed by the simple‑loop theorem; in practice, restrict σ as in the “Permutation pruning” note below.

Notes:
- This QP is in general indefinite (nonconvex) because W is skew‑symmetric and the lower‑triangular selection depends on σ. Modern global solvers (e.g., Gurobi/CPLEX/SCIP) accept such QPs and solve them via spatial branch‑and‑bound with McCormick relaxations and cuts. Provide and record a global optimality certificate (gap ≤ ε_gap).[^Krupp20]
- The four equalities A^T β = 0 enforce the vector equilibrium Σ β_i a_i = 0 which encodes closedness; h^T β = 1 pins the scale (action normalization).

### Program B — unified mixed QP/QAP (optional, small m)

Let P ∈ {0,1}^{m×m} be a permutation matrix (P e = e, P^T e = e) and define the strictly lower‑triangular mask L ∈ {0,1}^{m×m} with L_{ij}=1 for i>j and 0 otherwise. Then the objective can be written as
Q(β,P) = ⟨ L ∘ (P W P^T), β β^T ⟩,
where ∘ is the Hadamard product and ⟨·,·⟩ the Frobenius inner product. This yields a compact mixed 0–1 nonconvex quadratic program over (β,P). It is exact but only practical for very small m due to the QAP‑type combinatorics.[^LV24][^Krupp20]

Permutation pruning. Use Haim–Kislev’s reduction to cycles of a directed graph on facets (edge i→j present when the Reeb velocity can switch from facet i to j) to shrink σ to “combinatorially allowed” orders; this is exactly the graph our oriented‑edge chapter builds on (2‑faces drive feasible switches).[^HK19] In practice we enumerate simple cycles up to a cutoff length L (≤ m), set β_i=0 for i outside the cycle, and run Program A per cycle.

## Convex relaxations and bounds (LP/SDP)

Because M_K is a maximum of a bilinear form over a polytope, convex outer approximations in the lifted space give certified bounds. Let y_{ij} ≈ β_i β_j. The constraints A^T β=0, h^T β=1, β ≥ 0 define a compact polytope B_K with explicit upper bounds 0 ≤ β_i ≤ 1/ min(h_i,1e9) since h_i>0 and h^T β=1.

- LP (McCormick) relaxation — lower bound on c_EHZ:
  - For a fixed σ define y_{ij} for i>j and enforce McCormick envelopes over boxes [0,U_i]×[0,U_j]:
    y_{ij} ≥ 0; y_{ij} ≤ U_i β_j; y_{ij} ≤ U_j β_i; y_{ij} ≥ β_i + β_j − U_i − U_j,
    with U_i := 1/h_i.
  - Maximize Σ_{i>j} W_{σ(i),σ(j)} y_{σ(i)σ(j)} subject to β ∈ B_K and the envelopes.
  - Call the optimum \hat M_σ ≥ M_σ (outer relaxation). Then
    c_EHZ(K) ≥ 1/(2· max_σ \hat M_σ).
  - This is fast (HiGHS/CP‑SAT) and scales; it certifies a rigorous lower bound on c_EHZ.

- SDP (Shor/CP) relaxations — stronger lower bounds:
  - Lift to Y ≽ 0 with Y_{ij} ≈ β_i β_j, add linear side constraints Y e = β, diag(Y) ≤ U ∘ β, β ≥ 0, A^T β = 0, h^T β = 1, and maximize ⟨S_σ, Y⟩ with S_σ := L ∘ (P_σ W P_σ^T).
  - Replacing the completely positive cone by the PSD cone gives a tractable SDP; multiple SDP rounds over selected σ produce tight certified lower bounds.[^Krupp20]

Upper bounds on c_EHZ. Any feasible (β,σ) yields M ≤ M_K and thus c_EHZ(K) ≤ 1/(2M). Heuristics (local ascent for β on each σ, greedy σ from W’s positive entries, or rounding from relaxations) produce candidates; we then reconstruct a closed polygonal orbit and measure its action (next section), also cross‑checking with the oriented‑edge solver.

## Reconstructing a polygonal certificate from (β,σ)

Given a feasible (β,σ), define segment directions v_i := J a_{σ(i)} and positive times t_i := λ β_{σ(i)} for some λ>0. Choose λ so that Σ t_i h_{σ(i)} = 1 (normalization). The closure condition A^T β = 0 implies Σ t_i v_i = 0, so the concatenation gives a closed polygonal loop on ∂K with edges parallel to v_i. Its action equals
  A = 1 / (2· Σ_{j<i} β_{σ(i)} β_{σ(j)} ω(a_{σ(i)}, a_{σ(j)})) = 1/(2·Q(β;σ)),
matching the program’s value; this is the certificate we store (order, nonzero facets, times t_i, action).[^HK19][^Krupp20]

Numerical tolerances for the certificate:
- Closure residual: ||Σ t_i v_i||_2 ≤ τ_close (default 1e−10 · Σ t_i).
- Facet support consistency: |⟨a_{σ(i)}, x⟩ − h_{σ(i)}| ≤ τ_face when sampling a point x on each segment (diagnostic only).
- Action check: recompute polygonal action directly from vertices; relative mismatch ≤ 5e−9.

## Implementation Plan

Data plumbing.
- Accept H‑rep with unit normals: A ∈ R^{m×4}, h ∈ R^m, 0 ∈ int K. If normals are not unit, renormalize a_i ← a_i/||a_i||, h_i ← h_i·||a_i||.
- Build W = A J A^T once. Precompute facet graph used by the oriented‑edge chapter; reuse its cycle enumeration to prune σ.

Solvers.
- Exact/upper‑bound candidates: Program A with a global nonconvex QP solver (Gurobi/CPLEX/SCIP). Keep ε_gap ≤ 1e−6 and record solver’s optimality certificate (gap, best bound, node count, time).
- Lower‑bound certificates: LP McCormick (HiGHS) by default; optional SDP (MOSEK/SDPA) for tighter bounds on small m.
- Optional unified Program B for tiny m (≤ 14) to cross‑check reductions.

Assembly details.
- Rust: create `crates/viterbo/src/capacity/lpqp.rs` with:
  - `build_data(hrep: &HPoly4) -> (A: DMatrix<f64>, h: DVector<f64>, W: DMatrix<f64>)`
  - `enumerate_cycles(g: &FacetGraph, max_len: usize) -> impl Iterator<Item=Vec<usize>>`
  - `solve_qp_fixed_order(W, A, h, order) -> QpResult { beta, q_value, action, cert }`
  - `relax_lp_mccormick(...) -> LpBound { m_hat }`
- Python: thin wrappers in `src/viterbo/rust/` and a stage module `src/viterbo/capacity.stage_lpqp.py` to drive experiments and write provenance sidecars (`viterbo.provenance.write(...)`).
- Expose a unified `compute_capacity_lpqp(K)` that returns:
  - action estimate(s) with certificate(s),
  - lower bound (LP/SDP) and any exact matches,
  - solver logs (gap/time) for reproducibility.

Defaults and budgets (group-timeout wrapper).
- LP/graph enumeration: 10 s budget for m ≤ 80.
- Exact QP per order: 1–5 s; stop after best‑so‑far is within factor 1.02 of LP lower bound or after 120 s total.
- SDP (optional): 60–180 s small‑m runs only.

## Validation Plan

Against oriented‑edge algorithm (general polytopes).
- On each test polytope, run the oriented‑edge solver to get an action candidate A_oe. From LP/QP, collect:
  - an upper‑bound candidate A_up (from a feasible (β,σ)),
  - a lower bound A_low (from convex relaxation).
- Validate: A_low ≤ c_EHZ(K) ≤ min(A_oe, A_up) and, when oriented‑edge feasibility gives a simple loop, min(A_oe, A_up) agrees with A_low within tolerance.

Against Minkowski billiard (Lagrangian products).
- For K×T ⊂ R^4, run the billiard algorithm; the returned action must match the LP/QP result (best feasible candidate) within 1e−7 when both are exact.[^Rudolf24]

Sanity set.
- Balls/ellipsoids approximated by tight polytopes (convergence to π r^2).
- Centrally symmetric polytopes with symmetry pairs (use Haim–Kislev’s symmetric simplification).[^HK19]
- Small simplices (compare with values obtained in NP‑hardness constructions for cross‑checks).[^LV24]

## Guarantees, Tolerances, and Failure Modes

- Correctness (exact run). If Program A is solved globally for a σ that belongs to the reduced set covering all simple loops, the produced action equals c_EHZ(K). The certificate is the polygonal loop reconstructed above.
- Bounds. For any σ, LP/SDP relaxations certify c_EHZ(K) ≥ 1/(2·\hat M_σ). Any feasible (β,σ) yields c_EHZ(K) ≤ 1/(2·Q(β;σ)).
- Numerics. Use absolute tolerance 1e−9 on linear equalities, 1e−12 clipping for β ≥ 0, and a solver relative gap ≤ 1e−6 for declaring “exact”.
- Degeneracies. If some facets are nearly parallel or h_i very small, rescale K to unit inradius∈[0.5,2] before assembly; undo scaling on output. Fall back to oriented‑edge feasibility if β concentrates on <3 facets (degenerate paths).

## References (footnotes)

[^HK19]: Haim‑Kislev, P. On the Symplectic Size of Convex Polytopes. Geom. Funct. Anal. 29 (2019) 440–463; arXiv:1712.03494. Key results: simple‑loop theorem; combinatorial formula (their Eq. (1)); permutation reduction via a facet graph. Also see the arXiv version for a complete statement.

[^LV24]: Leipold, K., Vallentin, F. Computing the EHZ capacity is NP‑hard. Proc. Amer. Math. Soc. Ser. B 11 (2024), 603–611; arXiv:2402.09914. Restates the Haim–Kislev formula in P(A,b) form and proves NP‑hardness via reduction to a maximum acyclic subgraph/QAP.

[^Krupp20]: Krupp, S. Calculating the EHZ Capacity of Polytopes. PhD thesis, Univ. Köln (2020). Chapter 5 formulates the maximization, QAP view, and convex (CP/SDP) relaxations that yield strong certified bounds and often exact optima on small instances.

[^Rudolf24]: Rudolf, D. The Minkowski billiard characterization of the EHZ‑capacity of convex Lagrangian products. J. Dyn. Diff. Eq. (2024); arXiv:2203.01718. Used for cross‑validation on product‑structured examples.

[^Irie19]: Irie, K. Symplectic homology of fiberwise convex sets and loop spaces. arXiv:1907.09749. Establishes equality of symplectic homology and EHZ capacities for convex bodies, background for variational formulas.

 512b964 (Done. Canonical 2‑face orientation is now enforced everywhere; the knob is gone.)
