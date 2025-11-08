<!--
Ticket: 5ae1e6a6-5011-4693-8860-eeec4828cc0e
Notes: Record design choices for ridge charts and rotation used by the oriented‑edge algorithm.
-->

# Oriented‑Edge: Charts and Rotation

Purpose. Capture the invariants and rationale behind our choice of ridge charts and how we compute and use rotation ρ in the oriented‑edge algorithm, to avoid regressions and unnecessary toggles.

Key decisions
- Fixed per ridge: Each 2‑face (ridge) has a single chart chosen once and used from any incoming facet to keep signs consistent and avoid path‑dependent artifacts.
- Canonical orientation: The ridge basis (u₁,u₂) is orthonormal (Euclidean) and satisfies ω₀(u₁,u₂) > 0. This ties chart orientation to the ambient symplectic form and ensures per‑edge first‑hit maps are orientation‑preserving on admissible domains.
- Rotation from polar factor: For the 2×2 linear part M of the charted per‑edge map, compute the orthogonal polar factor R (M = R S, S ≻ 0) and define ρ := arg(R)/π ∈ [0,1]. This is robust in floating point and invariant to uniform scaling of M; we do not require det M ≈ 1.
- Area‑preserving vs Euclidean charts: First‑hit maps preserve dα‑area, not Euclidean area. Our orthonormal Euclidean charts scale dα by a positive ridge‑dependent constant, so det M need not equal 1; ρ via the polar factor is unaffected. We intentionally do not dα‑normalize charts (see “No toggles” below).
- Accumulation and index: Along closed cycles, ρ accumulates additively and the (generic, 2D) Conley–Zehnder index satisfies μ_CZ = ceil(ρ) + floor(ρ). The index‑3 minimizer has ρ ∈ (1,2), so the search prunes when ρ > 2. This bound is theory‑fixed, not a hyperparameter.

Non‑goals and avoided toggles
- No dual chart mode. We deliberately do not support an opt‑in dα‑unit chart mode (which would enforce det M ≈ 1) because it increases maintenance and test burden without improving rotation computation or pruning.
- No runtime orientation flips. Charts are fixed per ridge; we do not allow user‑configurable orientation switches.

Asserts and guards
- Orientation preservation: debug‑assert det M > 0 on per‑edge maps between canonical charts.
- Rotation extraction: ensure the polar factor R has det R > 0 and clamp arguments for atan2; treat |tr R| ≈ 2 as a near‑identity degeneracy.
- Domain construction: enforce τ > 0 and compare τ_j ≤ τ_k only for forward‑hitting co‑facets (⟨n_k, v_F⟩ > 0), with consistent epsilons.

Cross‑refs
- Background theory: CZ and rotation (Docs: thesis/Ekeland-Hofer-Zehnder-Capacity.md#cz-rotation).
- Algorithm spec: rotation and pruning policy (Docs: thesis/capacity-algorithm-oriented-edge-graph.md).

