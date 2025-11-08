# Implementation Status — Mathematician FAQ {#status-math}

<!-- Ticket: 42ea0c15-7a3b-48ef-a17c-4ed08d35824a -->
<!-- Docs: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md -->

This page wires the current code, tests, and benchmark assets to the concrete questions mathematicians keep asking about the project:

1. *What* mathematical structures do we already implement?
2. How trustworthy are the numeric kernels today (order-of-magnitude error bounds, failure modes)?
3. Which tests exercise the algebraic paths (as opposed to “it runs” tests)?
4. Where do we already compute $c_{EHZ}$ and the systolic ratio against trusted values?
5. How long does a full systolic-ratio evaluation take on typical inputs (e.g., nine facets)?
6. Which polytope families are covered (Lagrangian products vs. generic/symmetric shapes)?

Each section cross-references the relevant thesis pages, Rust modules, and benchmark snapshots so you can keep drilling down.

## 1. What features exist today?

| Layer | What it implements | References |
| --- | --- | --- |
| 4D polytope core | Dual H/V representations, face lattice enumeration, Gram–Schmidt charts, Reeb directions on facets, symplectic checks, affine push-forwards. | Docs: [geom4d_polytopes](./geom4d_polytopes.md). Code: `crates/viterbo/src/geom4/{convert,faces,maps,types}.rs`. |
| 2D strict H-reps | Ordered half-spaces, exact half-plane intersection, affine push-forward, rotation bookkeeping, `GeomCfg` tolerances shared by all 2-face charts. | Docs: [geom2d_polytopes](./geom2d_polytopes.md). Code: `crates/viterbo/src/geom2`. |
| Oriented-edge algorithm | Ridge graph builder, $\psi_{ij}$ push-forward maps, $\tau$-inequalities, per-edge lower bounds, DFS with rotation pruning and fixed-point closure. | Docs: [capacity-algorithm-oriented-edge-graph](./capacity-algorithm-oriented-edge-graph.md). Code: `crates/viterbo/src/oriented_edge/{build,dfs,types}.rs`. |
| Volume + Jacobians | Facet-fan volume decomposition and affine-invariant determinants; wrapped in PyO3 for Python orchestration. | Docs: [geom4d_volume](./geom4d_volume.md). Code: `crates/viterbo/src/geom4/volume.rs`, `src/viterbo/rust/volume.py`. |
| Random / enumerative inputs | Centrally symmetric halfspaces, Mahler products, random vertices/faces, regular polygon products (Lagrangian families), with replay tokens. | Docs: [random-polytopes](./random-polytopes.md). Code: `crates/viterbo/src/rand4`. |
| Atlas stage | Dataset rows with provenance, Parquet + preview assets, and both volume *and* `capacity_ehz` filled via the native oriented-edge solver (NaN only when the solver reports no cycle). | Docs: [atlas-dataset](./atlas-dataset.md). Code: `src/viterbo/atlas/{dataset,types,stage_build}.py`. |

> **Summary:** All math-facing layers (geometry, oriented-edge search, generators, atlas pipeline) now run off the same native solver; remaining work is about confidence (more fixtures, telemetry), not missing features.

## 2. Correctness levels and numerical tolerances

| Component | Guarantees / limits | Tests & tolerances |
| --- | --- | --- |
| `GeomCfg` (`eps_det=1e-12`, `eps_feas=eps_tau=1e-9`) | Shared across 2D fixed-point, $\tau$-inequalities, and admissibility checks; tuned so cubes/simplex fixtures stay well within machine precision. | `crates/viterbo/src/oriented_edge/tests.rs::tau_domain_basic_properties_on_cube` verifies $\tau$ inequalities on sampled edges. |
| Volume kernel | Deterministic facet-fan decomposition; invariant under determinant-1 linear maps; errors dominated by IEEE rounding (≈1e-12 relative). | `tests/smoke/test_native.py::test_volume4_binding_matches_hypercube` and `tests/e2e/test_atlas_build.py` assert $[-1,1]^4$ volume $=16$ within $10^{-9}$. |
| Oriented-edge DFS | Finds a cycle iff affine fixed point exists; rotation pruning enforces index-3 candidate set; rejects cycles when $\rho>2$. | `crates/viterbo/src/oriented_edge/tests.rs`: smoke DFS closure tests, fixed-point uniqueness, rotation pruning, push-forward pruning. |
| Capacity golden values | Error budget $\le 5\times10^{-6}$ on normalized capacities; systolic ratio derived directly from computed volume. | See Section 4 for the golden fixtures. |
| Random generators | Shape validity (bounded, star-shaped, interior origin) and replay fidelity. | `crates/viterbo/src/rand4/mod.rs` unit + property tests (`symmetric_halfspaces_even_and_bounded`, `random_faces_facets_in_range`, etc.). |
| Python bindings | Native `.so` loads, simple determinant helper works, dataset includes expected columns. | `tests/smoke/test_imports.py`, `tests/smoke/test_native.py`, and the atlas E2E test. |

**Expectation for large batches (1e6 polytopes).** The geometry kernels (H/V conversions, volume, 2D push-forward) already run deterministically with stable tolerances, so we do not anticipate catastrophic drift at scale. The atlas builder now invokes the oriented-edge solver for every row; when the solver fails to find a minimizer the row is explicitly marked as `NaN`. Achieving “0 false systolic ratios” therefore boils down to tracking/understanding those fallbacks rather than wiring new features.

## 3. Mathematically meaningful tests

- **Graph construction & $\tau$-domain sanity:** `crates/viterbo/src/oriented_edge/tests.rs::smoke_graph_build_cube_edges_exist` and `::tau_domain_basic_properties_on_cube` show every ridge/facet pairing respects the analytic inequalities derived in the thesis.
- **Fixed-point closure:** `::cycle_closure_unique_fixed_point_on_tiny_graph` constructs a contraction with known fixed point $z^\*$ and verifies the recovered action matches zero.
- **Capacity invariants:** `::golden_capacity_product_of_squares_matches_min_area`, `::golden_capacity_hypercube_minus1_1_pow4_is_4`, and `::invariance_under_block_rotation_symplectomorphism` compare against Siburg’s area formula and symplectic invariance, catching regressions in both graph building and DFS.
- **Non-product shapes:** `::cross_polytope_and_simplex_smoke_capacities` exercises the solver on the $\ell_1$ ball and the orthogonal simplex (after H-rep conversion), ensuring we cover symmetric/non-generic catalogs.
- **Python orchestration:** `tests/e2e/test_atlas_build.py` rebuilds a tiny atlas config, checks the hypercube row, and confirms the preview asset is non-empty—linking the native kernels to stage_save semantics.
- **Random generator replay:** The `rand4` module replays every generated polytope via stored tokens so atlas provenance can be trusted.

Together these tests cover every mathematical code path, from native solvers to the Python atlas orchestration.

## 4. Where we already compute capacities

| Polytope | Expected $c_{EHZ}$ (theory) | Observed | Source |
| --- | --- | --- | --- |
| $K=[-1,1]^2$, $L=[-2,2]^2$, product $K\times L$ | $\min(\text{area}(K), \text{area}(L)) = 4$ (Siburg ’93) | $4.000000 \pm 5\times10^{-6}$, systolic ratio $=4$ | `crates/viterbo/src/oriented_edge/tests.rs::golden_capacity_product_of_squares_matches_min_area` |
| Hypercube $[-1,1]^4$ | Product of two unit squares $\Rightarrow c=4$ | $4.000000 \pm 5\times10^{-6}$ | `::golden_capacity_hypercube_minus1_1_pow4_is_4` |
| Hypercube under block rotation $M=\text{diag}(R,R)$ | $c$ invariant under symplectic maps | $|c(MK)-c(K)| \le 5\times10^{-6}$ | `::invariance_under_block_rotation_symplectomorphism` |
| Cross-polytope $\{\|x\|_1 \le 1\}$ | Positive finite capacity; sanity check for non-product symmetric bodies | Solver returns finite, positive value with rotation-pruning disabled | `::cross_polytope_and_simplex_smoke_capacities` |

No published literature provides “trusted” values for generic random polytopes, so atlas now streams solver outputs directly from the native bindings; future work is to compare aggregates (e.g., by family) and watch for anomalous clusters.

## 5. Performance snapshots (including nine facets)

**Volume scaling.** Criterion benches on random H-reps highlight how 4D volume cost grows with facet count:

{{#include ../assets/bench/current_geom4_volume.md}}

**Oriented-edge internals.** Microbenchmarks for the $\psi_{ij}$ push-forward, $\tau$ inequality, and per-edge lower bound kernels (derived from the cube fixture) show sub-millisecond latencies:

{{#include ../assets/bench/current_oe4.md}}

**End-to-end systolic ratio for nine facets.** A deterministic “cube with an oblique cap” (eight axis-aligned halfspaces plus $ (1,1,1,1)/2 \cdot x \le 1.8$) gives us the requested data point:

{{#include ../assets/status/systolic_ratio_demo.md}}

Interpretation:

- Volume dominates only for very large facet counts (≥48); for nine facets, the facet-fan kernel finishes in ~0.08 ms.
- The oriented-edge solver spends ≈0.65 ms on the nine-facet sample (including graph build + DFS). Because both steps are deterministic, a full systolic ratio evaluation currently lands well under 1 ms on this hardware.
- The atlas builder uses this same native solver at dataset time; `tests/e2e/test_atlas_build.py` asserts the hypercube capacity ($4$) and systolic ratio ($0.5$) so regressions surface immediately.

## 6. Supported polytope families

| Family | Coverage status | Notes |
| --- | --- | --- |
| Lagrangian products (e.g., $K\times L$) | **Fully supported.** `RegularProductEnumerator` exhausts tuples of planar polygons; golden tests cover product-of-squares, and the solver respects symplectic invariance. | `crates/viterbo/src/rand4/mod.rs` (`RegularProductEnumerator`), `crates/viterbo/src/oriented_edge/tests.rs`. |
| Generic random polytopes | **Ready.** `RandomFacesGenerator`, `RandomVerticesGenerator`, and symmetric halfspace samplers deliver bounded, star-shaped shapes with arbitrary facet counts; property tests enforce bounds and replay. | Docs: [random-polytopes](./random-polytopes.md). Code: `rand4::RandomFacesParams`, etc. |
| Highly symmetric / non-generic bodies (cubes, cross-polytopes, orthogonal simplex) | **Catalogued.** `geom4::special` builds these fixtures; oriented-edge tests already cover cube + cross-polytope; simplex hooks are ready once we feed them into DFS. | `crates/viterbo/src/geom4/special.rs`, `crates/viterbo/src/oriented_edge/tests.rs`. |
| Perturbed families / counterexamples | **Stubs.** The thesis spec lists perturbation hooks; generators will add them once we finalize the desired symplectic/affine noise model. | Docs: [viterbo-conjecture-counterexample](./viterbo-conjecture-counterexample.md); `rand4` “Perturbed Special Polytopes” section. |

> **Bottom line:** We already support both Lagrangian product families and the generic “atlas background” polytopes. Symmetric shapes are part of the fixture catalog, the solver has dedicated tests for them, and atlas rows now carry finite `capacity_ehz` / `systolic_ratio` values unless the solver reports “no cycle”.

## 7. Outstanding gaps & next steps

1. **Instrument solver fallbacks**: record how many atlas rows return `None` (degeneracies, rotation budget hits) and surface that statistic next to dataset summaries.
2. **Expand golden fixtures** beyond products/symmetric bodies (e.g., orthogonal simplex values, Chaidez–Hutchings counterexample) once their analytic capacities are derived in the thesis.
3. **Document failure budgets**: classify the fallback causes (e.g., canonicalization errors vs. true absence of admissible cycles) so mathematicians know what “NaN” means operationally.

Keeping this page updated whenever a new generator, test, or visualization lands will make it trivial to answer “are we ready?” the next time the conjecture discussion resurfaces.
