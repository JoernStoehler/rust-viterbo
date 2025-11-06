<!-- Author: Codex -->

# Literature Reference — MSc Thesis (Viterbo/Capacities on Convex Bodies)

Purpose: a self‑contained, advisor‑friendly reference list with direct PDF links, clear takeaways, and tight scope on our MSc thesis topic (Viterbo’s conjecture, EHZ capacity on convex bodies, and closely related results). Items are grouped by relevance; most important appear first.

## Core Results and Bridges

- Haim‑Kislev, Ostrover (2024). A counterexample to Viterbo’s conjecture.
  - PDF: https://arxiv.org/pdf/2405.16513
  - Takeaways: First counterexample to Viterbo’s volume–capacity inequality using EHZ on Lagrangian products of rotated regular pentagons (extends to all even dimensions via products). Clarifies that normalized capacities need not coincide on convex domains.

- Viterbo (2000). Metric and isoperimetric problems in symplectic geometry (JAMS).
  - PDF (official; may require access): https://www.ams.org/journals/jams/2000-13-02/S0894-0347-00-00334-2/S0894-0347-00-00334-2.pdf
  - Takeaways: States the volume–capacity isoperimetric inequality and fixes standard normalization (ball/cylinder). Baseline for “normalized capacity” comparisons and conjectured extremality of the ball.

- Abbondandolo, Benedetti, Edtmair (2023). Symplectic capacities near the ball; Banach–Mazur geodesics.
  - PDF: https://arxiv.org/pdf/2312.07363
  - Takeaways: All normalized capacities coincide on sufficiently C^2‑near‑ball domains (but not at mere C^1 proximity). Provides the “near‑ball” regime where Viterbo‑type coincidences hold.

- Artstein‑Avidan, Karasev, Ostrover (2013/14). From symplectic measurements to the Mahler conjecture.
  - PDF: https://arxiv.org/pdf/1303.4197
  - Takeaways: In the centrally symmetric case, Viterbo’s inequality is equivalent to Mahler’s volume product bound. Key bridge to convex geometry and duality arguments.

- Balitskiy (2015). Equality cases in Viterbo‑type inequalities.
  - PDF: https://arxiv.org/pdf/1512.01657
  - Takeaways: Characterizes equality structures (via billiard trajectories) around Viterbo‑type bounds; useful for sanity checks on potential extremizers.

- Irie (2019→2022). Symplectic homology of fiberwise convex sets and loop spaces.
  - PDF: https://arxiv.org/pdf/1907.09749
  - Takeaways: Proves c_SH(K) = c_EHZ(K) for convex bodies; loop‑space formula for c_SH enables comparisons and subadditivity used in arguments about normalized capacities.

## Algorithms, Computation, and Limits (Polytopes, EHZ)

- Chaidez, Hutchings (2020/21). Computing Reeb dynamics on 4D convex polytopes.
  - PDF: https://arxiv.org/pdf/2008.10111
  - Takeaways: Combinatorial algorithm for closed Reeb orbits on polytope boundaries; practical for 4D EHZ‑related computations and validation.

- Rudolf (2022/24). Minkowski billiard characterization of EHZ for convex Lagrangian products.
  - PDF: https://arxiv.org/pdf/2203.01718
  - Takeaways: Shortest (K,T)‑Minkowski billiard trajectories compute c_EHZ(K×T). Natural fit for product‑structured examples (central to the counterexample family).

- Krupp (2020). Calculating the EHZ Capacity of Polytopes (PhD thesis).
  - PDF: https://kups.ub.uni-koeln.de/36196/1/DissertationKrupp.pdf
  - Takeaways: Optimization models (LP/SOCP/SDP) and implementation guidance for practical EHZ computations on polytopes; valuable engineering details.

- Leipold, Vallentin (2024). Computing the EHZ capacity of simplices is NP‑hard.
  - PDF: https://arxiv.org/pdf/2402.09914
  - Takeaways: NP‑hardness for simplices in 2n dimensions; motivates focusing on structured families (e.g., products, symmetry) and approximations for scale.

## Normalization and Related Variants

- Artstein‑Avidan, Ostrover (2007/08). A Brunn–Minkowski inequality for symplectic capacities.
  - PDF: https://arxiv.org/pdf/0712.2631
  - Takeaways: Brunn–Minkowski‑type inequality in the symplectic capacity setting; reinforces convex‑geometric structure relevant to normalization arguments.

- Altabar, … (2022). Cube‑normalized symplectic capacities.
  - PDF: https://arxiv.org/pdf/2208.13666
  - Takeaways: Alternative normalization calibrated to the cube; useful foil for standard ball/cylinder normalizations in comparisons and counterexamples.

## Optional Context (kept brief)

- Hutchings (2022). An elementary alternative to ECH capacities.
  - PDF (arXiv): https://arxiv.org/pdf/2201.03143
  - Takeaways: Combinatorial capacities approximating ECH obstructions; quick 4D context when comparing invariants.

- Cristofaro‑Gardiner, Hutchings, Ramos (2015). The asymptotics of ECH capacities.
  - PDF: https://arxiv.org/pdf/1210.2167
  - Takeaways: ECH capacities recover volume asymptotically on Liouville domains; background for 4D comparisons only when needed.

- Cristofaro‑Gardiner, Savale (2020). Sub‑leading asymptotics of ECH capacities.
  - PDF: https://arxiv.org/pdf/1811.00485
  - Takeaways: Refinements to ECH asymptotics in 4D; consult only if a comparison requires it.

- Schlenk (1999). On symplectic folding.
  - PDF: https://arxiv.org/pdf/math/9903086
  - Takeaways: Embedding techniques; background context, not central to our capacity focus.

- Hind (2014). Some optimal embeddings of symplectic ellipsoids.
  - PDF: https://arxiv.org/pdf/1409.5110
  - Takeaways: Explicit embedding constructions; context for capacity/embedding interplay beyond our core path.


## Appendix — Considered but Dismissed (with reasons)

- Gajewski, Goldin, Safin, Singh, Zhang (2019). Optimization on Symplectic Embeddings (private preprint).
  - Reason: private/unpublished; ML framing without concrete results needed for our capacity focus.

- Liu, Yi, Zhang, Huang (2023). Symplectic Structure‑Aware Hamiltonian (Graph) Embeddings.
  - PDF: https://arxiv.org/pdf/2309.04885
  - Reason: ML/graph methods; does not address symplectic capacities/convex bodies.

- Misc. toric quantum homology without capacity applications.
  - Reason: interesting background, but not directly used for EHZ/Viterbo results in convex bodies.


Notes
- Offline copies live under `data/downloads/` (gitignored). Run `bash scripts/paper-download.sh --match "keyword"` for a single entry or `--all` to sync every PDF link above; the script grabs arXiv sources when possible and records a `manifest.json` plus `.run.json` provenance next to each download.
- If an authoritative PDF is paywalled, link the official PDF page and note access constraints; prefer arXiv/author OA when available.
